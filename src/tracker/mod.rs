use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::mem;

use snowflake::ProcessUniqueId;

use crate::context::EvalContext;
use crate::eval::{Evaluation, Invalidate};
use crate::{transaction::Transaction, types::*};

use tracing::{event, Level};

mod imp;
mod weak;

use fmt::Debug;

pub use imp::*;
pub use weak::*;

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

pub struct Tracker<Impl: TrackerImpl = Local> {
    id: ProcessUniqueId,
    body: Impl::Body,
}

impl<Impl> Clone for Tracker<Impl>
where
    Impl: TrackerImpl,
{
    fn clone(&self) -> Self {
        Tracker {
            id: self.id.clone(),
            body: Impl::clone_body(&self.body),
        }
    }
}

impl<Impl> Tracker<Impl>
where
    Impl: TrackerImpl,
{
    pub fn new(name: String) -> Self {
        let body = RawTracker::new(name);
        let tracker = Tracker {
            id: body.id.clone(),
            body: Impl::wrap(body),
        };

        let reference = tracker.weak();
        Impl::write(&tracker.body, move |body| {
            body.reference = Some(reference);
        });

        tracker
    }

    pub fn name(&self) -> String {
        Impl::read(&self.body, |inner| inner.name.clone())
    }

    pub fn set_name(&self, name: String) {
        Impl::write(&self.body, |inner| inner.name = name)
    }

    pub fn weak(&self) -> WeakTracker<Impl> {
        return WeakTracker {
            id: self.id.clone(),
            body: Impl::downgrade(&self.body),
        };
    }

    pub fn before_access(&self, ctx: Option<&mut EvalContext<Impl>>) {
        if ctx.is_some() {
            ctx.unwrap().access(self.clone());
        }
        self.update()
    }

    pub fn get(
        &self,
        ctx: Option<&mut EvalContext<Impl>>,
    ) -> <Impl::Ptr as Apply<Impl::Any>>::Result {
        self.before_access(ctx);
        Impl::read(&self.body, |inner| inner.body.get())
    }

    pub fn change<F>(&self, tx: Option<&mut Transaction<Impl>>, closure: F)
    where
        F: FnOnce(&mut Impl::Eval) -> (u64, Invalidate),
    {
        let changed = Impl::write(&self.body, |inner| {
            let (new_hash, invalidate) = closure(&mut *inner.body);
            inner.update_hash(new_hash, invalidate)
        });

        if changed {
            if let Some(tx) = tx {
                tx.mark_changed(self.weak().clone());
            } else {
                self.notify_reactions()
            }
        }
    }

    pub fn set(
        &self,
        tx: Option<&mut Transaction<Impl>>,
        value: <Impl::Ptr as Apply<Impl::Any>>::Result,
    ) {
        let span = tracing::span!(Level::TRACE, "set", tracker = &*self.name());
        let _guard = span.enter();

        self.change(tx, move |eval| eval.set(value))
    }

    pub fn hash(&self) -> u64 {
        Impl::read(&self.body, |inner| inner.hash)
    }

    pub fn expire(&self) {
        Impl::write(&self.body, |inner| {
            inner.expire(Expired::ForSure, Invalidate::SelfAndDeps)
        })
    }

    pub fn update(&self) {
        if Impl::read(&self.body, |inner| inner.should_evaluate()) {
            Impl::write(&self.body, |inner| inner.evaluate())
        }
    }

    pub fn autorun(&self) {
        Impl::write(&self.body, |inner| inner.is_observer = true)
    }

    pub fn notify_reactions(&self) {
        let (is_observer, is_scheduled, is_observed) = Impl::read(&self.body, |inner| {
            let is_observer = inner.is_observer;
            let is_scheduled = inner.body.is_scheduled();
            let is_observed = inner.is_observed();
            (is_observer, is_scheduled, is_observed)
        });

        if is_observer {
            if is_scheduled {
                Impl::write(&self.body, |inner| inner.body.on_reaction());
            } else {
                self.update()
            }
        }

        if is_observed {
            event!(
                Level::INFO,
                tracker = &*self.name(),
                "Tracker is observed, notifying observers..."
            );
            // We should have a reaction somewhere up
            let used_by = Impl::read(&self.body, |inner| inner.used_by.clone());
            for tracker in used_by.into_iter() {
                let tracker = tracker.upgrade();
                if tracker.is_some() {
                    let tracker = tracker.unwrap();
                    event!(
                        Level::INFO,
                        tracker = &*self.name(),
                        "Notifying {}",
                        tracker.name()
                    );
                    tracker.notify_reactions()
                }
            }
        }
    }

    pub fn state(&self) -> Freshness {
        Impl::read(&self.body, |inner| inner.state.clone())
    }

    pub fn set_computation(&self, cb: Box<Impl::Eval>) {
        Impl::write(&self.body, move |inner| inner.body = cb)
    }
}

