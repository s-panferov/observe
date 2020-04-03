use crate::context::EvalContext;
use crate::{
    eval::Evaluation,
    tracker::TrackerImpl,
    types::{Apply, Type},
    variable::default_hash,
    Local, Shared,
};

use std::{any::Any, hash::Hash, mem, rc::Rc, sync::Arc};

pub struct Effect<T, R, Expr, Eff, Impl>
where
    T: Eq,
    R: Hash + 'static,
    Expr: Fn(&mut EvalContext<Impl>) -> T,
    Eff: Fn(&mut T) -> R,
    Impl::Ptr: Apply<R>,
    Impl: TrackerImpl,
{
    expr: Expr,
    eff: Eff,
    hash: u64,
    cached: Option<T>,
    current: Option<Type<Impl::Ptr, R>>,
}

impl<T, R, Expr, Eff, Impl> Effect<T, R, Expr, Eff, Impl>
where
    T: Eq,
    R: Hash + 'static,
    Expr: Fn(&mut EvalContext<Impl>) -> T,
    Eff: Fn(&mut T) -> R,
    Impl::Ptr: Apply<R>,
    Impl: TrackerImpl,
{
    pub fn new(expr: Expr, eff: Eff) -> Self {
        Effect {
            expr,
            eff,
            hash: 0,
            cached: None,
            current: None,
        }
    }

    fn evaluate(&mut self, context: &mut EvalContext<Impl>) -> u64 {
        let mut value: T = (self.expr)(context);
        if self.cached.as_ref() != Some(&value) {
            let res = (self.eff)(&mut value);
            let _old = mem::replace(&mut self.cached, Some(value));
            let hash = default_hash(&res);
            if self.hash != hash {
                self.hash = hash;
                let _old = mem::replace(&mut self.current, Some(Impl::ptr_wrap(res)));
            }
        }
        self.hash
    }
}

impl<T, R, Expr, Eff> Evaluation<Local> for Effect<T, R, Expr, Eff, Local>
where
    T: Eq + 'static,
    R: Hash + 'static,
    Expr: Fn(&mut EvalContext<Local>) -> T + 'static,
    Eff: Fn(&mut T) -> R + 'static,
{
    fn evaluate(&mut self, ctx: &mut EvalContext<Local>) -> u64 {
        self.evaluate(ctx)
    }

    fn get(&self) -> Rc<dyn Any> {
        self.current.as_ref().unwrap().clone()
    }
}

impl<T, R, Expr, Eff> Evaluation<Shared> for Effect<T, R, Expr, Eff, Shared>
where
    T: Eq + Send + Sync + 'static,
    R: Hash + Send + Sync + 'static,
    Expr: Fn(&mut EvalContext<Shared>) -> T + 'static,
    Eff: Fn(&mut T) -> R + 'static,
{
    fn evaluate(&mut self, ctx: &mut EvalContext<Shared>) -> u64 {
        self.evaluate(ctx)
    }

    fn get(&self) -> Arc<dyn Any + Send + Sync> {
        self.current.as_ref().unwrap().clone()
    }
}
