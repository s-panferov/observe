use std::{hash::Hash, ops::Deref, rc::Rc};

use crate::context::EvalContext;
use crate::{
    eval::{AnyValue, Evaluation},
    variable::Variable,
    Tracker, Value,
};

pub struct Computed<T: Hash> {
    value: Value<T>,
}

impl<T: Hash> Deref for Computed<T> {
    type Target = Value<T>;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Hash> From<Computed<T>> for Value<T> {
    fn from(from: Computed<T>) -> Value<T> {
        from.value.clone()
    }
}

impl<T: Hash + 'static> Default for Computed<T> {
    fn default() -> Self {
        Computed::empty()
    }
}

impl<T: Hash + 'static> Computed<T> {
    pub fn new<F>(handler: F) -> Computed<T>
    where
        F: Fn(&mut EvalContext) -> T + 'static,
    {
        let computed = Computed::empty();
        computed.set_handler(handler);
        computed
    }

    pub fn empty() -> Computed<T> {
        let tracker = Tracker::new("Computed".to_owned());
        Computed {
            value: Value::Dynamic { tracker },
        }
    }

    pub fn set_handler<F>(&self, handler: F)
    where
        F: Fn(&mut EvalContext) -> T + 'static,
    {
        self.set_computation(Box::new(ComputedEngine::new(None, handler, false)));
    }
}

pub struct ComputedEngine<T: Hash, F: Fn(&mut EvalContext) -> T> {
    is_observer: bool,
    current: Option<Rc<T>>,
    func: F,
}

impl<T: Hash, F: Fn(&mut EvalContext) -> T> ComputedEngine<T, F> {
    pub fn new(value: Option<T>, func: F, is_observer: bool) -> Self {
        ComputedEngine {
            is_observer,
            current: value.map(Rc::new),
            func,
        }
    }
}

impl<T: Hash + 'static, F: Fn(&mut EvalContext) -> T + 'static> Evaluation
    for ComputedEngine<T, F>
{
    fn evaluate(&mut self, ctx: &mut EvalContext) -> u64 {
        let next = (self.func)(ctx);
        let hash = Variable::hash(&next);
        self.current.replace(Rc::new(next));
        hash
    }

    fn is_observer(&self) -> bool {
        self.is_observer
    }

    /// Get the current value
    ///
    /// Returns "next" value inside the transaction or
    /// the "current" value outsize.
    fn get(&self) -> AnyValue {
        self.current.as_ref().unwrap().clone()
    }
}
