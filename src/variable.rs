use std::{
    any::Any,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    ops::Deref,
    rc::Rc,
    sync::Arc,
};

use crate::context::EvalContext;
use crate::{
    eval::{Evaluation, Invalidate},
    tracker::{Local, Shared, TrackerImpl},
    types::Apply,
    value::Value,
};

pub struct Var<T, Impl>
where
    T: Hash,
    Impl: TrackerImpl,
    Impl::Ptr: Apply<T>,
{
    current: <Impl::Ptr as Apply<T>>::Result,
    hash: u64,
}

pub fn default_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

impl<T, Impl> Var<T, Impl>
where
    T: Hash,
    Impl: TrackerImpl,
    Impl::Ptr: Apply<T>,
    <Impl::Ptr as Apply<T>>::Result: Deref<Target = T>,
{
    pub fn new(value: T) -> Self {
        let hash = default_hash(&value);
        Var {
            current: Impl::ptr_wrap(value),
            hash,
        }
    }

    fn update(&mut self, next: <Impl::Ptr as Apply<T>>::Result) -> (u64, Invalidate) {
        let next_hash = default_hash(&*next);
        if self.hash != next_hash {
            self.hash = next_hash;
            self.current = next;
        }

        (next_hash, Invalidate::OnlyDeps)
    }
}

impl<T> Evaluation<Local> for Var<T, Local>
where
    T: Hash + 'static,
{
    fn evaluate(&mut self, _ctx: &mut EvalContext<Local>) -> u64 {
        self.hash
    }

    fn get(&self) -> Rc<dyn Any + 'static> {
        return self.current.clone();
    }

    fn set(&mut self, next: Rc<dyn Any + 'static>) -> (u64, Invalidate) {
        let next = next.downcast::<T>().unwrap();
        self.update(next)
    }
}

impl<T> Evaluation<Shared> for Var<T, Shared>
where
    T: Hash + Send + Sync + 'static,
{
    fn evaluate(&mut self, _ctx: &mut EvalContext<Shared>) -> u64 {
        self.hash
    }

    fn get(&self) -> Arc<dyn Any + Send + Sync + 'static> {
        return self.current.clone();
    }

    fn set(&mut self, next: Arc<dyn Any + Send + Sync + 'static>) -> (u64, Invalidate) {
        let next = next.downcast::<T>().unwrap();
        self.update(next)
    }
}

impl<T> From<Var<T, Shared>> for Value<T, Shared>
where
    T: Hash + Send + Sync + 'static,
{
    fn from(from: Var<T, Shared>) -> Value<T, Shared> {
        let value = Value::<T, Shared>::uninit();
        value.set_computation(Box::new(from));
        value
    }
}

impl<T> From<Var<T, Local>> for Value<T, Local>
where
    T: Hash + 'static,
{
    fn from(from: Var<T, Local>) -> Value<T, Local> {
        let value = Value::<T, Local>::uninit();
        value.set_computation(Box::new(from));
        value
    }
}
