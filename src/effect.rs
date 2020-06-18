use std::{hash::Hash, sync::RwLock};

use crate::computed::ComputedFactory;
use crate::context::EvalContext;

pub struct Effect<T, R>
where
    T: Eq + 'static,
    R: Clone + Hash + 'static,
{
    expr: Box<dyn Fn(&EvalContext) -> T>,
    eff: Box<dyn Fn(&mut T) -> R>,
    cached: RwLock<Option<T>>,
}

impl<T, R> Effect<T, R>
where
    T: Eq + 'static,
    R: Clone + Hash + 'static,
{
    pub fn new(expr: Box<dyn Fn(&EvalContext) -> T>, eff: Box<dyn Fn(&mut T) -> R>) -> Self {
        Effect {
            expr,
            eff,
            cached: RwLock::new(None),
        }
    }
}

impl<T, R> ComputedFactory<R> for Effect<T, R>
where
    T: Eq + 'static,
    R: Clone + Hash + 'static,
{
    fn eval(&mut self, ctx: &EvalContext) -> Option<R> {
        let mut value: T = (self.expr)(ctx);
        if self.cached.read().unwrap().as_ref() != Some(&value) {
            let res = (self.eff)(&mut value);
            self.cached.write().unwrap().replace(value);
            Some(res)
        } else {
            None
        }
    }
}
