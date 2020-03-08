use core::any::Any;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::mem;
use std::sync::{Arc, Weak};

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use snowflake::ProcessUniqueId;

use crate::context::{EvalContext, TrackerBody};
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
    body: Arc<RwLock<RawTracker>>,
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
            body: Arc::new(RwLock::new(body)),
        };

        tracker.get_mut().reference = Some(tracker.weak());
        tracker
    }

    pub fn weak(&self) -> WeakTracker {
        return WeakTracker {
            id: self.id.clone(),
            body: Arc::downgrade(&self.body),
        };
    }

    pub fn get(&self) -> RwLockReadGuard<RawTracker> {
        self.body.read()
    }

    pub fn get_mut(&self) -> RwLockWriteGuard<RawTracker> {
        self.body.write()
    }

    pub fn notify_reactions(&self) {
        if self.get().is_observer {
            let mut this = self.get_mut();
            match &this.reaction_cb {
                Some(cb) => cb(),
                None => this.update(),
            }
        } else if self.get().is_observed() {
            // We should have a reaction somewhere up
            let used_by = self.get().used_by.clone();
            for tracker in used_by.into_iter() {
                let tracker = tracker.upgrade();
                if tracker.is_some() {
                    tracker.unwrap().notify_reactions()
                }
            }
        }
    }
}

pub struct RawTracker {
    id: ProcessUniqueId,
    name: String,
    hash: u64,
    // lock: Lock,
    used_by: HashSet<WeakTracker>,
    based_on: HashMap<Tracker, u64>,
    state: Freshness,
    observation_counter: i8,
    is_observer: bool,
    reference: Option<WeakTracker>,
    reaction_cb: Option<Box<dyn Fn()>>,
    observed_cb: Option<Box<dyn Fn()>>,
    unobserved_cb: Option<Box<dyn Fn()>>,
    body: Option<Box<dyn TrackerBody>>,
}

impl Drop for RawTracker {
    fn drop(&mut self) {
        for (tracker, _) in self.based_on.iter() {
            let mut tracker_body = tracker.get_mut();
            tracker_body.report_not_more_used_by(self.weak_ref());
            if self.is_observer {
                tracker_body.dec_observers()
            }
        }
    }
}

impl RawTracker {
    fn new(name: String) -> Self {
        RawTracker {
            id: ProcessUniqueId::new(),
            name,
            hash: 0,
            // lock: Lock::new(),
            used_by: HashSet::new(),
            based_on: HashMap::new(),
            state: Freshness::Expired(Expired::ForSure),
            reference: None,
            is_observer: false,
            observation_counter: 0,
            reaction_cb: None,
            observed_cb: None,
            unobserved_cb: None,
            body: None,
        }
    }

    pub fn weak_ref(&self) -> &WeakTracker {
        self.reference.as_ref().unwrap()
    }

    pub fn strong_ref(&self) -> Tracker {
        self.weak_ref().upgrade().unwrap()
    }

