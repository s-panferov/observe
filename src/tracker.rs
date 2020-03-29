use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::mem;
use std::rc::{Rc, Weak};

use snowflake::ProcessUniqueId;

use std::any::Any;
use std::cell::RefCell;

use crate::context::EvalContext;
use crate::eval::{AnyValue, Evaluation};
use crate::transaction::Transaction;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Freshness {
    UpToDate,
    Expired(Expired),
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Expired {
    Maybe,
    ForSure,
}

#[derive(Debug, Clone)]
pub struct Tracker {
    id: ProcessUniqueId,
    body: Rc<RefCell<RawTracker>>,
}

impl PartialEq for Tracker {
    fn eq(&self, other: &Tracker) -> bool {
        self.id == other.id
    }
}

impl Hash for Tracker {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for Tracker {}

impl Tracker {
    pub fn new(name: String) -> Self {
        let body = RawTracker::new(name);
        let tracker = Tracker {
            id: body.id.clone(),
            body: Rc::new(RefCell::new(body)),
        };

        tracker.inner_mut().reference = Some(tracker.weak());
        tracker
    }

    pub fn weak(&self) -> WeakTracker {
        return WeakTracker {
            id: self.id.clone(),
            body: Rc::downgrade(&self.body),
        };
    }

    pub fn before_access(&self, ctx: Option<&mut EvalContext>) {
        if ctx.is_some() {
            ctx.unwrap().access(self.clone());
        }
        self.update()
    }

    pub fn get(&self, ctx: Option<&mut EvalContext>) -> Rc<dyn Any + 'static> {
        self.before_access(ctx);
        self.inner().body.get()
    }

    pub fn change<F>(&self, tx: Option<&mut Transaction>, closure: F)
    where
        F: FnOnce(&mut dyn Evaluation) -> u64,
    {
        let new_hash = closure(&mut *self.inner_mut().body);
        self.inner_mut().update_hash(new_hash);
        if let Some(tx) = tx {
            tx.mark_changed(self.weak().clone());
        } else {
            self.notify_reactions()
        }
    }

    pub fn set(&self, tx: Option<&mut Transaction>, value: AnyValue) {
        self.change(tx, move |eval| eval.set(value))
    }

    pub fn update(&self) {
        if self.inner().should_evaluate() {
            self.inner_mut().evaluate()
        }
    }

    pub fn notify_reactions(&self) {
        if self.inner().body.is_observer() {
            if self.inner().body.is_scheduled() {
                self.inner_mut().body.on_reaction();
            } else {
                self.update()
            }
        }

        if self.inner().is_observed() {
            // We should have a reaction somewhere up
            let used_by = self.inner().used_by.clone();
            for tracker in used_by.into_iter() {
                let tracker = tracker.upgrade();
                if tracker.is_some() {
                    tracker.unwrap().notify_reactions()
                }
            }
        }
    }

    pub fn state(&self) -> Freshness {
        let inner = self.inner();
        inner.state.clone()
    }

    pub fn set_computation(&self, cb: Box<dyn Evaluation>) {
        self.inner_mut().body = cb;
    }

    fn inner(&self) -> std::cell::Ref<RawTracker> {
        self.body.borrow()
    }

    fn inner_mut(&self) -> std::cell::RefMut<RawTracker> {
        self.body.borrow_mut()
    }
}

pub struct RawTracker {
    id: ProcessUniqueId,
    name: String,
    hash: u64,
    used_by: HashSet<WeakTracker>,
    based_on: HashMap<Tracker, u64>,
    state: Freshness,
    observation_counter: i8,
    reference: Option<WeakTracker>,
    body: Box<dyn Evaluation>,
}

impl Drop for RawTracker {
    fn drop(&mut self) {
        for (tracker, _) in self.based_on.iter() {
            let mut tracker_body = tracker.inner_mut();
            tracker_body.report_not_more_used_by(self.weak_ref());
            if self.body.is_observer() {
                tracker_body.dec_observers()
            }
        }
    }
}

struct EmptyBody {}

impl Evaluation for EmptyBody {
    fn evaluate(&mut self, _: &mut EvalContext) -> u64 {
        0
    }
}

impl RawTracker {
    fn new(name: String) -> Self {
        RawTracker {
            id: ProcessUniqueId::new(),
            name,
            hash: 0,
            used_by: HashSet::new(),
            based_on: HashMap::new(),
            state: Freshness::Expired(Expired::ForSure),
            reference: None,
            observation_counter: 0,
            body: Box::new(EmptyBody {}),
        }
    }

    pub fn weak_ref(&self) -> &WeakTracker {
        self.reference.as_ref().unwrap()
    }

    fn report_used_by(&mut self, tracker: WeakTracker) {
        self.used_by.insert(tracker);
    }

    fn report_not_more_used_by(&mut self, tracker: &WeakTracker) {
        self.used_by.remove(tracker);
    }

    pub fn is_observed(&self) -> bool {
        self.observation_counter > 0
    }

    pub fn should_evaluate(&self) -> bool {
        match self.state {
            Freshness::UpToDate => false,
            Freshness::Expired(Expired::ForSure) => true,
            Freshness::Expired(Expired::Maybe) => {
                let mut changed = false;
                for (tracker, hash) in self.based_on.iter() {
                    if tracker.inner().should_evaluate() {
                        tracker.inner_mut().evaluate();
                    }
                    if &tracker.inner().hash != hash {
                        changed = true;
                        break;
                    }
                }
                changed
            }
        }
    }

    pub fn update_hash(&mut self, new_hash: u64) {
        if self.hash != new_hash {
            self.hash = new_hash;
            self.expire(Expired::ForSure);
        }
    }

    pub fn evaluate(&mut self) {
        let body = self.body.as_mut();
        let prev_used = mem::replace(&mut self.based_on, HashMap::new());
        let mut ctx = EvalContext::new(prev_used);
        let hash = body.evaluate(&mut ctx);
        let using = HashMap::from_iter(ctx.using.iter().map(|t| {
            let version = t.inner().hash;
            (t.clone(), version)
        }));

        self.hash = hash;
        self.based_on = using;
        self.state = Freshness::UpToDate;

        for based_on in ctx.diff_added() {
            let mut based_on = based_on.inner_mut();
            based_on.report_used_by(self.weak_ref().clone());
            if self.body.is_observer() {
                based_on.inc_observers()
            }
        }

        for based_on in ctx.diff_removed() {
            let mut based_on = based_on.inner_mut();
            based_on.report_not_more_used_by(self.weak_ref());
            if self.body.is_observer() {
                based_on.dec_observers()
            }
        }
    }

    /// Called by Reactions
    pub fn inc_observers(&mut self) {
        self.observation_counter += 1;

        //become observable
        if self.observation_counter == 1 {
            self.body.on_become_observed();
            for (tracker, _) in &mut self.based_on.iter() {
                tracker.inner_mut().inc_observers()
            }
        }
    }

    /// Called by Reactions
    pub fn dec_observers(&mut self) {
        self.observation_counter -= 1;

        assert!(
            self.observation_counter >= 0,
            "Observation counter should be > 0"
        );

        //become unobservable
        if self.observation_counter == 0 {
            self.body.on_become_unobserved();
            for (tracker, _) in &mut self.based_on.iter() {
                tracker.inner_mut().dec_observers()
            }
        }
    }

    fn expire(&mut self, expire: Expired) {
        if self.state == Freshness::Expired(expire.clone())
            || self.state == Freshness::Expired(Expired::ForSure)
        {
            return;
        }

        self.state = Freshness::Expired(expire);
        for derived in &mut self.used_by.iter() {
            let derived = derived.upgrade();
            if derived.is_some() {
                derived.unwrap().inner_mut().expire(Expired::Maybe)
            }
        }
    }
}

impl fmt::Debug for RawTracker {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Tracker[name: {}, id: {}]", self.name, self.id)
    }
}

