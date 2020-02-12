use snowflake::ProcessUniqueId;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter;
use std::mem;
use std::rc::{Rc, Weak};
use typemap::{Key, TypeMap};

use crate::transaction::Transaction;

#[derive(PartialEq, Clone)]
enum AtomState {
    UpToDate,
    Expired(Expired),
}

#[derive(PartialEq, Clone)]
pub enum Expired {
    Maybe,
    ForSure,
}

pub struct EvaluateKey;

impl Key for EvaluateKey {
    type Value = Box<dyn Evaluateable>;
}

pub struct BecomeObservedKey;

impl Key for BecomeObservedKey {
    type Value = Box<dyn Fn()>;
}

pub struct BecomeUnobservedKey;

impl Key for BecomeUnobservedKey {
    type Value = Box<dyn Fn()>;
}

pub trait Evaluateable {
    fn evaluate(&mut self, ctx: &mut EvalContext) -> bool;
}

pub struct EvalContext {
    changed: bool,
    prev_used: HashSet<Atom>,
    used_atoms: HashSet<Atom>,
}

impl EvalContext {
    pub fn new(prev_used: HashSet<Atom>) -> Self {
        EvalContext {
            prev_used,
            changed: false,
            used_atoms: HashSet::new(),
        }
    }

    pub fn report(&mut self, atom: Atom) {
        self.used_atoms.insert(atom);
    }

    pub fn changed(&mut self) {
        self.changed = true
    }

    pub fn diff_added(&self) -> Vec<Atom> {
        self.used_atoms
            .difference(&self.prev_used)
            .cloned()
            .collect()
    }

    pub fn diff_removed(&self) -> Vec<Atom> {
        self.prev_used
            .difference(&self.used_atoms)
            .cloned()
            .collect()
    }

    pub fn into_used(self) -> HashSet<Atom> {
        self.used_atoms
    }
}

#[derive(Debug)]
pub struct Atom {
    id: ProcessUniqueId,
    body: Rc<RefCell<AtomBody>>,
}

impl Drop for Atom {
    fn drop(&mut self) {
        // Will drop atom
        if Rc::strong_count(&self.body) == 1 {
            let body = self.body.borrow();
            for atom in body.based_on.iter() {
                atom.report_not_more_used_by(self)
            }
        }
    }
}

impl PartialEq for Atom {
    fn eq(&self, other: &Atom) -> bool {
        self.id == other.id
    }
}

impl Hash for Atom {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for Atom {}

impl Clone for Atom {
    fn clone(&self) -> Self {
        Atom {
            id: self.id.clone(),
            body: self.body.clone(),
        }
    }
}

impl Atom {
    pub fn new(name: String) -> Self {
        let body = AtomBody::new(name);
        Atom {
            id: body.id.clone(),
            body: Rc::new(RefCell::new(body)),
        }
    }

    fn weak(&self) -> WeakAtom {
        return WeakAtom {
            id: self.id.clone(),
            body: Rc::downgrade(&self.body),
        };
    }

    pub fn on_become_observed<F: Fn() + 'static>(&mut self, cb: F) {
        self.ext_mut().insert::<BecomeObservedKey>(Box::new(cb));
    }

