use crate::context::TrackerBody;
use std::collections::hash_map::DefaultHasher;
use std::{
    any::Any,
    hash::{Hash, Hasher},
    sync::Arc,
};

use crate::context::EvalContext;

pub struct ComputedBody<T: Hash, F: Fn(&mut EvalContext) -> T> {
    current: Option<Arc<T>>,
    func: F,
}

impl<T: Hash, F: Fn(&mut EvalContext) -> T> ComputedBody<T, F> {
    pub fn new(value: Option<T>, func: F) -> Self {
        ComputedBody {
            current: value.map(Arc::new),
            func,
        }
    }
}

impl<T: Hash + Send + Sync + 'static, F: Fn(&mut EvalContext) -> T> TrackerBody
    for ComputedBody<T, F>
{
    fn evaluate(&mut self, ctx: &mut EvalContext) -> u64 {
        let next = (self.func)(ctx);
        let mut hasher = DefaultHasher::new();
        next.hash(&mut hasher);
        self.current.replace(Arc::new(next));
        hasher.finish()
    }

    /// Get the current value
    ///
    /// Returns "next" value inside the transaction or
    /// the "current" value outsize.
    fn get(&self) -> Arc<dyn Any + Send + Sync> {
        self.current.as_ref().unwrap().clone()
    }
}

pub struct ValueBody<T: Hash> {
    current: Arc<T>,
    hash: u64,
}

impl<T: Hash> ValueBody<T> {
    pub fn new(value: T) -> Self {
        let hash = ValueBody::hash(&value);
        ValueBody {
            current: Arc::new(value),
            hash,
        }
    }

    fn hash(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }
}

impl<T: Hash + Send + Sync + 'static + Send + Sync> TrackerBody for ValueBody<T> {
    fn evaluate(&mut self, _ctx: &mut EvalContext) -> u64 {
        self.hash
    }

    /// Get the current value
    ///
    /// Returns "next" value inside the transaction or
    /// the "current" value outsize.
    fn get(&self) -> Arc<dyn Any + Send + Sync> {
        return self.current.clone();
    }

    fn set(&mut self, next: Arc<dyn Any + Send + Sync>) -> u64 {
        let next_typed = next.downcast::<T>().unwrap();
        let next_hash = Self::hash(&next_typed);

        if self.hash != next_hash {
            self.hash = next_hash;
            self.current = next_typed;
        }

        next_hash
    }
}