impl PartialEq for RawTracker {
    fn eq(&self, other: &RawTracker) -> bool {
        self.id == other.id
    }
}

impl Hash for RawTracker {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for RawTracker {}

#[derive(Debug)]
pub struct WeakTracker {
    id: ProcessUniqueId,
    body: Weak<RefCell<RawTracker>>,
}

impl WeakTracker {
    pub fn upgrade(&self) -> Option<Tracker> {
        let body = self.body.upgrade();
        body.map(|body| Tracker {
            id: self.id.clone(),
            body,
        })
    }
}

impl PartialEq for WeakTracker {
    fn eq(&self, other: &WeakTracker) -> bool {
        self.id == other.id
    }
}

impl Hash for WeakTracker {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for WeakTracker {}

impl Clone for WeakTracker {
    fn clone(&self) -> Self {
        WeakTracker {
            id: self.id.clone(),
            body: self.body.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Expired, Freshness, Tracker};
    use crate::context::EvalContext;
    use crate::test::{SharedMock, Spy};

    #[test]
    fn call_computation_on_update() {
        let tracker = Tracker::new("Test".to_owned());
        let spy = SharedMock::new();
        tracker.set_computation(Box::new({
            let mock = spy.clone();
            move |_: &mut _| {
                mock.get().trigger();
                0
            }
        }));

        spy.get().expect_trigger().return_const(()).times(1);
        tracker.update();

        spy.get().checkpoint();

        spy.get().expect_trigger().return_const(()).times(0);
        tracker.update();
    }

    #[test]
    fn properly_expire() {
        let a = Tracker::new("A".to_owned());
        let b = Tracker::new("B".to_owned());
        let c = Tracker::new("C".to_owned());

        a.set_computation({
            let b = b.clone();
            Box::new(move |ctx: &mut EvalContext| {
                b.update();
                ctx.access(b.clone());
                0
            })
        });

        b.set_computation({
            let c = c.clone();
            Box::new(move |ctx: &mut EvalContext| {
                c.update();
                ctx.access(c.clone());
                0
            })
        });

        a.update();

        assert_eq!(a.inner().based_on.len(), 1);
        assert_eq!(b.inner().based_on.len(), 1);
        assert_eq!(c.inner().based_on.len(), 0);

        assert_eq!(c.inner().used_by.len(), 1);
        assert_eq!(b.inner().used_by.len(), 1);
        assert_eq!(a.inner().used_by.len(), 0);

        assert_eq!(c.inner().state, Freshness::UpToDate);
        assert_eq!(c.inner().state, Freshness::UpToDate);
        assert_eq!(c.inner().state, Freshness::UpToDate);

        c.inner_mut().expire(Expired::ForSure);

        assert_eq!(a.inner().state, Freshness::Expired(Expired::Maybe));
        assert_eq!(b.inner().state, Freshness::Expired(Expired::Maybe));
        assert_eq!(c.inner().state, Freshness::Expired(Expired::ForSure));

        a.update();

        assert_eq!(c.inner().state, Freshness::UpToDate);
        assert_eq!(c.inner().state, Freshness::UpToDate);
        assert_eq!(c.inner().state, Freshness::UpToDate);
    }
}
