use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::sync::RwLock;
use std::sync::{Arc, Weak};

use snowflake::ProcessUniqueId;

use crate::context::EvalContext;
use crate::transaction::Transaction;

pub enum Invalidate {
    SelfAndDeps,
    OnlyDeps,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum TrackerState {
    Good,
    Expired(Expired),
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Expired {
    Maybe,
    ForSure,
}

pub trait Evaluation {
    fn on_reaction(&self) {}
    fn on_become_observed(&self) {}
    fn on_become_unobserved(&self) {}
    fn is_scheduled(&self) -> bool {
        false
    }
    fn eval(&self, ctx: &EvalContext) -> u64;
}

#[derive(Clone)]
pub struct Tracker {
    id: ProcessUniqueId,
    body: Arc<RwLock<TrackerBody>>,
}

impl Tracker {
    pub fn new() -> Self {
        Tracker {
            id: ProcessUniqueId::new(),
            body: Arc::new(RwLock::new(TrackerBody::new())),
        }
    }

    pub fn set_name(&self, name: String) {
        self.body.write().unwrap().name = Some(name);
    }

    pub fn autorun(&self) {
        self.body.write().unwrap().is_observer = true;
    }

    pub fn access(&self, ctx: Option<&EvalContext>) {
        if ctx.is_some() {
            ctx.unwrap().access(self.clone());
        }
        self.update()
    }

    pub fn expire(&self, expire: Expired, inv: Invalidate) {
        self.body.write().unwrap().expire(expire, inv)
    }

    pub fn hash(&self) -> u64 {
        self.body.read().unwrap().hash
    }

    pub fn change(&self, new_hash: u64, inv: Invalidate, tx: Option<&mut Transaction>) -> bool {
        let changed = self.body.write().unwrap().update_hash(new_hash, inv);

        if changed {
            if let Some(tx) = tx {
                tx.mark_changed(Tracker::downgrade(self));
            } else {
                self.notify_reactions()
            }
        }

        changed
    }

    pub fn set_eval(tracker: &Tracker, eval: Arc<dyn Evaluation>) {
        tracker.body.write().unwrap().set_eval(eval);
    }

    pub fn update(&self) {
        if self.body.read().unwrap().should_evaluate() {
            self.body
                .write()
                .unwrap()
                .evaluate(Tracker::downgrade(self))
        }
    }

    pub(crate) fn notify_reactions(&self) {
        let body = self.body.read().unwrap();
        let is_observer = body.is_observer;
        let is_scheduled = body.eval().is_scheduled();
        let is_observed = body.is_observed();
        std::mem::drop(body);

        if is_observer {
            if is_scheduled {
                self.body.read().unwrap().eval().on_reaction()
            } else {
                self.update()
            }
        }

        if is_observed {
            // We should have a reaction somewhere up
            let used_by = self.body.read().unwrap().used_by.clone();
            for tracker in used_by.into_iter() {
                if let Some(tracker) = tracker.upgrade() {
                    tracker.notify_reactions()
                }
            }
        }
    }

    pub fn downgrade(tracker: &Tracker) -> WeakTracker {
        WeakTracker {
            id: tracker.id.clone(),
            body: Arc::downgrade(&tracker.body),
        }
    }

    pub(crate) fn inc_observers(&self) {
        self.body.write().unwrap().inc_observers()
    }

    pub(crate) fn dec_observers(&self) {
        self.body.write().unwrap().dec_observers()
    }
}

pub struct TrackerBody {
    name: Option<String>,
    state: TrackerState,
    hash: u64,
    counter: u8,
    is_observer: bool,
    based_on: HashMap<Tracker, u64>,
    used_by: HashSet<WeakTracker>,
    eval: Option<Arc<dyn Evaluation>>,
}

impl TrackerBody {
    pub fn new() -> Self {
        TrackerBody {
            name: None,
            state: TrackerState::Expired(Expired::ForSure),
            hash: 0,
            counter: 0,
            is_observer: false,
            based_on: HashMap::new(),
            used_by: HashSet::new(),
            eval: None,
        }
    }

    pub fn set_eval(&mut self, eval: Arc<dyn Evaluation>) {
        self.eval = Some(eval)
    }

    fn eval(&self) -> &dyn Evaluation {
        self.eval
            .as_ref()
            .expect("Tracker body should be initialized")
            .as_ref()
    }

    pub fn hash(&self) -> u64 {
        self.hash
    }

    pub fn should_evaluate(&self) -> bool {
        match self.state {
            TrackerState::Good => false,
            TrackerState::Expired(Expired::ForSure) => true,
            TrackerState::Expired(Expired::Maybe) => {
                let mut changed = false;
                for (tracker, hash) in self.based_on.iter() {
                    tracker.update();
                    if &tracker.hash() != hash {
                        changed = true;
                        break;
                    }
                }
                changed
            }
        }
    }

    pub fn evaluate(&mut self, self_ref: WeakTracker) {
        let prev_used = std::mem::replace(&mut self.based_on, HashMap::new());

        let mut ctx = EvalContext::new();

        let prev_used = HashSet::from_iter(prev_used.keys().cloned());
        let hash = self.eval().eval(&mut ctx);
        let using = ctx.into_used();

        for based_on in using.difference(&prev_used) {
            let mut inner = based_on.body.write().unwrap();
            inner.report_used_by(self_ref.clone());
            if self.is_observer {
                inner.inc_observers()
            }
        }

        for based_on in prev_used.difference(&using) {
            let mut inner = based_on.body.write().unwrap();
            inner.report_not_more_used_by(&self_ref);
            if self.is_observer {
                inner.dec_observers()
            }
        }

        let using = HashMap::from_iter(using.into_iter().map(|t| {
            let version = t.hash();
            (t, version)
        }));

        self.hash = hash;
        self.based_on = using;
        self.state = TrackerState::Good;
    }

    /// Called by Reactions
    pub(crate) fn inc_observers(&mut self) {
        self.counter += 1;

        //become observable
        if self.counter == 1 {
            self.eval().on_become_observed();
            for (tracker, _) in &mut self.based_on.iter() {
                tracker.inc_observers()
            }
        }
    }

    /// Called by Reactions
    pub fn dec_observers(&mut self) {
        self.counter -= 1;

        //become unobservable
        if self.counter == 0 {
            self.eval().on_become_unobserved();
            for (tracker, _) in &mut self.based_on.iter() {
                tracker.dec_observers()
            }
        }
    }

    pub fn update_hash(&mut self, new_hash: u64, inv: Invalidate) -> bool {
        let changed = self.hash != new_hash;
        if changed {
            self.hash = new_hash;
            self.expire(Expired::ForSure, inv);
        }

        changed
    }

    fn expire(&mut self, expire: Expired, inv: Invalidate) {
        match &self.state {
            TrackerState::Expired(state) if state == &expire => return,
            TrackerState::Expired(Expired::ForSure) => return,
            _ => {}
        }

        if matches!(inv, Invalidate::SelfAndDeps) {
            self.state = TrackerState::Expired(expire);
        }

        for derived in &mut self.used_by.iter() {
            if let Some(derived) = derived.upgrade() {
                derived.expire(Expired::Maybe, Invalidate::SelfAndDeps)
            }
        }
    }

    pub fn is_observed(&self) -> bool {
        self.counter > 0
    }

    fn report_used_by(&mut self, tracker: WeakTracker) {
        self.used_by.insert(tracker);
    }

    fn report_not_more_used_by(&mut self, tracker: &WeakTracker) {
        self.used_by.remove(tracker);
    }
}

impl std::fmt::Debug for Tracker {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Tracker[id: {}]", self.id)
    }
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

pub struct WeakTracker {
    pub(crate) id: ProcessUniqueId,
    pub(crate) body: Weak<RwLock<TrackerBody>>,
}

impl WeakTracker {
    pub fn upgrade(&self) -> Option<Tracker> {
        let body = Weak::upgrade(&self.body);
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
