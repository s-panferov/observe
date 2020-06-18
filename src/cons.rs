use std::hash::Hash;
use std::{ops::Deref, sync::Arc};

use crate::context::EvalContext;
use crate::hashed::Hashed;
use crate::observable::{Observable, Ref};
use crate::{tracker::Evaluation, Value};

pub struct Const<T>
where
    T: Hash + 'static,
{
    body: Arc<ConstBody<T>>,
}

impl<T> Clone for Const<T>
where
    T: Hash + 'static,
{
    fn clone(&self) -> Self {
        Const {
            body: self.body.clone(),
        }
    }
}

impl<T> Observable<T> for Const<T>
where
    T: Hash + 'static,
{
    fn access(&self, ctx: Option<&EvalContext>) -> Ref<T> {
        self.body.access(ctx)
    }
}

impl<T> Deref for Const<T>
where
    T: Hash + 'static,
{
    type Target = Arc<ConstBody<T>>;
    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

pub struct ConstBody<T> {
    hashed: Hashed<T>,
}

impl<T> Const<T>
where
    T: Hash + 'static,
{
    pub fn new(value: T) -> Self {
        let body = Arc::new(ConstBody {
            hashed: Hashed::new(value),
        });
        Const { body }
    }
}

impl<T> Evaluation for ConstBody<T>
where
    T: Hash + 'static,
{
    fn eval(&self, _ctx: &EvalContext) -> u64 {
        self.hashed.hash
    }
}

impl<T> Observable<T> for ConstBody<T>
where
    T: Hash + 'static,
{
    fn access(&self, _ctx: Option<&EvalContext>) -> Ref<T> {
        Ref::Ref(&self.hashed.value)
    }
}

impl<T> From<Const<T>> for Value<T>
where
    T: Hash + 'static,
{
    fn from(value: Const<T>) -> Self {
        Value { value: value.body }
    }
}
