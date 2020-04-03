use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

use crate::tracker::{Tracker, TrackerImpl};

pub struct EvalContext<Impl>
where
    Impl: TrackerImpl,
{
    pub(crate) prev_used: HashSet<Tracker<Impl>>,
    pub(crate) using: HashSet<Tracker<Impl>>,
}

impl<Impl> EvalContext<Impl>
where
    Impl: TrackerImpl,
{
    pub fn new(prev_used: HashMap<Tracker<Impl>, u64>) -> Self {
        let prev_used = HashSet::from_iter(prev_used.keys().cloned());
        EvalContext {
            prev_used,
            using: HashSet::new(),
        }
    }

    // TODO optimize empty case
    pub fn empty() -> Self {
        EvalContext {
            prev_used: HashSet::new(),
            using: HashSet::new(),
        }
    }

    pub(crate) fn access(&mut self, tracker: Tracker<Impl>) {
        self.using.insert(tracker);
    }

    pub(crate) fn diff_added(&self) -> impl Iterator<Item = &Tracker<Impl>> {
        self.using.difference(&self.prev_used)
    }

    pub(crate) fn diff_removed(&self) -> impl Iterator<Item = &Tracker<Impl>> {
        self.prev_used.difference(&self.using)
    }

    pub fn into_used(self) -> HashSet<Tracker<Impl>> {
        self.using
    }
}
