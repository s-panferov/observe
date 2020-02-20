use std::collections::HashSet;

use crate::tracker::WeakTracker;

pub struct Transaction {
    changed: HashSet<WeakTracker>,
}

impl Transaction {
    pub fn new() -> Self {
        Transaction {
            changed: HashSet::new(),
        }
    }

    pub fn mark_changed(&mut self, tracker: WeakTracker) -> bool {
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
