use parking_lot::RwLock;
use std::collections::HashSet;

use crate::tracker::Tracker;

pub struct EvalContext {
    body: RwLock<EvalContextBody>,
}

struct EvalContextBody {
    pub(crate) using: HashSet<Tracker>,
}

impl EvalContext {
    pub fn new() -> Self {
        EvalContext {
            body: RwLock::new(EvalContextBody {
                using: HashSet::new(),
            }),
        }
    }

    // TODO optimize empty case
    pub fn empty() -> Self {
        EvalContext {
            body: RwLock::new(EvalContextBody {
                using: HashSet::new(),
            }),
        }
    }

    pub(crate) fn access(&self, tracker: Tracker) {
        self.body.write().using.insert(tracker);
    }

    pub(crate) fn into_used(self) -> HashSet<Tracker> {
        self.body.into_inner().using
    }
}