impl<Impl: TrackerImpl> PartialEq for Tracker<Impl> {
    fn eq(&self, other: &Tracker<Impl>) -> bool {
        self.id == other.id
    }
}

impl<Impl: TrackerImpl> Hash for Tracker<Impl> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<Impl: TrackerImpl> Eq for Tracker<Impl> {}

pub struct RawTracker<Impl>
where
    Impl: TrackerImpl,
{
    id: ProcessUniqueId,
    name: String,
    hash: u64,
    used_by: HashSet<WeakTracker<Impl>>,
    based_on: HashMap<Tracker<Impl>, u64>,
    is_observer: bool,
    state: Freshness,
    observation_counter: i8,
    reference: Option<WeakTracker<Impl>>,
    body: Box<Impl::Eval>,
}

impl<Impl> Drop for RawTracker<Impl>
where
    Impl: TrackerImpl,
{
    fn drop(&mut self) {
        let self_ref = self.weak_ref();
        let is_observer = self.is_observer;
        for (tracker, _) in self.based_on.iter() {
            Impl::write(&tracker.body, move |inner| {
                inner.report_not_more_used_by(&self_ref);
                if is_observer {
                    inner.dec_observers()
                }
            })
        }
    }
}

struct EmptyBody {}

impl<Impl> Evaluation<Impl> for EmptyBody
where
    Impl: TrackerImpl,
{
    fn evaluate(&mut self, _: &mut EvalContext<Impl>) -> u64 {
        0
    }
}

impl<Impl> RawTracker<Impl>
where
    Impl: TrackerImpl,
{
    fn new(name: String) -> Self {
        RawTracker {
            id: ProcessUniqueId::new(),
            name,
            hash: 0,
            used_by: HashSet::new(),
            based_on: HashMap::new(),
            is_observer: false,
            state: Freshness::Expired(Expired::ForSure),
            reference: None,
            observation_counter: 0,
            body: Impl::empty_body(),
        }
    }

    pub fn weak_ref(&self) -> &WeakTracker<Impl> {
        self.reference.as_ref().unwrap()
    }

    pub fn is_observed(&self) -> bool {
        self.observation_counter > 0
    }

    pub fn should_evaluate(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "should_evaluate", tracker = &*self.name);
        let _guard = span.enter();

        match self.state {
            Freshness::UpToDate => {
                event!(Level::TRACE, "Up to date");
                false
            }
            Freshness::Expired(Expired::ForSure) => {
                event!(Level::TRACE, "Expired");
                true
            }
            Freshness::Expired(Expired::Maybe) => {
                let mut changed = false;
                for (tracker, hash) in self.based_on.iter() {
                    tracker.update();
                    if &tracker.hash() != hash {
                        event!(
                            Level::TRACE,
                            "Expired because dep. tracker was changed: {}",
                            tracker.name()
                        );
                        changed = true;
                        break;
                    }
                }
                if !changed {
                    event!(Level::TRACE, "All deps are up to date");
                }
                changed
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

    pub fn evaluate(&mut self) {
        let span = tracing::span!(Level::TRACE, "evaluate", tracker = &*self.name);
        let _guard = span.enter();

        let body = self.body.as_mut();
        let prev_used = mem::replace(&mut self.based_on, HashMap::new());
        let mut ctx = EvalContext::new(prev_used);
        let hash = body.evaluate(&mut ctx);
        let using = HashMap::from_iter(ctx.using.iter().map(|t| {
            let version = t.hash();
            (t.clone(), version)
        }));

        self.hash = hash;
        self.based_on = using;
        self.state = Freshness::UpToDate;

        let self_ref = self.weak_ref();
        let is_observer = self.is_observer;

        for based_on in ctx.diff_added() {
            Impl::write(&based_on.body, move |inner| {
                inner.report_used_by(self_ref.clone());
                if is_observer {
                    inner.inc_observers()
                }
            })
        }

        for based_on in ctx.diff_removed() {
            Impl::write(&based_on.body, move |inner| {
                inner.report_not_more_used_by(&self_ref);
                if is_observer {
                    inner.dec_observers()
                }
            })
        }
    }

    /// Called by Reactions
    pub fn inc_observers(&mut self) {
        self.observation_counter += 1;

        //become observable
        if self.observation_counter == 1 {
            self.body.on_become_observed();
            for (tracker, _) in &mut self.based_on.iter() {
                Impl::write(&tracker.body, move |inner| inner.inc_observers())
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
                Impl::write(&tracker.body, move |inner| inner.dec_observers())
            }
        }
    }

    fn expire(&mut self, expire: Expired, inv: Invalidate) {
        match &self.state {
            Freshness::Expired(state) if state == &expire => return,
            Freshness::Expired(Expired::ForSure) => return,
            _ => {}
        }

        if matches!(inv, Invalidate::SelfAndDeps) {
            self.state = Freshness::Expired(expire);
        }

        for derived in &mut self.used_by.iter() {
            if let Some(derived) = derived.upgrade() {
                Impl::write(&derived.body, move |inner| {
                    inner.expire(Expired::Maybe, Invalidate::SelfAndDeps)
                });
            }
        }
    }

    fn report_used_by(&mut self, tracker: WeakTracker<Impl>) {
        self.used_by.insert(tracker);
    }

    fn report_not_more_used_by(&mut self, tracker: &WeakTracker<Impl>) {
        self.used_by.remove(tracker);
    }
}

impl<Impl> fmt::Debug for RawTracker<Impl>
where
    Impl: TrackerImpl,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Tracker[name: {}, id: {}]", self.name, self.id)
    }
}

