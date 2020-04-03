use std::future::Future;
use std::hash::Hash;

use crate::{tracker::TrackerImpl, EvalContext};

pub trait FutureProvider<T, Impl>
where
    Impl: TrackerImpl,
{
    type Output: Future<Output = T> + 'static;
    fn eval(&mut self, ctx: &mut EvalContext<Impl>) -> Option<Self::Output>;
}

impl<T, H, F, Impl> FutureProvider<T, Impl> for H
where
    T: Hash + 'static,
    H: FnMut(&mut EvalContext<Impl>) -> F,
    F: Future<Output = T> + 'static,
    Impl: TrackerImpl,
{
    type Output = F;
    fn eval(&mut self, ctx: &mut EvalContext<Impl>) -> Option<F> {
        Some((self)(ctx))
    }
}
