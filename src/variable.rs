use crate::computed::Computed;
use crate::transaction::Transaction;
use core::marker::PhantomData;
use std::fmt::Debug;
use std::{hash::Hash, sync::Arc};

use crate::context::EvalContext;
use crate::{body::ValueBody, tracker::Tracker, value::Value};

pub struct Var<T: Hash> {
    tracker: Tracker,
    _t: PhantomData<T>,
}

impl<T: Hash> Clone for Var<T> {
    fn clone(&self) -> Var<T> {
        Var {
            tracker: self.tracker.clone(),
            _t: PhantomData,
        }
    }
}

impl<T: Hash + Send + Sync + Debug + 'static> Var<T> {
    pub fn new(value: T) -> Var<T> {
        let tracker = Tracker::new("Value".to_string());
        let body = ValueBody::new(value);
        tracker.get_mut().set_computation(body);
        Var {
            tracker,
            _t: PhantomData,
        }
    }

    fn _observe(&self, ctx: Option<&mut EvalContext>) -> Arc<T> {
        if ctx.is_some() {
            ctx.unwrap().access(self.tracker.clone());
        }

        if self.tracker.get().should_update() {
            self.tracker.get_mut().update();
        }

        let tracker = self.tracker.get();
        tracker.get().downcast::<T>().unwrap()
    }

    pub fn observe(&self, ctx: &mut EvalContext) -> Arc<T> {
        self._observe(Some(ctx))
    }

    pub fn once(&self) -> Arc<T> {
        self._observe(None)
    }

    pub fn set(&self, next: T, tx: &mut Transaction) {
        self.tracker.get_mut().set(Arc::new(next));
        tx.mark_changed(self.tracker.weak().clone());
    }

    pub fn set_now(&self, next: T) {
        self.tracker.get_mut().set(Arc::new(next));
        self.tracker.notify_reactions()
    }

    pub fn to_value(&self) -> Value<T> {
        Value::Var(Var {
            tracker: self.tracker.clone(),
            _t: PhantomData,
        })
    }

    pub fn map<R, F>(&self, handler: F) -> Computed<R>
    where
        F: Fn(&mut EvalContext, &T) -> R + 'static,
        R: Hash + Send + Sync + 'static,
    {
        let this = Var {
            tracker: self.tracker.clone(),
            _t: PhantomData,
        };

        Computed::new(move |ctx| {
            let value = this.observe(ctx);
            handler(ctx, &value)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Var;
    use crate::context::EvalContext;
    use crate::tracker::{Expired, Freshness, Tracker};

    #[test]
    fn expire_on_set() {
        let tracker = Tracker::new("Tracker".to_owned());
        let value = Var::new(10);

        tracker.get_mut().set_computation({
            let value = value.clone();
            move |ctx: &mut EvalContext| *value.observe(ctx)
        });

        tracker.get_mut().update();

        assert_eq!(*tracker.get().state(), Freshness::UpToDate);

        value.set_now(20);

        assert_eq!(*tracker.get().state(), Freshness::Expired(Expired::Maybe));
    }
}
