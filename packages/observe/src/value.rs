use crate::{observable::Ref, Const, EvalContext, Observable};
use std::{fmt::Debug, hash::Hash, sync::Arc};

pub struct Value<T>
where
    T: Hash,
{
    pub(crate) value: Arc<dyn Observable<T>>,
}

impl<T> Debug for Value<T>
where
    T: Hash + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Value[")?;
        self.value.debug(f)?;
        write!(f, "]")?;
        Ok(())
    }
}

impl<T> From<T> for Value<T>
where
    T: Hash + 'static,
{
    fn from(value: T) -> Self {
        Value::from(Const::new(value))
    }
}

impl<T> From<T> for Value<Option<T>>
where
    T: Hash + 'static,
{
    fn from(value: T) -> Self {
        Value::from(Const::new(Some(value)))
    }
}

impl<T> Clone for Value<T>
where
    T: Hash + 'static,
{
    fn clone(&self) -> Self {
        Value {
            value: self.value.clone(),
        }
    }
}

impl<T> Observable<T> for Value<T>
where
    T: Hash + 'static,
{
    fn access(&self, ctx: Option<&EvalContext>) -> Ref<T> {
        self.value.access(ctx)
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    where
        T: Debug,
    {
        self.value.debug(f)
    }
}

impl<T> Default for Value<T>
where
    T: Hash + Default + 'static,
{
    fn default() -> Self {
        Value::from(Const::new(Default::default()))
    }
}