    pub fn on_become_unobserved<F: Fn() + 'static>(&mut self, cb: F) {
        self.ext_mut().insert::<BecomeUnobservedKey>(Box::new(cb));
    }

    pub fn report_used_in(&self, ctx: &mut EvalContext) {
        ctx.report(self.clone())
    }

    pub fn report_used_by(&self, atom: &Atom) {
        self.body.borrow_mut().report_used_by(atom.weak());
    }

    fn report_not_more_used_by(&self, atom: &Atom) {
        self.body.borrow_mut().used_by.remove(&atom.weak());
    }

    pub fn report_changed(&mut self, tx: Option<&mut Transaction>) {
        self.expire(Expired::ForSure);
        if tx.is_some() {
            tx.unwrap().mark_changed(self.clone());
        } else {
            Transaction::fire_reactions(iter::once(&*self))
        }
    }

    pub fn is_observed(&self) -> bool {
        self.body.borrow().observation_counter > 0
    }

    pub fn ext(&self) -> Ref<TypeMap> {
        let body = self.body.borrow();
        Ref::map(body, |b| &b.ext)
    }

    pub fn ext_mut(&mut self) -> RefMut<TypeMap> {
        let body = self.body.borrow_mut();
        RefMut::map(body, |b| &mut b.ext)
    }

    pub fn walk<P: FnMut(&Atom) -> bool>(&self, predicate: &mut P) {
        if predicate(&self) {
            let body = self.body.borrow();
            for atom in &mut body.used_by.iter() {
                let atom = atom.upgrade();
                if atom.is_some() {
                    atom.unwrap().walk(predicate)
                }
            }
        }
    }

    /// Call to mark current Atom as "expired". Also signals that
    /// all dependants CAN BE expired.
    pub fn expire(&mut self, expire: Expired) {
        let mut body = self.body.borrow_mut();
        body.expire(expire)
    }

    pub fn based_on(&self) -> Ref<HashSet<Atom>> {
        Ref::map(self.body.borrow(), |b| &b.based_on)
    }

    pub fn should_evaluate(&self) -> bool {
        let body = self.body.borrow();
        match body.state {
            AtomState::UpToDate => false,
            AtomState::Expired(Expired::ForSure) => true,
            AtomState::Expired(Expired::Maybe) => {
                let mut should_evaluate = false;
                for atom in body.based_on.iter() {
                    if atom.clone().update() {
                        should_evaluate = true
                    }
                }

                should_evaluate
            }
        }
    }

    pub fn update(&self) -> bool {
        if self.should_evaluate() {
            self.evaluate()
        } else {
            false
        }
    }

    pub fn id(&self) -> Ref<ProcessUniqueId> {
        Ref::map(self.body.borrow(), |b| &b.id)
    }

    pub fn evaluate(&self) -> bool {
        let ctx = {
            let mut body = self.body.borrow_mut();
            body.evaluate()
        };

        for atom in &mut ctx.diff_added() {
            atom.report_used_by(self)
        }

        for atom in &mut ctx.diff_removed() {
            atom.report_not_more_used_by(self)
        }

        ctx.changed
    }

    /// Called by Reactions
    pub fn become_observed(&mut self) {
        let mut body = self.body.borrow_mut();
        body.observation_counter += 1;

        //become observable
        if body.observation_counter == 1 {
            {
                let on_become_observable = body.ext.get_mut::<BecomeObservedKey>();
                if on_become_observable.is_some() {
                    on_become_observable.unwrap()();
                };
            }

            for atom in &mut body.based_on.iter() {
                atom.clone().become_observed()
            }
        }
    }

    /// Called by Reactions
    pub fn become_unobserved(&mut self) {
        let mut body = self.body.borrow_mut();
        body.observation_counter -= 1;

        assert!(
            body.observation_counter >= 0,
            "Observation counter should be > 0"
        );

        //become unobservable
        if body.observation_counter == 0 {
            {
                let on_become_unobservable = body.ext.get_mut::<BecomeUnobservedKey>();
                if on_become_unobservable.is_some() {
                    on_become_unobservable.unwrap()();
                };
            }

            for atom in &mut body.based_on.iter() {
                atom.clone().become_unobserved()
            }
        }
    }
}

struct AtomBody {
    id: ProcessUniqueId,
    name: String,
    used_by: HashSet<WeakAtom>,
    based_on: HashSet<Atom>,
    ext: TypeMap,
    state: AtomState,
    observation_counter: i8,
}

impl AtomBody {
    fn new(name: String) -> Self {
        AtomBody {
            id: ProcessUniqueId::new(),
            ext: TypeMap::new(),
            name,
            used_by: HashSet::new(),
            based_on: HashSet::new(),
            state: AtomState::Expired(Expired::ForSure),
            observation_counter: 0,
        }
    }

    fn report_used_by(&mut self, atom: WeakAtom) {
        self.used_by.insert(atom);
    }

    fn evaluate(&mut self) -> EvalContext {
        let evaluate = self.ext.get_mut::<EvaluateKey>();
        let context = match evaluate {
            Some(ev) => {
                let prev_used = mem::replace(&mut self.based_on, HashSet::new());
                let mut ctx = EvalContext::new(prev_used);

                let changed = ev.evaluate(&mut ctx);
                if changed {
                    ctx.changed()
                }

                self.based_on = ctx.used_atoms.clone();
                ctx
            }
            None => EvalContext::new(HashSet::new()),
        };

        self.state = AtomState::UpToDate;

        context
    }

    fn expire(&mut self, expire: Expired) {
        if self.state == AtomState::Expired(expire.clone())
            || self.state == AtomState::Expired(Expired::ForSure)
        {
            return;
        }

        self.state = AtomState::Expired(expire);
        for derived in &mut self.used_by.iter() {
            let derived = derived.upgrade();
            if derived.is_some() {
                derived.unwrap().expire(Expired::Maybe)
            }
        }
    }
}

impl fmt::Debug for AtomBody {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Atom[name: {}, id: {}]", self.name, self.id)
    }
}

impl PartialEq for AtomBody {
    fn eq(&self, other: &AtomBody) -> bool {
        self.id == other.id
    }
}

impl Hash for AtomBody {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for AtomBody {}

#[derive(Debug)]
pub struct WeakAtom {
    id: ProcessUniqueId,
    body: Weak<RefCell<AtomBody>>,
}

impl WeakAtom {
    pub fn upgrade(&self) -> Option<Atom> {
        let body = self.body.upgrade();
        body.map(|body| Atom {
            id: self.id.clone(),
            body: body,
        })
    }
}

impl PartialEq for WeakAtom {
    fn eq(&self, other: &WeakAtom) -> bool {
        self.id == other.id
    }
}

impl Hash for WeakAtom {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for WeakAtom {}

impl Clone for WeakAtom {
    fn clone(&self) -> Self {
        WeakAtom {
            id: self.id.clone(),
            body: self.body.clone(),
        }
    }
}
