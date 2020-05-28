use std::hash::Hash;
use std::{fmt::Debug, ops::Deref, sync::Arc};

use parking_lot::{RwLock, RwLockReadGuard};

use crate::context::EvalContext;
use crate::hashed::Hashed;
use crate::observable::{MutObservable, Observable, Ref};
use crate::{
    tracker::{Evaluation, Invalidate, Tracker},
    Transaction, Value,
};

pub struct Var<T>
where
    T: Hash + 'static,
{
    body: Arc<VarBody<T>>,
}

impl<T> std::fmt::Debug for Var<T>
where
    T: Hash + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.body.fmt(f)
    }
}

impl<T> Clone for Var<T>
where
    T: Hash + 'static,
{
    fn clone(&self) -> Self {
        Var {
            body: self.body.clone(),
        }
    }
}

impl<T> Observable<T> for Var<T>
where
    T: Hash + 'static,
{
    fn access(&self, ctx: Option<&mut EvalContext>) -> Ref<T> {
        self.body.access(ctx)
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    where
        T: Debug,
    {
        self.body.fmt(f)
    }
}

impl<T> Deref for Var<T>
where
    T: Hash + 'static,
{
    type Target = Arc<VarBody<T>>;
    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

pub struct VarBody<T> {
    hashed: RwLock<Hashed<T>>,
    tracker: Tracker,
}

impl<T> Var<T>
where
    T: Hash + 'static,
{
    pub fn new(value: T) -> Self {
        let tracker = Tracker::new();
        let body = Arc::new(VarBody {
            hashed: RwLock::new(Hashed::new(value)),
            tracker: tracker.clone(),
        });

        Tracker::set_eval(&tracker, body.clone());
        Var { body }
    }
}

impl<T> Deref for VarBody<T> {
    type Target = Tracker;
    fn deref(&self) -> &Self::Target {
        &self.tracker
    }
}

impl<T> Evaluation for VarBody<T> {
    fn eval(&self, _ctx: &mut EvalContext) -> u64 {
        self.hashed.read().hash
    }
}

impl<T> Observable<T> for VarBody<T> {
    fn access(&self, ctx: Option<&mut EvalContext>) -> Ref<T> {
        self.tracker.access(ctx);
        Ref::Lock(RwLockReadGuard::map(self.hashed.read(), |v| &v.value))
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    where
        T: Debug,
    {
        write!(f, "Var[{:?}]", self.hashed.read())
    }
}

impl<T> MutObservable<T> for VarBody<T>
where
    T: Hash,
{
    fn modify<F>(&self, tx: Option<&mut Transaction>, mapper: F)
    where
        F: FnOnce(&mut T),
    {
        let mut hashed = self.hashed.write();
        mapper(&mut hashed.value);
        let new_hash = fxhash::hash64(&hashed.value);
        if hashed.hash != new_hash {
            hashed.hash = new_hash;
            std::mem::drop(hashed);
            self.tracker.change(new_hash, Invalidate::OnlyDeps, tx);
        }
    }
}

impl<T> From<Var<T>> for Value<T>
where
    T: Hash + 'static,
{
    fn from(value: Var<T>) -> Self {
        Value { value: value.body }
    }
}
