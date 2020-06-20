use crate::{context::EvalContext, Computed, Transaction};
use std::{fmt::Debug, hash::Hash, ops::Deref};

#[cfg(feature = "futures")]
use crate::future::{ComputedFuture, FutureRuntime};

use parking_lot::MappedRwLockReadGuard;

pub enum Ref<'a, T> {
    Lock(MappedRwLockReadGuard<'a, T>),
    Ref(&'a T),
}

impl<'a, T> Deref for Ref<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Ref::Lock(guard) => guard.deref(),
            Ref::Ref(t) => t,
        }
    }
}

pub trait Observable<T> {
    fn access(&self, ctx: Option<&EvalContext>) -> Ref<T>;

    fn get(&self, ctx: &EvalContext) -> Ref<T> {
        self.access(Some(ctx))
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    where
        T: Debug,
    {
        write!(f, "_")
    }

    fn once(&self) -> Ref<T> {
        self.access(None)
    }
}

pub trait ObservableExt<T>: Observable<T> + Clone {
    fn map<R, F>(&self, handler: F) -> Computed<R>
    where
        F: Fn(&EvalContext, &T) -> R + 'static,
        R: Clone + Hash + 'static,
        Self: 'static,
    {
        let this = self.clone();
        Computed::new(move |ctx| {
            let value = this.get(ctx);
            handler(ctx, &value)
        })
    }

    #[cfg(feature = "futures")]
    fn stream<Rt>(
        &self,
    ) -> (
        ComputedFuture<(), Rt>,
        futures::channel::mpsc::UnboundedReceiver<T>,
    )
    where
        T: Clone + 'static,
        Rt: FutureRuntime,
        Self: 'static,
    {
        let (sender, recv) = futures::channel::mpsc::unbounded(); // TODO oneshot

        let this = self.clone();
        let reaction = ComputedFuture::new({
            let this = this.clone();
            move |ctx: &EvalContext| {
                use futures::sink::SinkExt;
                let value = this.get(ctx).clone();
                let mut sender = sender.clone();
                async move {
                    sender.send(value).await.unwrap();
                }
            }
        });

        (reaction, recv)
    }
}

impl<T, V> ObservableExt<V> for T where T: Observable<V> + Clone {}

pub trait MutObservable<T>: Observable<T> {
    fn modify<F>(&self, tx: Option<&mut Transaction>, mapper: F)
    where
        F: FnOnce(&mut T);

    fn set(&self, tx: &mut Transaction, value: T) {
        self.modify(Some(tx), move |v| *v = value)
    }

    fn set_now(&self, value: T) {
        self.modify(None, move |v| *v = value)
    }
}
