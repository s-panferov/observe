use core::any::Any;
use std::collections::{HashMap, HashSet};
use std::{iter::FromIterator, sync::Arc};

use crate::tracker::Tracker;

pub trait TrackerBody {
    fn evaluate(&mut self, ctx: &mut EvalContext) -> u64;
    fn get(&self) -> Arc<dyn Any + Send + Sync> {
        unimplemented!();
    }
    fn set(&mut self, _value: Arc<dyn Any + Send + Sync>) -> u64 {
        unimplemented!();
    }
}

impl<T> TrackerBody for T
where
    T: FnMut(&mut EvalContext) -> u64,
{
    fn evaluate(&mut self, ctx: &mut EvalContext) -> u64 {
        self(ctx)
    }
}

pub struct EvalContext {
    pub(crate) prev_used: HashSet<Tracker>,
    pub(crate) using: HashSet<Tracker>,
}

impl EvalContext {
    pub fn new(prev_used: HashMap<Tracker, u64>) -> Self {
        let prev_used = HashSet::from_iter(prev_used.keys().cloned());
        EvalContext {
            prev_used,
            using: HashSet::new(),
        }
    }

    pub fn access(&mut self, tracker: Tracker) {
        self.using.insert(tracker);
    }

    pub fn diff_added(&self) -> impl Iterator<Item = &Tracker> {
        self.using.difference(&self.prev_used)
    }

    pub fn diff_removed(&self) -> impl Iterator<Item = &Tracker> {
        self.prev_used.difference(&self.using)
    }

    pub fn into_used(self) -> HashSet<Tracker> {
        self.using
    }
}
