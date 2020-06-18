use std::hash::Hash;
use std::{fmt::Debug, ops::Deref, sync::Arc};

use crate::context::EvalContext;
use crate::hashed::Hashed;
use crate::observable::{Observable, Ref};
use crate::{
    tracker::{Evaluation, Tracker},
    Value,
};

use parking_lot::{RwLock, RwLockReadGuard};

pub trait ComputedFactory<T> {
    fn eval(&mut self, ctx: &EvalContext) -> Option<T>;
}

impl<T, H> ComputedFactory<T> for H
where
    T: Hash + 'static,
    H: FnMut(&EvalContext) -> T,
{
    fn eval(&mut self, ctx: &EvalContext) -> Option<T> {
        Some((self)(ctx))
    }
}

pub struct Computed<T>
where
    T: Hash + 'static,
{
    body: Arc<ComputedBody<T>>,
}

impl<T> Observable<T> for Computed<T>
where
    T: Hash + 'static,
{
    fn access(&self, ctx: Option<&EvalContext>) -> Ref<T> {
        self.body.access(ctx)
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    where
        T: Debug,
    {
        self.body.debug(f)
    }
}

impl<T> Clone for Computed<T>
where
    T: Hash + 'static,
{
    fn clone(&self) -> Self {
        Computed {
            body: self.body.clone(),
        }
    }
}

impl<T> Deref for Computed<T>
where
    T: Hash + 'static,
{
    type Target = Arc<ComputedBody<T>>;
    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

impl<T> Computed<T>
where
    T: Hash + 'static,
{
    pub fn new(func: impl Fn(&EvalContext) -> T + 'static) -> Self {
        Self::create(None, Some(Box::new(func)))
    }

    pub fn with(func: impl ComputedFactory<T> + 'static) -> Self {
        Self::create(None, Some(Box::new(func)))
    }

    pub fn create(value: Option<T>, func: Option<Box<dyn ComputedFactory<T>>>) -> Self {
        let tracker = Tracker::new();
        let body = Arc::new(ComputedBody {
            current: RwLock::new(value.map(|v| Hashed::new(v))),
            func: RwLock::new(func),
            tracker: tracker.clone(),
        });

        let computed = Computed { body };

        Tracker::set_eval(&tracker, computed.body.clone());
        computed
    }

    pub fn uninit() -> Self {
        Self::create(None, None)
    }
}

pub struct ComputedBody<T>
where
    T: Hash + 'static,
{
    current: RwLock<Option<Hashed<T>>>,
    func: RwLock<Option<Box<dyn ComputedFactory<T>>>>,
    tracker: Tracker,
}

impl<T> ComputedBody<T>
where
    T: Hash + 'static,
{
    pub fn set_func(&self, func: Box<dyn ComputedFactory<T>>) {
        *self.func.write() = Some(func);
    }
}

impl<T> Deref for ComputedBody<T>
where
    T: Hash + 'static,
{
    type Target = Tracker;
    fn deref(&self) -> &Self::Target {
        &self.tracker
    }
}

impl<T> Evaluation for ComputedBody<T>
where
    T: Hash + 'static,
{
    fn eval(&self, ctx: &EvalContext) -> u64 {
        let mut func = self.func.write();
        let func = func.as_mut().expect("Function should be initialized");
        let next = func.eval(ctx);
        if next.is_none() {
            // None means we don't want to do anything
            return self.hash();
        }
        let hashed = Hashed::new(next.unwrap());
        let hash = hashed.hash;
        *self.current.write() = Some(hashed);
        hash
    }
}

impl<T> Observable<T> for ComputedBody<T>
where
    T: Hash + 'static,
{
    fn access(&self, ctx: Option<&EvalContext>) -> Ref<T> {
        self.tracker.access(ctx);
        Ref::Lock(RwLockReadGuard::map(self.current.read(), |v| {
            &v.as_ref().unwrap().value
        }))
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    where
        T: Debug,
    {
        write!(f, "Computed[{:?}]", self.current.read())
    }
}

impl<T> From<Computed<T>> for Value<T>
where
    T: Hash + 'static,
{
    fn from(value: Computed<T>) -> Self {
        Value { value: value.body }
    }
}

impl<T> From<&Computed<T>> for Value<T>
where
    T: Hash + 'static,
{
    fn from(value: &Computed<T>) -> Self {
        Value {
            value: value.body.clone(),
        }
    }
}
