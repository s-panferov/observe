use std::{any::Any, rc::Rc};

use crate::EvalContext;

pub type AnyValue = Rc<dyn Any + 'static>;

pub trait Evaluation {
    fn evaluate(&mut self, ctx: &mut EvalContext) -> u64;

    fn is_observer(&self) -> bool {
        false
    }

    fn is_scheduled(&self) -> bool {
        false
    }

    fn on_reaction(&mut self) {}
    fn on_become_observed(&mut self) {}
    fn on_become_unobserved(&mut self) {}

    fn get(&self) -> AnyValue {
        unimplemented!()
    }

    fn set(&mut self, _value: AnyValue) -> u64 {
        unimplemented!()
    }
}

impl<F: 'static> Evaluation for F
where
    F: FnMut(&mut EvalContext) -> u64,
{
    fn evaluate(&mut self, ctx: &mut EvalContext) -> u64 {
        self(ctx)
    }
}
