use std::collections::HashSet;

use crate::tracker::WeakTracker;

pub struct Transaction {
    changed: HashSet<WeakTracker>,
}

impl Transaction {
    fn new() -> Self {
        Transaction {
            changed: HashSet::new(),
        }
    }

    pub(crate) fn mark_changed(&mut self, tracker: WeakTracker) -> bool {
        self.changed.insert(tracker)
    }

    fn commit(&mut self) {
        for tracker in &self.changed {
            if let Some(tracker) = tracker.upgrade() {
                tracker.notify_reactions()
            }
        }
    }
}

pub fn transaction<F: FnOnce(&mut Transaction)>(outer: Option<&mut Transaction>, func: F) {
    if outer.is_some() {
        let tx = outer.unwrap();
        func(tx);
    } else {
        let mut tx = Transaction::new();
        func(&mut tx);
        tx.commit();
    };
}
