use std::fmt::{Debug, Formatter};
use std::{hash::Hash, ops::Deref, rc::Rc, sync::Arc};

use crate::context::EvalContext;
use crate::transaction::Transaction;
use crate::{
    computed::Computed,
    future::{ComputedFuture, FutureRuntime},
    tracker::{Local, Shared, Tracker, TrackerImpl},
    types::Apply,
    WeakTracker,
};

use std::task::Poll;

pub enum Value<T, Impl>
where
    Impl: TrackerImpl,
    Impl::Ptr: Apply<T>,
{
    Empty,
    Const(<Impl::Ptr as Apply<T>>::Result),
    Dynamic { tracker: Tracker<Impl> },
}

impl<T, Impl> Debug for Value<T, Impl>
where
    T: Debug,
    Impl: TrackerImpl,
    Impl::Ptr: Apply<T>,
{
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<T, Impl> Clone for Value<T, Impl>
where
    Impl: TrackerImpl,
    Impl::Ptr: Apply<T>,
    <Impl::Ptr as Apply<T>>::Result: Clone,
{
    fn clone(&self) -> Value<T, Impl> {
        match self {
            Value::Empty => Value::Empty,
            Value::Const(v) => Value::Const(v.clone()),
            Value::Dynamic { tracker } => Value::Dynamic {
                tracker: tracker.clone(),
            },
        }
    }
}

impl<T, Impl> Default for Value<Option<T>, Impl>
where
    T: Debug,
    Impl: TrackerImpl,
    Impl::Ptr: Apply<Option<T>>,
{
    fn default() -> Self {
        Value::Empty
    }
}

impl<T, Impl> From<T> for Value<Option<T>, Impl>
where
    T: Hash + Debug + 'static,
    Impl: TrackerImpl,
    Impl::Ptr: Apply<Option<T>>,
    Impl::PtrWeak: Apply<Option<T>>,
    <Impl::Ptr as Apply<Option<T>>>::Result: Clone + Deref<Target = Option<T>>,
{
    fn from(v: T) -> Self {
        Value::cons(Some(v))
    }
}

impl<T, Impl> From<T> for Value<T, Impl>
where
    T: Hash + 'static,
    Impl: TrackerImpl,
    Impl::Ptr: Apply<T>,
    Impl::PtrWeak: Apply<T>,
    <Impl::Ptr as Apply<T>>::Result: Clone + Deref<Target = T>,
{
    fn from(v: T) -> Self {
        Value::cons(v)
    }
}

impl<T, Impl> Value<T, Impl>
where
    T: Hash + 'static,
    Impl: TrackerImpl,
    Impl::Ptr: Apply<T>,
    Impl::PtrWeak: Apply<T>,
    <Impl::Ptr as Apply<T>>::Result: Clone + Deref<Target = T>,
{
    pub fn cons(value: T) -> Value<T, Impl> {
        Value::Const(Impl::ptr_wrap(value))
    }

    pub fn uninit() -> Value<T, Impl> {
        let tracker = Tracker::<Impl>::new(String::from(""));
        Value::Dynamic { tracker }
    }

    pub fn name(&self) -> String {
        match self {
            Value::Empty => String::new(),
            Value::Const(_v) => String::new(),
            Value::Dynamic { tracker } => tracker.name(),
        }
    }

    pub fn set_name(&self, name: String) {
        match self {
            Value::Empty => {}
            Value::Const(_v) => {}
            Value::Dynamic { tracker } => tracker.set_name(name),
        };
    }

    pub fn set_computation(&self, eval: Box<Impl::Eval>) {
        match self {
            Value::Const(_) | Value::Empty => unreachable!(),
            Value::Dynamic { tracker } => {
                tracker.set_computation(eval);
            }
        }
    }

    pub fn autorun(&self) {
        match self {
            Value::Empty => {}
            Value::Const(_v) => {}
            Value::Dynamic { tracker } => tracker.autorun(),
        };
    }

    pub fn update(&self) {
        match self {
            Value::Empty => unreachable!(),
            Value::Const(_v) => {}
            Value::Dynamic { tracker } => tracker.update(),
        };
    }

    pub fn is_empty(&self) -> bool {
        if let Value::Empty = self {
            return true;
        } else {
            return false;
        }
    }

    pub fn tracker(&self) -> Option<&Tracker<Impl>> {
        match self {
            Value::Empty => None,
            Value::Const(_c) => None,
            Value::Dynamic { ref tracker } => Some(tracker),
        }
    }

    pub fn weak(&self) -> WeakValue<T, Impl> {
        match self {
            Value::Empty => WeakValue::Uninit,
            Value::Const(c) => WeakValue::Const(Impl::ptr_downgrade(c)),
            Value::Dynamic { tracker } => WeakValue::Dynamic {
                tracker: tracker.weak(),
            },
        }
    }
}

impl<T> Value<T, Local>
where
    T: Hash + 'static,
{
    pub fn set(&self, tx: &mut Transaction<Local>, next: T) {
        match self {
            Value::Empty => unreachable!(),
            Value::Const(_v) => {}
            Value::Dynamic { tracker } => {
                tracker.set(Some(tx), Rc::new(next));
            }
        }
    }

    pub fn set_now(&self, next: T) {
        match self {
            Value::Empty => unreachable!(),
            Value::Const(_v) => {}
            Value::Dynamic { tracker } => {
                tracker.set(None, Rc::new(next));
            }
        }
    }

    pub fn map<R, F>(&self, handler: F) -> Value<R, Local>
    where
        F: Fn(&mut EvalContext<Local>, &T) -> R + 'static,
        R: Hash + Debug + 'static,
    {
        match self {
            Value::Empty => Value::Empty,
            Value::Const(v) => Value::Const(Rc::new(handler(&mut EvalContext::empty(), &v))),
            Value::Dynamic { .. } => {
                let this = (*self).clone();
                Value::from(Computed::new(move |ctx| {
                    let value = this.observe(ctx);
                    handler(ctx, &value)
                }))
            }
        }
    }

    pub fn observe(&self, ctx: &mut EvalContext<Local>) -> Rc<T> {
        self.get(Some(ctx))
    }

    pub fn once(&self) -> Rc<T> {
        self.get(None)
    }

    fn get(&self, ctx: Option<&mut EvalContext<Local>>) -> Rc<T> {
        match self {
            Value::Empty => unreachable!(),
            Value::Const(v) => v.clone(),
            Value::Dynamic { tracker } => tracker.get(ctx).downcast::<T>().unwrap(),
        }
    }
}

impl<T> Value<T, Shared>
where
    T: Hash + Send + Sync + 'static,
{
    pub fn set(&self, tx: &mut Transaction<Shared>, next: T) {
        match self {
            Value::Empty => unreachable!(),
            Value::Const(_v) => {}
            Value::Dynamic { tracker } => {
                tracker.set(Some(tx), Arc::new(next));
            }
        }
    }

    pub fn set_now(&self, next: T) {
        match self {
            Value::Empty => unreachable!(),
            Value::Const(_v) => {}
            Value::Dynamic { tracker } => {
                tracker.set(None, Arc::new(next));
            }
        }
    }

    pub fn map<R, F>(&self, handler: F) -> Value<R, Shared>
    where
        F: Fn(&mut EvalContext<Shared>, &T) -> R + Send + Sync + 'static,
        R: Hash + Send + Sync + Debug + 'static,
    {
        match self {
            Value::Empty => Value::Empty,
            Value::Const(v) => Value::Const(Arc::new(handler(&mut EvalContext::empty(), &v))),
            Value::Dynamic { .. } => {
                let this = self.clone();
                Value::from(Computed::new(move |ctx| {
                    let value = this.observe(ctx);
                    handler(ctx, &value)
                }))
            }
        }
    }

    pub fn observe(&self, ctx: &mut EvalContext<Shared>) -> Arc<T> {
        self.get(Some(ctx))
    }

    pub fn once(&self) -> Arc<T> {
        self.get(None)
    }

    fn get(&self, ctx: Option<&mut EvalContext<Shared>>) -> Arc<T> {
        match self {
            Value::Empty => unreachable!(),
            Value::Const(v) => v.clone(),
            Value::Dynamic { tracker } => tracker.get(ctx).downcast::<T>().unwrap(),
        }
    }

    // where
    // T: Hash + Send + Sync + 'static,
    // P: FutureProvider<T, Shared> + Send + Sync + 'static,
    // S: FutureRuntime + Send + Sync + 'static,
    // P::Output: Future<Output = T> + Send + 'static,

    #[cfg(feature = "futures")]
    pub fn stream<Rt>(
        &self,
    ) -> (
        Value<Poll<()>, Shared>,
        futures::channel::mpsc::UnboundedReceiver<Arc<T>>,
    )
    where
        Rt: FutureRuntime + Send + Sync + 'static,
    {
        let (sender, recv) = futures::channel::mpsc::unbounded(); // TODO oneshot
        let this = self.clone();
        let reaction = Value::from(ComputedFuture::<(), Rt, _, Shared>::new({
            let this = this.clone();
            move |ctx: &mut EvalContext<Shared>| {
                use futures::sink::SinkExt;
                let value = this.observe(ctx);
                let mut sender = sender.clone();
                async move {
                    sender.send(value).await.unwrap();
                }
            }
        }));

        reaction.set_name(self.name() + "_channel");

        (reaction, recv)
    }
}

pub enum WeakValue<T, Impl>
where
    T: Hash + 'static,
    Impl: TrackerImpl,
    Impl::PtrWeak: Apply<T>,
{
    Uninit,
    Const(<Impl::PtrWeak as Apply<T>>::Result),
    Dynamic { tracker: WeakTracker<Impl> },
}

impl<T, Impl> WeakValue<T, Impl>
where
    T: Hash + 'static,
    Impl: TrackerImpl,
    Impl::Ptr: Apply<T>,
    Impl::PtrWeak: Apply<T>,
{
    pub fn upgrade(&self) -> Option<Value<T, Impl>> {
        match self {
            WeakValue::Uninit => Some(Value::Empty),
            WeakValue::Const(c) => Impl::ptr_upgrade(c).map(|v| Value::Const(v)),
            WeakValue::Dynamic { tracker } => {
                tracker.upgrade().map(|tracker| Value::Dynamic { tracker })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::context::EvalContext;
    use crate::{
        tracker::{Expired, Freshness, Shared, Tracker},
        Value, Var,
    };

    #[test]
    fn expire_on_set() {
        let tracker = Tracker::<Shared>::new("Tracker".to_owned());
        let value = Value::<_, Shared>::from(Var::<_, Shared>::new(10));

        tracker.set_computation({
            let value = value.clone();
            Box::new(move |ctx: &mut EvalContext<_>| *value.observe(ctx))
        });

        tracker.update();

        assert_eq!(tracker.state(), Freshness::UpToDate);

        value.set_now(20);

        assert_eq!(tracker.state(), Freshness::Expired(Expired::Maybe));
    }
}
