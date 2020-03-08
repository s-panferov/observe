use std::{fmt::Debug, hash::Hash, sync::Arc};

use crate::computed::Computed;
use crate::context::EvalContext;
use crate::variable::Var;

#[derive(Debug)]
pub enum Value<T: Hash + Send + Sync> {
    Const(Arc<T>),
    Var(Var<T>),
    Computed(Computed<T>),
}

impl<T: Hash + Default + Send + Sync> Default for Value<T> {
    fn default() -> Self {
        Value::Const(Arc::new(T::default()))
    }
}

impl<T: Hash + Send + Sync> Clone for Value<T> {
    fn clone(&self) -> Value<T> {
        match self {
            Value::Const(v) => Value::Const(v.clone()),
            Value::Var(v) => Value::Var(v.clone()),
            Value::Computed(v) => Value::Computed(v.clone()),
        }
    }
}

impl<T> From<T> for Value<T>
where
    T: Send + Sync + Hash,
{
    fn from(value: T) -> Value<T> {
        Value::Const(Arc::new(value))
    }
}

impl<T> From<T> for Value<Option<T>>
where
    T: Send + Sync + Hash,
{
    fn from(value: T) -> Value<Option<T>> {
        Value::Const(Arc::new(Some(value)))
    }
}

impl<T> From<Computed<T>> for Value<T>
where
    T: Hash + Send + Sync + Debug + 'static,
{
    fn from(value: Computed<T>) -> Value<T> {
        Value::Computed(value)
    }
}

impl<T> From<Var<T>> for Value<T>
where
    T: Hash + Send + Sync + Debug + 'static,
{
    fn from(value: Var<T>) -> Value<T> {
        Value::Var(value)
    }
}

impl<T> Value<T>
where
    T: Hash + Send + Sync + Debug + 'static,
{
    pub fn observe(&self, ctx: &mut EvalContext) -> Arc<T> {
        match self {
            Value::Const(v) => v.clone(),
            Value::Var(v) => v.observe(ctx),
            Value::Computed(v) => v.observe(ctx),
        }
    }

    pub fn once(&self) -> Arc<T> {
        match self {
            Value::Const(v) => v.clone(),
            Value::Var(v) => v.once(),
            Value::Computed(v) => v.once(),
        }
    }

    pub fn as_var(&self) -> Option<&Var<T>> {
        match self {
            Value::Const(_) => None,
            Value::Var(v) => Some(v),
            Value::Computed(_) => None,
        }
    }

    pub fn as_const(&self) -> Option<&Arc<T>> {
        match self {
            Value::Const(a) => Some(a),
            Value::Var(_v) => None,
            Value::Computed(_) => None,
        }
    }

    pub fn as_computed(&self) -> Option<&Computed<T>> {
        match self {
            Value::Const(_) => None,
            Value::Var(_) => None,
            Value::Computed(c) => Some(c),
        }
    }

    pub fn map<R, F>(&self, handler: F) -> Computed<R>
    where
        F: Fn(&mut EvalContext, &T) -> R + 'static,
        R: Hash + Send + Sync + 'static,
    {
        let this = self.clone();
        Computed::new(move |ctx| {
            let value = this.observe(ctx);
            handler(ctx, &value)
        })
    }
}
