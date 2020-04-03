use std::future::Future;
use std::{hash::Hash, marker::PhantomData, mem};

use super::FutureProvider;
use crate::{tracker::TrackerImpl, EvalContext};

pub struct FutureEff<V, R, F, Expr, Eff, Impl>
where
    V: Eq + 'static,
    R: Hash + 'static,
    F: Future<Output = R>,
    Expr: FnMut(&mut EvalContext<Impl>) -> V,
    Eff: FnMut(&mut V) -> F,
    Impl: TrackerImpl,
{
    expr: Expr,
    eff: Eff,
    cached: Option<V>,
    _i: PhantomData<Impl>,
}

impl<V, R, F, Expr, Eff, Impl> FutureEff<V, R, F, Expr, Eff, Impl>
where
    V: Eq + 'static,
    R: Hash + 'static,
    F: Future<Output = R> + 'static,
    Expr: FnMut(&mut EvalContext<Impl>) -> V,
    Eff: FnMut(&mut V) -> F,
    Impl: TrackerImpl,
{
    pub fn new(expr: Expr, eff: Eff) -> Self {
        FutureEff {
            expr,
            eff,
            cached: None,
            _i: PhantomData,
        }
    }
}

impl<V, R, F, Expr, Eff, Impl> FutureProvider<R, Impl> for FutureEff<V, R, F, Expr, Eff, Impl>
where
    V: Eq + 'static,
    R: Hash + 'static,
    F: Future<Output = R> + 'static,
    Expr: FnMut(&mut EvalContext<Impl>) -> V,
    Eff: FnMut(&mut V) -> F,
    Impl: TrackerImpl,
{
    type Output = F;
    fn eval(&mut self, ctx: &mut EvalContext<Impl>) -> Option<F> {
        let mut value = (self.expr)(ctx);
        if self.cached.as_ref() != Some(&value) {
            let res = (self.eff)(&mut value);
            let _old = mem::replace(&mut self.cached, Some(value));
            return Some(res);
        }
        None
    }
}
