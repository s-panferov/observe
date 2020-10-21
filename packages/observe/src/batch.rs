use std::collections::HashSet;

use crate::tracker::WeakTracker;

pub struct Batch {
    changed: HashSet<WeakTracker>,
}

impl Batch {
    fn new() -> Self {
        Batch {
            changed: HashSet::new(),
        }
    }

    pub(crate) fn mark_changed(&mut self, tracker: WeakTracker) -> bool {
        self.changed.insert(tracker)
    }

    fn complete(&mut self) {
        for tracker in &self.changed {
            if let Some(tracker) = tracker.upgrade() {
                tracker.notify_reactions()
            }
        }
    }
}

pub fn batch<F: FnOnce(&mut Batch)>(outer: Option<&mut Batch>, func: F) {
    if outer.is_some() {
        let batch = outer.unwrap();
        func(batch);
    } else {
        let mut batch = Batch::new();
        func(&mut batch);
        batch.complete();
    };
}
