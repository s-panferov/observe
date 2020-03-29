use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    rc::Rc,
};

use crate::context::EvalContext;
use crate::eval::{AnyValue, Evaluation};

pub struct Variable<T: Hash> {
    current: Rc<T>,
    hash: u64,
}

impl<T: Hash> Variable<T> {
    pub fn new(value: T) -> Self {
        let hash = Variable::hash(&value);
        Variable {
            current: Rc::new(value),
            hash,
        }
    }

    pub fn hash(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }
}

impl<T: Hash + 'static> Evaluation for Variable<T> {
    fn evaluate(&mut self, _ctx: &mut EvalContext) -> u64 {
        self.hash
    }

    /// Get the current value
    ///
    /// Returns "next" value inside the transaction or
    /// the "current" value outsize.
    fn get(&self) -> AnyValue {
        return self.current.clone();
    }

    fn set(&mut self, next: AnyValue) -> u64 {
        let next = next.downcast::<T>().unwrap();
        let next_hash = Self::hash(&next);
        if self.hash != next_hash {
            self.hash = next_hash;
            self.current = next;
        }
        next_hash
    }
}