impl<Impl> PartialEq for RawTracker<Impl>
where
    Impl: TrackerImpl,
{
    fn eq(&self, other: &RawTracker<Impl>) -> bool {
        self.id == other.id
    }
}

impl<Impl> Hash for RawTracker<Impl>
where
    Impl: TrackerImpl,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<Impl> Eq for RawTracker<Impl> where Impl: TrackerImpl {}

#[cfg(test)]
mod tests {
    use super::{Expired, Freshness, Local, Tracker, TrackerImpl};
    use crate::context::EvalContext;
    use crate::test::{SharedMock, Spy};

    #[test]
    fn call_computation_on_update() {
        let tracker = Tracker::<Local>::new("Test".to_owned());
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
        let a = Tracker::<Local>::new("A".to_owned());
        let b = Tracker::<Local>::new("B".to_owned());
        let c = Tracker::<Local>::new("C".to_owned());

        a.set_computation({
            let b = b.clone();
            Box::new(move |ctx: &mut EvalContext<_>| {
                b.update();
                ctx.access(b.clone());
                0
            })
        });

        b.set_computation({
            let c = c.clone();
            Box::new(move |ctx: &mut EvalContext<_>| {
                c.update();
                ctx.access(c.clone());
                0
            })
        });

        a.update();

        assert_eq!(Local::read(&a.body, |inner| inner.based_on.len()), 1);
        assert_eq!(Local::read(&b.body, |inner| inner.based_on.len()), 1);
        assert_eq!(Local::read(&c.body, |inner| inner.based_on.len()), 0);

        assert_eq!(Local::read(&c.body, |inner| inner.used_by.len()), 1);
        assert_eq!(Local::read(&b.body, |inner| inner.used_by.len()), 1);
        assert_eq!(Local::read(&a.body, |inner| inner.used_by.len()), 0);

        assert_eq!(c.state(), Freshness::UpToDate);
        assert_eq!(c.state(), Freshness::UpToDate);
        assert_eq!(c.state(), Freshness::UpToDate);

        c.expire();

        assert_eq!(a.state(), Freshness::Expired(Expired::Maybe));
        assert_eq!(b.state(), Freshness::Expired(Expired::Maybe));
        assert_eq!(c.state(), Freshness::Expired(Expired::ForSure));

        a.update();

        assert_eq!(c.state(), Freshness::UpToDate);
        assert_eq!(c.state(), Freshness::UpToDate);
        assert_eq!(c.state(), Freshness::UpToDate);
    }
}
