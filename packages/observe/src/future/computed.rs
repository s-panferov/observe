use std::{
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, RwLock, Weak},
    task::Poll,
};

use futures::future::AbortHandle;

use super::factory::FutureFactory;
use super::runtime::FutureRuntime;

use crate::{
    observable::Ref, tracker::Evaluation, EvalContext, MutObservable, Observable, Tracker,
    Transaction, Value, Var,
};

pub struct ComputedFuture<T, Rt>
where
    T: Hash + 'static,
    Rt: FutureRuntime + 'static,
{
    body: Arc<ComputedFutureBody<T, Rt>>,
}

impl<T, Rt> Clone for ComputedFuture<T, Rt>
where
    T: Hash + 'static,
    Rt: FutureRuntime + 'static,
{
    fn clone(&self) -> Self {
        ComputedFuture {
            body: self.body.clone(),
        }
    }
}

impl<T, Rt> Observable<Poll<T>> for ComputedFuture<T, Rt>
where
    T: Hash + 'static,
    Rt: FutureRuntime + 'static,
{
    fn access(&self, ctx: Option<&EvalContext>) -> Ref<Poll<T>> {
        self.body.access(ctx)
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    where
        Poll<T>: Debug,
    {
        self.body.debug(f)
    }
}

impl<T, Rt> Deref for ComputedFuture<T, Rt>
where
    T: Hash + 'static,
    Rt: FutureRuntime + 'static,
{
    type Target = Arc<ComputedFutureBody<T, Rt>>;
    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

pub struct ComputedFutureBody<T, Rt>
where
    T: Hash + 'static,
    Rt: FutureRuntime + 'static,
{
    state: RwLock<ComputedFutureState<T, Rt>>,
    tracker: Tracker,
    current: Var<Poll<T>>,
    _rt: PhantomData<Rt>,
}

pub struct ComputedFutureState<T, Rt>
where
    T: Hash + 'static,
    Rt: FutureRuntime + 'static,
{
    observed: bool,
    reference: Option<Weak<ComputedFutureBody<T, Rt>>>,
    factory: Option<Box<dyn FutureFactory<T>>>,
    abort: Option<AbortHandle>,
}

impl<T, Rt> Deref for ComputedFutureBody<T, Rt>
where
    T: Hash + 'static,
    Rt: FutureRuntime + 'static,
{
    type Target = Var<Poll<T>>;
    fn deref(&self) -> &Self::Target {
        &self.current
    }
}

impl<T, Rt> ComputedFuture<T, Rt>
where
    T: Hash + 'static,
    Rt: FutureRuntime + 'static,
{
    pub fn new(factory: impl FutureFactory<T> + 'static) -> Self {
        Self::create(Some(Box::new(factory)))
    }

    pub fn uninit() -> Self {
        Self::create(None)
    }

    fn create(factory: Option<Box<dyn FutureFactory<T>>>) -> Self {
        let body = Arc::new(ComputedFutureBody {
            current: Var::new(Poll::Pending),
            tracker: Tracker::new(),
            state: RwLock::new(ComputedFutureState {
                observed: false,
                reference: None,
                factory,
                abort: None,
            }),
            _rt: PhantomData,
        });

        body.state.write().unwrap().reference = Some(Arc::downgrade(&body));
        Tracker::set_eval(&body.tracker, body.clone());

        let future = ComputedFuture { body };

        future
    }
}

impl<T, Rt> ComputedFutureBody<T, Rt>
where
    T: Hash + 'static,
    Rt: FutureRuntime + 'static,
{
    pub fn set_factory(&self, factory: impl FutureFactory<T> + 'static) {
        self.state.write().unwrap().factory = Some(Box::new(factory))
    }
}

impl<T, Rt> Observable<Poll<T>> for ComputedFutureBody<T, Rt>
where
    T: Hash + 'static,
    Rt: FutureRuntime + 'static,
{
    fn access(&self, ctx: Option<&EvalContext>) -> Ref<Poll<T>> {
        if let Some(ctx) = ctx {
            self.tracker.access(Some(ctx));
            self.current.access(Some(ctx))
        } else {
            self.tracker.access(None);
            self.current.access(None)
        }
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    where
        Poll<T>: Debug,
    {
        self.current.debug(f)
    }
}

impl<T, Rt> MutObservable<Poll<T>> for ComputedFutureBody<T, Rt>
where
    T: Hash + 'static,
    Rt: FutureRuntime + 'static,
{
    fn modify<F>(&self, tx: Option<&mut Transaction>, value: F)
    where
        F: FnOnce(&mut Poll<T>),
    {
        self.current.modify(tx, value)
    }
}

impl<T, Rt> Evaluation for ComputedFutureBody<T, Rt>
where
    T: Hash + 'static,
    Rt: FutureRuntime + 'static,
{
    fn on_become_observed(&self) {
        self.state.write().unwrap().observed = true;
    }

    fn on_become_unobserved(&self) {
        self.state.write().unwrap().observed = false;
    }

    fn eval(&self, ctx: &EvalContext) -> u64 {
        let mut state = self.state.write().unwrap();

        let future = state.factory.as_mut().and_then(|f| f.eval(ctx));
        if future.is_none() {
            // None means we don't want to update future this time
            return self.current.hash();
        }

        if state.abort.is_some() {
            let abort = state.abort.take();
            abort.unwrap().abort();
        }

        let future = future.unwrap();

        self.set_now(Poll::Pending);

        let (future, abort) = futures::future::abortable(future);
        state.abort = Some(abort);

        let this = state.reference.clone().expect("Ref should be initialized");

        Rt::spawn(async move {
            let value = future.await;
            if let Err(_) = value {
                return; // Aborted
            }
            if let Some(this) = this.upgrade() {
                let payload = Poll::Ready(value.unwrap());
                this.set_now(payload);
            }
        });

        self.current.hash()
    }
}

impl<T, Rt> From<ComputedFuture<T, Rt>> for Value<Poll<T>>
where
    T: Hash + 'static,
    Rt: FutureRuntime + Send + Sync + 'static,
{
    fn from(value: ComputedFuture<T, Rt>) -> Self {
        Value { value: value.body }
    }
}
