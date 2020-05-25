use crate::{observable::Ref, Const, EvalContext, Observable};
use std::{hash::Hash, sync::Arc};

pub struct Value<T> {
    pub(crate) value: Arc<dyn Observable<T>>,
}

impl<T> Clone for Value<T> {
    fn clone(&self) -> Self {
        Value {
            value: self.value.clone(),
        }
    }
}

impl<T> Observable<T> for Value<T> {
    fn access(&self, ctx: Option<&mut EvalContext>) -> Ref<T> {
        self.value.access(ctx)
    }
}

impl<T> Default for Value<T>
where
    T: Hash + Clone + Default + 'static,
{
    fn default() -> Self {
        Value::from(Const::new(Default::default()))
    }
}
