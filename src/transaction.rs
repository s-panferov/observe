use std::collections::HashSet;

use crate::tracker::{TrackerImpl, WeakTracker};

pub struct Transaction<Impl>
where
    Impl: TrackerImpl,
{
    changed: HashSet<WeakTracker<Impl>>,
}

impl<Impl> Transaction<Impl>
where
    Impl: TrackerImpl,
{
    pub fn new() -> Self {
        Transaction {
            changed: HashSet::new(),
        }
    }

    pub fn mark_changed(&mut self, tracker: WeakTracker<Impl>) -> bool {
        self.changed.insert(tracker)
    }

    fn commit(&mut self) {
        for tracker in &self.changed {
            let tracker = tracker.upgrade();
            if tracker.is_some() {
                tracker.unwrap().notify_reactions()
            }
        }
    }
}

pub fn transaction<F: FnOnce(&mut Transaction<Impl>), Impl>(
    outer: Option<&mut Transaction<Impl>>,
    func: F,
) where
    Impl: TrackerImpl,
{
    if outer.is_some() {
        let tx = outer.unwrap();
        func(tx);
    } else {
        let mut tx = Transaction::new();
        func(&mut tx);
        tx.commit();
    };
}
