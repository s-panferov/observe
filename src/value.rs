use crate::transaction::Transaction;
use core::marker::PhantomData;
use std::fmt::Debug;
use std::{hash::Hash, sync::Arc};

use crate::context::EvalContext;
use crate::{body::ValueBody, tracker::Tracker};

#[derive(Clone)]
pub struct Value<T: Hash> {
    tracker: Tracker,
    _t: PhantomData<T>,
}

impl<T: Hash + Send + Sync + Debug + 'static> Value<T> {
    pub fn new(value: T) -> Value<T> {
        let tracker = Tracker::new("Value".to_string());
        let body = ValueBody::new(value);
        tracker.get_mut().set_computation(body);
        Value {
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

    pub fn set(&self, next: T, tx: &mut Transaction) {
        self.tracker.get_mut().set(Arc::new(next));
        tx.mark_changed(self.tracker.weak().clone());
    }

    pub fn set_now(&self, next: T) {
        self.tracker.get_mut().set(Arc::new(next));
        self.tracker.notify_reactions()
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
        let value = Value::new(10);

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