    pub fn on_observed<F: Fn() + 'static>(&mut self, cb: F) {
        self.observed_cb = Some(Box::new(cb));
    }

    pub fn on_unobserved<F: Fn() + 'static>(&mut self, cb: F) {
        self.unobserved_cb = Some(Box::new(cb));
    }

    pub fn on_reaction<F: Fn() + 'static>(&mut self, cb: F) {
        self.reaction_cb = Some(Box::new(cb));
    }

    pub fn set_computation<F: TrackerBody + 'static>(&mut self, cb: F) {
        self.body = Some(Box::new(cb));
    }

    pub fn set_is_observer(&mut self) {
        self.is_observer = true;
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

    pub fn based_on(&self) -> &HashMap<Tracker, u64> {
        &self.based_on
    }

    pub fn report_changed(&mut self, tx: &mut Transaction) {
        // try to grab a write lock to avoid concurrent updates
        // self.lock.lock();
        self.expire(Expired::ForSure);
        tx.mark_changed(self.weak_ref().clone());
    }

    pub fn should_evaluate(&self) -> bool {
        match self.state {
            Freshness::UpToDate => false,
            Freshness::Expired(Expired::ForSure) => true,
            Freshness::Expired(Expired::Maybe) => {
                let mut changed = false;
                for (tracker, hash) in self.based_on.iter() {
                    if tracker.get().should_evaluate() {
                        tracker.get_mut().evaluate();
                    }
                    if &tracker.get().hash != hash {
                        changed = true;
                        break;
                    }
                }
                changed
            }
        }
    }

    pub fn update(&mut self) {
        if self.should_evaluate() {
            self.evaluate()
        }
    }

    pub fn get(&self) -> Arc<dyn Any + Send + Sync> {
        self.body.as_ref().unwrap().get()
    }

    pub fn set(&mut self, value: Arc<dyn Any + Send + Sync>) {
        let new_hash = self.body.as_mut().unwrap().set(value);
        if self.hash != new_hash {
            self.hash = new_hash;
            self.expire(Expired::ForSure);
        }
    }

    pub fn evaluate(&mut self) {
        let evaluate = self.body.as_mut();
        let prev_used = mem::replace(&mut self.based_on, HashMap::new());
        let ctx = match evaluate {
            Some(ev) => {
                let mut ctx = EvalContext::new(prev_used);
                let hash = ev.evaluate(&mut ctx);
                let using = HashMap::from_iter(ctx.using.iter().map(|t| {
                    let version = t.get().hash;
                    (t.clone(), version)
                }));

                self.hash = hash;
                self.based_on = using;
                ctx
            }
            None => EvalContext::new(HashMap::new()),
        };

        self.state = Freshness::UpToDate;

        for based_on in ctx.diff_added() {
            let mut based_on = based_on.get_mut();
            based_on.report_used_by(self.weak_ref().clone());
            if self.is_observer {
                based_on.inc_observers()
            }
        }

        for based_on in ctx.diff_removed() {
            let mut based_on = based_on.get_mut();
            based_on.report_not_more_used_by(self.weak_ref());
            if self.is_observer {
                based_on.dec_observers()
            }
        }
    }

    pub fn id(&self) -> &ProcessUniqueId {
        &self.id
    }

    /// Called by Reactions
    pub fn inc_observers(&mut self) {
        self.observation_counter += 1;

        //become observable
        if self.observation_counter == 1 {
            {
                let observed_cb = self.observed_cb.as_ref();
                if observed_cb.is_some() {
                    observed_cb.unwrap()();
                };
            }

            for (tracker, _) in &mut self.based_on.iter() {
                tracker.get_mut().inc_observers()
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
            {
                let unobserved_cb = self.unobserved_cb.as_ref();
                if unobserved_cb.is_some() {
                    unobserved_cb.unwrap()();
                };
            }

            for (tracker, _) in &mut self.based_on.iter() {
                tracker.get_mut().dec_observers()
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
                derived.unwrap().get_mut().expire(Expired::Maybe)
            }
        }
    }

    pub fn state(&self) -> &Freshness {
        &self.state
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
    body: Weak<RwLock<RawTracker>>,
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
        tracker.get_mut().set_computation({
            let mock = spy.clone();
            move |_: &mut _| {
                mock.get().trigger();
                0
            }
        });

        spy.get().expect_trigger().return_const(()).times(1);
        tracker.get_mut().update();

        spy.get().checkpoint();

        spy.get().expect_trigger().return_const(()).times(0);
        tracker.get_mut().update();
    }

    #[test]
    fn properly_expire() {
        let a = Tracker::new("A".to_owned());
        let b = Tracker::new("B".to_owned());
        let c = Tracker::new("C".to_owned());

        a.get_mut().set_computation({
            let b = b.clone();
            move |ctx: &mut EvalContext| {
                b.get_mut().update();
                ctx.access(b.clone());
                0
            }
        });

        b.get_mut().set_computation({
            let c = c.clone();
            move |ctx: &mut EvalContext| {
                c.get_mut().update();
                ctx.access(c.clone());
                0
            }
        });

        a.get_mut().update();

        assert_eq!(a.get().based_on.len(), 1);
        assert_eq!(b.get().based_on.len(), 1);
        assert_eq!(c.get().based_on.len(), 0);

        assert_eq!(c.get().used_by.len(), 1);
        assert_eq!(b.get().used_by.len(), 1);
        assert_eq!(a.get().used_by.len(), 0);

        assert_eq!(c.get().state, Freshness::UpToDate);
        assert_eq!(c.get().state, Freshness::UpToDate);
        assert_eq!(c.get().state, Freshness::UpToDate);

        c.get_mut().expire(Expired::ForSure);

        assert_eq!(a.get().state, Freshness::Expired(Expired::Maybe));
        assert_eq!(b.get().state, Freshness::Expired(Expired::Maybe));
        assert_eq!(c.get().state, Freshness::Expired(Expired::ForSure));

        a.get_mut().update();

        assert_eq!(c.get().state, Freshness::UpToDate);
        assert_eq!(c.get().state, Freshness::UpToDate);
        assert_eq!(c.get().state, Freshness::UpToDate);
    }
}
