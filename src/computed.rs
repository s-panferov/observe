use crate::value::Value;
use std::hash::Hash;
use std::{marker::PhantomData, sync::Arc};

use crate::context::EvalContext;
use crate::{body::ComputedBody, tracker::Tracker};

pub struct Computed<T: Hash + Send + Sync> {
    tracker: Tracker,
    _t: PhantomData<T>,
}

impl<T: Hash + Send + Sync> Clone for Computed<T> {
    fn clone(&self) -> Computed<T> {
        Computed {
            tracker: self.tracker.clone(),
            _t: PhantomData,
        }
    }
}

impl<T: Hash + Send + Sync + 'static> std::default::Default for Computed<T> {
    fn default() -> Self {
        Computed::<T>::init()
    }
}

impl<T: Hash + Send + Sync + 'static> Computed<T> {
    pub fn new<F: Fn(&mut EvalContext) -> T + 'static>(handler: F) -> Self {
        let tracker = Tracker::new("Computed".to_owned());
        {
            let mut mut_tracker = tracker.get_mut();
            mut_tracker.set_computation(ComputedBody::new(None, handler));
        }
        Computed {
            tracker,
            _t: PhantomData,
        }
    }

    fn init() -> Self {
        let tracker = Tracker::new("Computed".to_owned());
        Computed {
            tracker,
            _t: PhantomData,
        }
    }

    pub fn set_handler<F: Fn(&mut EvalContext) -> T + 'static>(&self, handler: F) {
        {
            let mut mut_tracker = self.tracker.get_mut();
            mut_tracker.set_computation(ComputedBody::new(None, handler));
        }
    }

    fn get(&self, ctx: Option<&mut EvalContext>) -> Arc<T> {
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
        self.get(Some(ctx))
    }

    pub fn once(&self) -> Arc<T> {
        self.get(None)
    }

    pub fn map<R, F>(&self, handler: F) -> Computed<R>
    where
        F: Fn(&mut EvalContext, &T) -> R + 'static,
        R: Hash + Send + Sync + 'static,
    {
        let this = Computed {
            tracker: self.tracker.clone(),
            _t: PhantomData,
        };

        Computed::new(move |ctx| {
            let value = this.observe(ctx);
            handler(ctx, &value)
        })
    }

    pub fn to_value(&self) -> Value<T> {
        Value::Computed(Computed {
            tracker: self.tracker.clone(),
            _t: PhantomData,
        })
    }
}
