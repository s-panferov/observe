use std::fmt::{Debug, Formatter};
use std::{
    hash::Hash,
    rc::{Rc, Weak},
};

use crate::computed::ComputedEngine;
use crate::context::EvalContext;
use crate::transaction::Transaction;
use crate::{
    eval::Evaluation,
    future::{BoxedFuture, FutureBody, FutureEff, FutureProvider},
    payload::Payload,
    reaction::Effect,
    tracker::Tracker,
    variable::Variable,
    WeakTracker,
};

pub enum Value<T> {
    Uninit,
    Const(Rc<T>),
    Dynamic { tracker: Tracker },
}

impl<T> Debug for Value<T>
where
    T: Debug,
{
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<T> Clone for Value<T> {
    fn clone(&self) -> Value<T> {
        match self {
            Value::Uninit => Value::Uninit,
            Value::Const(v) => Value::Const(v.clone()),
            Value::Dynamic { tracker } => Value::Dynamic {
                tracker: tracker.clone(),
            },
        }
    }
}

impl<T> Default for Value<Option<T>> {
    fn default() -> Self {
        Value::Uninit
    }
}

impl<T: Hash + 'static> From<T> for Value<Option<T>> {
    fn from(v: T) -> Self {
        Value::cons(Some(v))
    }
}

impl<T> From<T> for Value<T>
where
    T: Hash + 'static,
{
    fn from(v: T) -> Self {
        Value::cons(v)
    }
}

// impl<T: Hash + 'static> Value<Payload<T>> {
//     pub fn become_fut_autorun(&self, handler: Box<dyn FutureProvider<T>>) {
//         if let Value::Dynamic { tracker } = self {
//             let body = FutureBody::new(tracker.weak(), handler);
//             self.set_computation(Box::new(body));
//         } else {
//             unreachable!()
//         }
//     }
// }

