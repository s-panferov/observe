use std::{fmt::Debug, hash::Hash, sync::Arc};

use crate::computed::Computed;
use crate::context::EvalContext;
use crate::variable::Var;

#[derive(Debug)]
pub enum MaybeValue<T: Hash + Send + Sync> {
    Empty,
    Const(Arc<Option<T>>),
    Var(Var<Option<T>>),
    Computed(Computed<Option<T>>),
}

impl<T: Hash + Send + Sync> Default for MaybeValue<T> {
    fn default() -> Self {
        MaybeValue::Empty
    }
}

impl<T: Hash + Send + Sync> Clone for MaybeValue<T> {
    fn clone(&self) -> MaybeValue<T> {
        match self {
            MaybeValue::Empty => MaybeValue::Empty,
            MaybeValue::Const(v) => MaybeValue::Const(v.clone()),
            MaybeValue::Var(v) => MaybeValue::Var(v.clone()),
            MaybeValue::Computed(v) => MaybeValue::Computed(v.clone()),
        }
    }
}

impl<T> From<T> for MaybeValue<T>
where
    T: Send + Sync + Hash,
{
    fn from(value: T) -> MaybeValue<T> {
        MaybeValue::Const(Arc::new(Some(value)))
    }
}

impl<T> From<Option<T>> for MaybeValue<T>
where
    T: Send + Sync + Hash,
{
    fn from(value: Option<T>) -> MaybeValue<T> {
        MaybeValue::Const(Arc::new(value))
    }
}

impl<T> From<Computed<Option<T>>> for MaybeValue<T>
where
    T: Hash + Send + Sync + Debug + 'static,
{
    fn from(value: Computed<Option<T>>) -> MaybeValue<T> {
        MaybeValue::Computed(value)
    }
}

impl<T> From<Var<Option<T>>> for MaybeValue<T>
where
    T: Hash + Send + Sync + Debug + 'static,
{
    fn from(value: Var<Option<T>>) -> MaybeValue<T> {
        MaybeValue::Var(value)
    }
}

impl<T> MaybeValue<T>
where
    T: Hash + Send + Sync + Debug + 'static,
{
    pub fn exists(&self) -> bool {
        if let MaybeValue::Empty = self {
            return false;
        } else {
            return true;
        }
    }

    pub fn observe(&self, ctx: &mut EvalContext) -> Arc<Option<T>> {
        match self {
            MaybeValue::Empty => unimplemented!("Empty value cannot be observed"),
            MaybeValue::Const(v) => v.clone(),
            MaybeValue::Var(v) => v.observe(ctx),
            MaybeValue::Computed(v) => v.observe(ctx),
        }
    }

    pub fn once(&self) -> Arc<Option<T>> {
        match self {
            MaybeValue::Empty => unimplemented!("Empty value cannot be observed"),
            MaybeValue::Const(v) => v.clone(),
            MaybeValue::Var(v) => v.once(),
            MaybeValue::Computed(v) => v.once(),
        }
    }

    pub fn as_var(&self) -> Option<&Var<Option<T>>> {
        match self {
            MaybeValue::Empty => None,
            MaybeValue::Const(_) => None,
            MaybeValue::Var(v) => Some(v),
            MaybeValue::Computed(_) => None,
        }
    }

    pub fn as_const(&self) -> Option<&Arc<Option<T>>> {
        match self {
            MaybeValue::Empty => None,
            MaybeValue::Const(a) => Some(a),
            MaybeValue::Var(_v) => None,
            MaybeValue::Computed(_) => None,
        }
    }

    pub fn as_computed(&self) -> Option<&Computed<Option<T>>> {
        match self {
            MaybeValue::Empty => None,
            MaybeValue::Const(_) => None,
            MaybeValue::Var(_) => None,
            MaybeValue::Computed(c) => Some(c),
        }
    }

    pub fn map<R, F>(&self, handler: F) -> MaybeValue<R>
    where
        F: Fn(&mut EvalContext, &T) -> R + 'static,
        R: Hash + Send + Sync + 'static,
    {
        match self {
            MaybeValue::Empty => MaybeValue::Empty,
            MaybeValue::Const(v) => MaybeValue::Const(Arc::new(
                (**v)
                    .as_ref()
                    .map(|v| handler(&mut EvalContext::empty(), &v)),
            )),
            MaybeValue::Var(v) => {
                let v = v.clone();
                MaybeValue::Computed(Computed::new(move |ctx| {
                    let value = v.observe(ctx);
                    (*value).as_ref().map(|v| handler(ctx, v))
                }))
            }
            MaybeValue::Computed(v) => {
                let v = v.clone();
                MaybeValue::Computed(Computed::new(move |ctx| {
                    let value = v.observe(ctx);
                    (*value).as_ref().map(|v| handler(ctx, &v))
                }))
            }
        }
    }
}
