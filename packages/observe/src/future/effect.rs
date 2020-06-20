use std::future::Future;
use std::{hash::Hash, mem, pin::Pin};

use super::FutureFactory;
use crate::EvalContext;

pub struct FutureEffect<V, R, F, Expr, Eff>
where
    V: Eq + 'static,
    R: Hash + 'static,
    F: Future<Output = R>,
    Expr: FnMut(&EvalContext) -> V,
    Eff: FnMut(&mut V) -> F,
{
    expr: Expr,
    eff: Eff,
    cached: Option<V>,
}

impl<V, R, F, Expr, Eff> FutureEffect<V, R, F, Expr, Eff>
where
    V: Eq + 'static,
    R: Hash + 'static,
    F: Future<Output = R> + 'static,
    Expr: FnMut(&EvalContext) -> V,
    Eff: FnMut(&mut V) -> F,
{
    pub fn new(expr: Expr, eff: Eff) -> Self {
        FutureEffect {
            expr,
            eff,
            cached: None,
        }
    }
}

impl<V, R, F, Expr, Eff> FutureFactory<R> for FutureEffect<V, R, F, Expr, Eff>
where
    V: Eq + 'static,
    R: Hash + 'static,
    F: Future<Output = R> + 'static,
    Expr: FnMut(&EvalContext) -> V,
    Eff: FnMut(&mut V) -> F,
{
    fn eval(&mut self, ctx: &EvalContext) -> Option<Pin<Box<dyn Future<Output = R>>>> {
        let mut value = (self.expr)(ctx);
        if self.cached.as_ref() != Some(&value) {
            let res = (self.eff)(&mut value);
            let _old = mem::replace(&mut self.cached, Some(value));
            return Some(Box::pin(res));
        }
        None
    }
}
