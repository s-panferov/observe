use std::future::Future;
use std::task::Poll;

use std::{any::Any, hash::Hash, marker::PhantomData, rc::Rc, sync::Arc};

use crate::eval::{Evaluation, Invalidate};
use crate::{
    tracker::TrackerImpl,
    types::{Apply, Type},
    value::Value,
    variable::default_hash,
    EvalContext, Local, Shared, WeakTracker,
};

use futures::future::{AbortHandle, Abortable};
use tracing::{event, span, Level};

mod eff;
mod provider;
mod spawn;

#[cfg(test)]
mod tests;

pub use eff::*;
pub use provider::*;
pub use spawn::*;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "tokio-0")]
pub type TokioComputedFuture<T, P, Impl> = ComputedFuture<T, TokioRuntime, P, Impl>;

#[cfg(feature = "wasm-bindgen-futures-0")]
pub type WasmComputedFuture<T, P, Impl> = ComputedFuture<T, WasmRuntime, P, Impl>;

pub struct ComputedFuture<T, S, P, Impl>
where
    T: Hash,
    S: FutureRuntime,
    P: FutureProvider<T, Impl>,
    Impl: TrackerImpl,
    Impl::Ptr: Apply<Poll<T>>,
{
    tracker: Option<WeakTracker<Impl>>,
    observed: bool,
    abort: Option<AbortHandle>,
    hash: u64,
    payload: Type<Impl::Ptr, Poll<T>>,
    provider: P,
    _spawn: PhantomData<S>,
}

impl<T, S, P, Impl> ComputedFuture<T, S, P, Impl>
where
    T: Hash,
    S: FutureRuntime,
    P: FutureProvider<T, Impl>,
    Impl: TrackerImpl,
    Impl::Ptr: Apply<Poll<T>>,
{
    pub fn new(provider: P) -> Self {
        ComputedFuture {
            tracker: None,
            abort: None,
            hash: 0,
            payload: Impl::ptr_wrap(Poll::Pending),
            observed: false,
            provider,
            _spawn: PhantomData,
        }
    }

    pub fn set_tracker(&mut self, tracker: WeakTracker<Impl>) {
        self.tracker = Some(tracker);
    }

    fn update(&mut self, value: Poll<T>) {
        self.payload = Impl::ptr_wrap(value);
    }

    fn evaluate<H>(&mut self, ctx: &mut EvalContext<Impl>, handler: H) -> u64
    where
        H: FnOnce(Abortable<P::Output>),
    {
        let span = span!(Level::TRACE, "ComputedFuture.evaluate");
        let _enter = span.enter();

        let future = self.provider.eval(ctx);
        if future.is_none() {
            return self.hash;
        }

        let future = future.unwrap();

        event!(Level::INFO, "Future -> Loading");
        self.update(Poll::Pending);

        if let Some(handle) = &self.abort {
            event!(Level::INFO, "Aborting the old future");
            handle.abort()
        }

        let (future, abort) = futures::future::abortable(future);
        self.abort = Some(abort);

        handler(future);

        1 // For Loading
    }
}

impl<T, S, P> Evaluation<Local> for ComputedFuture<T, S, P, Local>
where
    T: Hash + 'static,
    P: FutureProvider<T, Local>,
    S: FutureRuntime,
{
    fn evaluate(&mut self, ctx: &mut EvalContext<Local>) -> u64 {
        let tracker = self.tracker.clone();
        self.evaluate(ctx, move |future| {
            <S as FutureRuntime>::spawn_local(async move {
                let value = future.await;
                if let Err(_) = value {
                    return; // Aborted
                }
                if let Some(tracker) = tracker.expect("Tracker should be initialized").upgrade() {
                    let payload = Rc::new(Poll::Ready(value.unwrap()));
                    tracker.set(None, payload);
                }
            });
        })
    }

    fn get(&self) -> Rc<dyn Any> {
        self.payload.clone()
    }

    fn set(&mut self, next: Rc<dyn Any>) -> (u64, Invalidate) {
        let next = next.downcast::<Poll<T>>().unwrap();
        let next_hash = default_hash(&next);
        self.payload = next;
        (next_hash, Invalidate::OnlyDeps)
    }

    fn on_become_observed(&mut self) {
        self.observed = false;
    }

    fn on_become_unobserved(&mut self) {
        self.observed = false;
    }
}

impl<T, S, P> Evaluation<Shared> for ComputedFuture<T, S, P, Shared>
where
    T: Hash + Send + Sync + 'static,
    P: FutureProvider<T, Shared>,
    S: FutureRuntime,
    P::Output: Future<Output = T> + Send + 'static,
{
    fn evaluate(&mut self, ctx: &mut EvalContext<Shared>) -> u64 {
        let tracker = self.tracker.clone();
        self.evaluate(ctx, move |future| {
            event!(Level::INFO, "Spawning the future");
            <S as FutureRuntime>::spawn(async move {
                let value = future.await;
                if let Err(_) = value {
                    return; // Aborted
                }
                if let Some(tracker) = tracker.expect("Tracker should be initialized").upgrade() {
                    event!(Level::INFO, "Notify tracker about new future payload");
                    let payload = Arc::new(Poll::Ready(value.unwrap()));
                    tracker.set(None, payload);
                } else {
                    event!(Level::WARN, "Tracker was dropped before future completion");
                }
            });
        })
    }

    fn get(&self) -> Arc<dyn Any + Send + Sync> {
        self.payload.clone()
    }

    fn set(&mut self, next: Arc<dyn Any + Send + Sync>) -> (u64, Invalidate) {
        let next = next.downcast::<Poll<T>>().unwrap();
        event!(Level::INFO, "New future payload");
        let next_hash = default_hash(&next);
        event!(Level::INFO, next_hash, "Next hash");
        self.payload = next;
        (next_hash, Invalidate::OnlyDeps)
    }

    fn on_become_observed(&mut self) {
        self.observed = false;
    }

    fn on_become_unobserved(&mut self) {
        self.observed = false;
    }
}

impl<T, S, P> From<ComputedFuture<T, S, P, Shared>> for Value<Poll<T>, Shared>
where
    T: Hash + Send + Sync + 'static,
    P: FutureProvider<T, Shared> + Send + Sync + 'static,
    S: FutureRuntime + Send + Sync + 'static,
    P::Output: Future<Output = T> + Send + 'static,
{
    fn from(mut from: ComputedFuture<T, S, P, Shared>) -> Value<Poll<T>, Shared> {
        let value = Value::<Poll<T>, Shared>::uninit();
        from.tracker = Some(value.tracker().unwrap().weak());
        value.set_computation(Box::new(from));
        value
    }
}

impl<T, S, P> From<ComputedFuture<T, S, P, Local>> for Value<Poll<T>, Local>
where
    T: Hash + 'static,
    P: FutureProvider<T, Local> + 'static,
    S: FutureRuntime + 'static,
    P::Output: Future<Output = T> + 'static,
{
    fn from(mut from: ComputedFuture<T, S, P, Local>) -> Value<Poll<T>, Local> {
        let value = Value::<Poll<T>, Local>::uninit();
        from.tracker = Some(value.tracker().unwrap().weak());
        value.set_computation(Box::new(from));
        value
    }
}
