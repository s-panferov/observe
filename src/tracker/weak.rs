use snowflake::ProcessUniqueId;
use std::hash::{Hash, Hasher};

use super::TrackerImpl;
use crate::Tracker;

pub struct WeakTracker<I: TrackerImpl> {
    pub(crate) id: ProcessUniqueId,
    pub(crate) body: I::WeakBody,
}

impl<Impl> WeakTracker<Impl>
where
    Impl: TrackerImpl,
{
    pub fn upgrade(&self) -> Option<Tracker<Impl>> {
        let body = Impl::upgrade(&self.body);
        body.map(|body| Tracker {
            id: self.id.clone(),
            body,
        })
    }
}

impl<Impl> PartialEq for WeakTracker<Impl>
where
    Impl: TrackerImpl,
{
    fn eq(&self, other: &WeakTracker<Impl>) -> bool {
        self.id == other.id
    }
}

impl<Impl> Hash for WeakTracker<Impl>
where
    Impl: TrackerImpl,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<Impl> Eq for WeakTracker<Impl> where Impl: TrackerImpl {}

impl<Impl> Clone for WeakTracker<Impl>
where
    Impl: TrackerImpl,
{
    fn clone(&self) -> Self {
        WeakTracker {
            id: self.id.clone(),
            body: Impl::clone_weak_body(&self.body),
        }
    }
}
