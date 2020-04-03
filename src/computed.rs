use std::{any::Any, hash::Hash, rc::Rc, sync::Arc};

use crate::context::EvalContext;
use crate::{
    eval::Evaluation,
    tracker::{Local, Shared, TrackerImpl},
    types::{Apply, Type},
    value::Value,
    variable::default_hash,
};

// pub struct Computed<T: Hash, Impl: TrackerImpl> {
//     value: Value<T>,
// }

// impl<T, Impl> Deref for Computed<T, Impl>
// where
//     T: Hash,
//     Impl: TrackerImpl,
// {
//     type Target = Value<T>;
//     fn deref(&self) -> &Self::Target {
//         &self.value
//     }
// }

// // impl<T> From<Computed<T>> for Value<T> {
// //     fn from(from: Computed<T>) -> Value<T> {
// //         from.value.clone()
// //     }
// // }

// impl<T, Impl> Default for Computed<T, Impl>
// where
//     T: Hash,
//     Impl: TrackerImpl,
// {
//     fn default() -> Self {
//         Computed::empty()
//     }
// }

// impl<T, Impl> Computed<T, Impl>
// where
//     T: Hash,
//     Impl: TrackerImpl,
// {
//     pub fn new<F>(handler: F) -> Self
//     where
//         F: Fn(&mut EvalContext<Impl>) -> T + 'static,
//     {
//         let computed = Computed::empty();
//         computed.set_handler(handler);
//         computed
//     }

//     pub fn empty() -> Computed<T, Impl> {
//         let tracker = Tracker::new("Computed".to_owned());
//         Computed {
//             value: Value::Dynamic { tracker },
//         }
//     }

//     pub fn set_handler<F>(&self, handler: F)
//     where
//         F: Fn(&mut EvalContext<Impl>) -> T + 'static,
//     {
//         self.set_computation(Box::new(ComputedEngine::new(None, handler, false)));
//     }
// }

pub struct Computed<T, F, Impl>
where
    T: Hash,
    F: FnMut(&mut EvalContext<Impl>) -> T,
    Impl: TrackerImpl,
    Impl::Ptr: Apply<T>,
{
    current: Option<Type<Impl::Ptr, T>>,
    func: F,
}

impl<T, F, Impl> Computed<T, F, Impl>
where
    T: Hash,
    F: FnMut(&mut EvalContext<Impl>) -> T,
    Impl: TrackerImpl,
    Impl::Ptr: Apply<T>,
{
    pub fn new(func: F) -> Self {
        Computed {
            current: None,
            func,
        }
    }

    pub fn create(value: Option<T>, func: F, _is_observer: bool) -> Self {
        Computed {
            current: value.map(|v| Impl::ptr_wrap(v)),
            func,
        }
    }

    fn evaluate(&mut self, ctx: &mut EvalContext<Impl>) -> u64 {
        let next = (self.func)(ctx);
        let hash = default_hash(&next);
        self.current.replace(Impl::ptr_wrap(next));
        hash
    }
}

impl<T, F> Evaluation<Local> for Computed<T, F, Local>
where
    T: Hash + 'static,
    F: FnMut(&mut EvalContext<Local>) -> T,
{
    fn evaluate(&mut self, ctx: &mut EvalContext<Local>) -> u64 {
        self.evaluate(ctx)
    }

    fn get(&self) -> Rc<dyn Any + 'static> {
        self.current.as_ref().map(|r| r.clone()).unwrap()
    }
}

impl<T, F> Evaluation<Shared> for Computed<T, F, Shared>
where
    T: Hash + Send + Sync + 'static,
    F: FnMut(&mut EvalContext<Shared>) -> T,
{
    fn evaluate(&mut self, ctx: &mut EvalContext<Shared>) -> u64 {
        self.evaluate(ctx)
    }

    fn get(&self) -> Arc<dyn Any + Send + Sync + 'static> {
        self.current.as_ref().map(|r| r.clone()).unwrap()
    }
}

impl<T, F> From<Computed<T, F, Shared>> for Value<T, Shared>
where
    T: Hash + Send + Sync + 'static,
    F: FnMut(&mut EvalContext<Shared>) -> T + Send + Sync + 'static,
{
    fn from(from: Computed<T, F, Shared>) -> Value<T, Shared> {
        let value = Value::<T, Shared>::uninit();
        value.set_computation(Box::new(from));
        value
    }
}

impl<T, F> From<Computed<T, F, Local>> for Value<T, Local>
where
    T: Hash + 'static,
    F: FnMut(&mut EvalContext<Local>) -> T + 'static,
{
    fn from(from: Computed<T, F, Local>) -> Value<T, Local> {
        let value = Value::<T, Local>::uninit();
        value.set_computation(Box::new(from));
        value
    }
}
