use std::hash::Hash;
use std::{
    ops::Deref,
    sync::{Arc, RwLock},
};

use crate::context::EvalContext;
use crate::hashed::Hashed;
use crate::observable::{MutObservable, Observable};
use crate::{
    tracker::{Evaluation, Invalidate, Tracker},
    Transaction, Value,
};

pub struct Var<T>
where
    T: Clone + Hash + 'static,
{
    body: Arc<VarBody<T>>,
}

impl<T> Clone for Var<T>
where
    T: Clone + Hash + 'static,
{
    fn clone(&self) -> Self {
        Var {
            body: self.body.clone(),
        }
    }
}

impl<T> Observable<T> for Var<T>
where
    T: Clone + Hash + 'static,
{
    fn access(&self, ctx: Option<&mut EvalContext>) -> T {
        self.body.access(ctx)
    }
}

impl<T> Deref for Var<T>
where
    T: Clone + Hash + 'static,
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
    T: Hash + Clone + 'static,
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
        self.hashed.read().unwrap().hash
    }
}

impl<T> Observable<T> for VarBody<T>
where
    T: Clone,
{
    fn access(&self, ctx: Option<&mut EvalContext>) -> T {
        self.tracker.access(ctx);
        self.hashed.read().unwrap().value.clone()
    }
}

impl<T> MutObservable<T> for VarBody<T>
where
    T: Hash + Clone,
{
    fn modify(&self, tx: Option<&mut Transaction>, next: T) {
        let hashed = Hashed::new(next);
        if self.hashed.read().unwrap().hash != hashed.hash {
            let hash = hashed.hash;
            *self.hashed.write().unwrap() = hashed;
            self.tracker.change(hash, Invalidate::OnlyDeps, tx);
        }
    }
}

impl<T> From<Var<T>> for Value<T>
where
    T: Hash + Clone + 'static,
{
    fn from(value: Var<T>) -> Self {
        Value { value: value.body }
    }
}
