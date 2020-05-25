use std::future::Future;
use std::{hash::Hash, pin::Pin};

use crate::EvalContext;

pub trait FutureFactory<T> {
    fn eval(&mut self, ctx: &mut EvalContext) -> Option<Pin<Box<dyn Future<Output = T>>>>;
}

impl<T, H, F> FutureFactory<T> for H
where
    T: Hash + 'static,
    H: FnMut(&mut EvalContext) -> F,
    F: Future<Output = T> + 'static,
{
    fn eval(&mut self, ctx: &mut EvalContext) -> Option<Pin<Box<dyn Future<Output = T>>>> {
        Some(Box::pin((self)(ctx)))
    }
}