impl<T: Hash + 'static> Value<T> {
    pub fn cons(value: T) -> Value<T> {
        Value::Const(Rc::new(value))
    }

    pub fn var(value: T) -> Value<T> {
        let tracker = Tracker::new(String::from("Variable"));
        tracker.set_computation(Box::new(Variable::new(value)));
        Value::Dynamic { tracker }
    }

    pub fn init() -> Value<T> {
        let tracker = Tracker::new(String::from(""));
        Value::Dynamic { tracker }
    }

    pub fn computed<F>(handler: F) -> Value<T>
    where
        F: Fn(&mut EvalContext) -> T + 'static,
    {
        let tracker = Tracker::new("Computed".to_owned());
        tracker.set_computation(Box::new(ComputedEngine::new(None, handler, false)));
        Value::Dynamic { tracker }
    }

    pub fn become_computed<F>(&self, handler: F)
    where
        F: Fn(&mut EvalContext) -> T + 'static,
    {
        self.set_computation(Box::new(ComputedEngine::new(None, handler, false)));
    }

    pub fn autorun<F>(handler: F) -> Value<T>
    where
        F: Fn(&mut EvalContext) -> T + 'static,
    {
        let tracker = Tracker::new("Autorun".to_owned());
        tracker.set_computation(Box::new(ComputedEngine::new(None, handler, true)));
        Value::Dynamic { tracker }
    }

    pub fn become_autorun<F>(&self, handler: F)
    where
        F: Fn(&mut EvalContext) -> T + 'static,
    {
        self.set_computation(Box::new(ComputedEngine::new(None, handler, true)));
    }

    pub fn effect<V, R, Expr, Eff>(expr: Expr, eff: Eff) -> Value<R>
    where
        V: Eq + 'static,
        R: Hash + 'static,
        Expr: Fn(&mut EvalContext) -> V + 'static,
        Eff: Fn(&mut V) -> R + 'static,
    {
        let tracker = Tracker::new("Effect".to_owned());
        tracker.set_computation(Box::new(Effect::new(expr, eff)));
        Value::Dynamic { tracker }
    }

    // pub fn fut_autorun<F>(handler: F) -> Value<Payload<T>>
    // where
    //     F: Fn(&mut EvalContext) -> BoxedFuture<T> + 'static,
    // {
    //     let tracker = Tracker::new("Future Autorun".to_owned());
    //     let body = FutureBody::new(tracker.weak(), Box::new(handler));
    //     tracker.set_computation(Box::new(body));

    //     Value::Dynamic { tracker }
    // }

    // pub fn fut_effect<V, R, Expr, Eff>(expr: Expr, eff: Eff) -> Value<R>
    // where
    //     V: Eq + 'static,
    //     R: Hash + 'static,
    //     Expr: Fn(&mut EvalContext) -> V + 'static,
    //     Eff: Fn(&mut V) -> BoxedFuture<R> + 'static,
    // {
    //     let tracker = Tracker::new("Effect".to_owned());
    //     let body = FutureBody::new(tracker.weak(), Box::new(FutureEff::new(expr, eff)));
    //     tracker.set_computation(Box::new(body));
    //     Value::Dynamic { tracker }
    // }

    pub fn set_computation(&self, eval: Box<dyn Evaluation + 'static>) {
        match self {
            Value::Const(_) | Value::Uninit => unreachable!(),
            Value::Dynamic { tracker } => {
                tracker.set_computation(eval);
            }
        }
    }

    pub fn set(&self, tx: &mut Transaction, next: T) {
        match self {
            Value::Uninit => unreachable!(),
            Value::Const(_v) => {
                // do nothing
            }
            Value::Dynamic { tracker } => {
                tracker.set(Some(tx), Rc::new(next));
            }
        }
    }

    pub fn set_now(&self, next: T) {
        match self {
            Value::Uninit => unreachable!(),
            Value::Const(_v) => {
                // do nothing
            }
            Value::Dynamic { tracker } => {
                tracker.set(None, Rc::new(next));
            }
        }
    }

    pub fn map<R, F>(&self, handler: F) -> Value<R>
    where
        F: Fn(&mut EvalContext, &T) -> R + 'static,
        R: Hash + Debug + 'static,
    {
        match self {
            Value::Uninit => Value::Uninit,
            Value::Const(v) => Value::Const(Rc::new(handler(&mut EvalContext::empty(), &v))),
            Value::Dynamic { .. } => {
                let this = self.clone();
                Value::computed(move |ctx| {
                    let value = this.observe(ctx);
                    handler(ctx, &value)
                })
            }
        }
    }

    pub fn update(&self) {
        match self {
            Value::Uninit => unreachable!(),
            Value::Const(_v) => {}
            Value::Dynamic { tracker } => tracker.update(),
        };
    }

    pub fn observe(&self, ctx: &mut EvalContext) -> Rc<T> {
        self.get(Some(ctx))
    }

    pub fn once(&self) -> Rc<T> {
        self.get(None)
    }

    fn get(&self, ctx: Option<&mut EvalContext>) -> Rc<T> {
        match self {
            Value::Uninit => unreachable!(),
            Value::Const(v) => v.clone(),
            Value::Dynamic { tracker } => tracker.get(ctx).downcast::<T>().unwrap(),
        }
    }

    pub fn is_uninit(&self) -> bool {
        if let Value::Uninit = self {
            return true;
        } else {
            return false;
        }
    }

    pub fn weak(&self) -> WeakValue<T> {
        match self {
            Value::Uninit => WeakValue::Uninit,
            Value::Const(c) => WeakValue::Const(Rc::downgrade(c)),
            Value::Dynamic { tracker } => WeakValue::Dynamic {
                tracker: tracker.weak(),
            },
        }
    }
}

#[derive(Debug)]
pub enum WeakValue<T> {
    Uninit,
    Const(Weak<T>),
    Dynamic { tracker: WeakTracker },
}

impl<T> WeakValue<T> {
    pub fn upgrade(&self) -> Option<Value<T>> {
        match self {
            WeakValue::Uninit => Some(Value::Uninit),
            WeakValue::Const(c) => c.upgrade().map(|v| Value::Const(v)),
            WeakValue::Dynamic { tracker } => {
                tracker.upgrade().map(|tracker| Value::Dynamic { tracker })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Value;
    use crate::context::EvalContext;
    use crate::tracker::{Expired, Freshness, Tracker};

    #[test]
    fn expire_on_set() {
        let tracker = Tracker::new("Tracker".to_owned());
        let value = Value::var(10);

        tracker.set_computation({
            let value = value.clone();
            Box::new(move |ctx: &mut EvalContext| *value.observe(ctx))
        });

        tracker.update();

        assert_eq!(tracker.state(), Freshness::UpToDate);

        value.set_now(20);

        assert_eq!(tracker.state(), Freshness::Expired(Expired::Maybe));
    }
}
