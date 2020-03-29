use std::future::Future;
use std::{hash::Hash, marker::PhantomData, mem, pin::Pin, rc::Rc};

use crate::eval::{AnyValue, Evaluation};
use crate::{payload::Payload, variable::Variable, EvalContext, WeakTracker};

use futures::future::AbortHandle;

pub type BoxedFuture<T> = Box<dyn Future<Output = T>>;

pub trait FutureProvider<T> {
    fn eval(&mut self, ctx: &mut EvalContext) -> Option<BoxedFuture<T>>;
}

pub trait FutureSpawn {
    fn spawn_local<F>(future: F)
    where
        F: Future<Output = ()> + 'static;
}

impl<F, T> FutureProvider<T> for F
where
    T: Hash + 'static,
    F: FnMut(&mut EvalContext) -> BoxedFuture<T>,
{
    fn eval(&mut self, ctx: &mut EvalContext) -> Option<BoxedFuture<T>> {
        Some((self)(ctx))
    }
}

pub struct FutureEff<V, R, Expr, Eff>
where
    V: Eq + 'static,
    R: Hash + 'static,
    Expr: FnMut(&mut EvalContext) -> V,
    Eff: FnMut(&mut V) -> BoxedFuture<R>,
{
    expr: Expr,
    eff: Eff,
    cached: Option<V>,
}

impl<V, R, Expr, Eff> FutureEff<V, R, Expr, Eff>
where
    V: Eq + 'static,
    R: Hash + 'static,
    Expr: FnMut(&mut EvalContext) -> V,
    Eff: FnMut(&mut V) -> BoxedFuture<R>,
{
    pub fn new(expr: Expr, eff: Eff) -> Self {
        FutureEff {
            expr,
            eff,
            cached: None,
        }
    }
}

impl<V, R, Expr, Eff> FutureProvider<R> for FutureEff<V, R, Expr, Eff>
where
    V: Eq + 'static,
    R: Hash + 'static,
    Expr: FnMut(&mut EvalContext) -> V,
    Eff: FnMut(&mut V) -> BoxedFuture<R>,
{
    fn eval(&mut self, ctx: &mut EvalContext) -> Option<BoxedFuture<R>> {
        let mut value = (self.expr)(ctx);
        if self.cached.as_ref() != Some(&value) {
            let res = (self.eff)(&mut value);
            let _old = mem::replace(&mut self.cached, Some(value));
            return Some(res);
        }
        None
    }
}

pub struct FutureBody<T, S>
where
    T: Hash,
    S: FutureSpawn,
{
    tracker: WeakTracker,
    observed: bool,
    abort: Option<AbortHandle>,
    hash: u64,
    payload: Rc<Payload<T>>,
    handler: Box<dyn FutureProvider<T>>,
    _spawn: PhantomData<S>,
}

impl<T, S> FutureBody<T, S>
where
    T: Hash,
    S: FutureSpawn,
{
    pub fn new(tracker: WeakTracker, handler: Box<dyn FutureProvider<T>>) -> Self {
        FutureBody {
            tracker,
            handler,
            abort: None,
            hash: 0,
            payload: Rc::new(Payload::Nothing),
            observed: false,
            _spawn: PhantomData,
        }
    }

    fn update(&mut self, value: Payload<T>) {
        self.payload = Rc::new(value);
    }
}

impl<T, S> Evaluation for FutureBody<T, S>
where
    T: Hash + 'static,
    S: FutureSpawn,
{
    fn evaluate(&mut self, ctx: &mut EvalContext) -> u64 {
        let future = self.handler.eval(ctx);
        if future.is_none() {
            return self.hash;
        }

        let future = future.unwrap();
        self.update(Payload::Loading);

        let tracker = self.tracker.clone();
        let future = <Pin<Box<_>>>::from(future);
        if let Some(handle) = &self.abort {
            handle.abort()
        }

        let (future, abort) = futures::future::abortable(future);
        self.abort = Some(abort);

        <S as FutureSpawn>::spawn_local(async move {
            let value = future.await;
            if let Err(_) = value {
                return; // Aborted
            }
            if let Some(tracker) = tracker.upgrade() {
                let payload = Rc::new(Payload::Value(value.unwrap()));
                tracker.set(None, payload);
            }
        });

        1 // For Loading
    }

    fn get(&self) -> AnyValue {
        self.payload.clone()
    }

    fn set(&mut self, next: AnyValue) -> u64 {
        let next = next.downcast::<Payload<T>>().unwrap();
        let next_hash = match &*next {
            Payload::Nothing => 0,
            Payload::Loading => 1,
            Payload::Value(v) => Variable::hash(&v),
        };
        self.payload = next;
        next_hash
    }

    fn on_become_observed(&mut self) {
        self.observed = false;
    }

    fn on_become_unobserved(&mut self) {
        self.observed = false;
    }
}
