use std::hash::Hash;
use std::{marker::PhantomData, sync::Arc};

use crate::context::EvalContext;
use crate::{body::ComputedBody, tracker::Tracker};

#[derive(Clone)]
pub struct Computed<T: Hash + Send + Sync> {
    tracker: Tracker,
    _t: PhantomData<T>,
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
}
