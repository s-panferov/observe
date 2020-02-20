use crate::context::{EvalContext, TrackerBody};
use crate::tracker::Tracker;

use std::fmt;
use std::mem;

pub struct Reaction {
    tracker: Tracker,
}

impl fmt::Debug for Reaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Reaction[tracker: {:?}]", self.tracker)
    }
}

impl Reaction {
    fn new<E: TrackerBody + 'static>(eval: E, name: Option<String>) -> Self {
        let tracker = Tracker::new(name.unwrap_or("Reaction".to_string()));
        let mut tr = tracker.get_mut();
        tr.set_computation(eval);
        tr.set_is_observer();
        std::mem::drop(tr);

        Reaction { tracker }
    }

    pub fn run(&self) {
        self.tracker.get_mut().update();
    }
}

pub struct Autorun<F: Fn(&mut EvalContext)> {
    handler: F,
}

impl<F: Fn(&mut EvalContext)> TrackerBody for Autorun<F> {
    fn evaluate(&mut self, context: &mut EvalContext) -> u64 {
        (self.handler)(context);
        return 0;
    }
}

pub struct Effect<T: Eq, Expr: Fn(&mut EvalContext) -> T, Eff: Fn(&mut T)> {
    expr: Expr,
    eff: Eff,
    cached: Option<T>,
}

impl<T: Eq, Expr: Fn(&mut EvalContext) -> T, Eff: Fn(&mut T)> TrackerBody for Effect<T, Expr, Eff> {
    fn evaluate(&mut self, context: &mut EvalContext) -> u64 {
        let mut value = (self.expr)(context);
        if self.cached.as_ref() != Some(&value) {
            (self.eff)(&mut value);
            let _old = mem::replace(&mut self.cached, Some(value));
        }
        0
    }
}

pub fn autorun<F: Fn(&mut EvalContext) + 'static>(func: F, name: Option<String>) -> Reaction {
    Reaction::new(Autorun { handler: func }, name)
}

pub fn reaction<
    T: Eq + 'static,
    Expr: Fn(&mut EvalContext) -> T + 'static,
    Eff: Fn(&mut T) + 'static,
>(
    expr: Expr,
    eff: Eff,
    name: Option<String>,
) -> Reaction {
    Reaction::new(
        Effect {
            expr,
            eff,
            cached: None,
        },
        name,
    )
}
