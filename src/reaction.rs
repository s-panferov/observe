use crate::context::EvalContext;
use crate::{
    eval::{AnyValue, Evaluation},
    variable::Variable,
};

use std::{hash::Hash, mem, rc::Rc};

pub struct Effect<T: Eq, R: Hash + 'static, Expr: Fn(&mut EvalContext) -> T, Eff: Fn(&mut T) -> R> {
    expr: Expr,
    eff: Eff,
    hash: u64,
    cached: Option<T>,
    current: Option<Rc<R>>,
}

impl<T, R, Expr, Eff> Effect<T, R, Expr, Eff>
where
    T: Eq,
    R: Hash + 'static,
    Expr: Fn(&mut EvalContext) -> T,
    Eff: Fn(&mut T) -> R,
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
}

impl<T, R, Expr, Eff> Evaluation for Effect<T, R, Expr, Eff>
where
    T: Eq + 'static,
    R: Hash + 'static,
    Expr: Fn(&mut EvalContext) -> T + 'static,
    Eff: Fn(&mut T) -> R + 'static,
{
    fn evaluate(&mut self, context: &mut EvalContext) -> u64 {
        let mut value: T = (self.expr)(context);
        if self.cached.as_ref() != Some(&value) {
            let res = (self.eff)(&mut value);
            let _old = mem::replace(&mut self.cached, Some(value));
            let hash = Variable::hash(&res);
            if self.hash != hash {
                self.hash = hash;
                let _old = mem::replace(&mut self.current, Some(Rc::new(res)));
            }
        }
        self.hash
    }

    fn get(&self) -> AnyValue {
        self.current.as_ref().unwrap().clone()
    }

    fn is_observer(&self) -> bool {
        true
    }
}
