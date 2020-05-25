use crate::{context::EvalContext, Computed, Transaction};
use std::hash::Hash;

#[cfg(feature = "futures")]
use crate::future::{ComputedFuture, FutureRuntime};

pub trait Observable<T> {
    fn access(&self, ctx: Option<&mut EvalContext>) -> T;

    fn get(&self, ctx: &mut EvalContext) -> T {
        self.access(Some(ctx))
    }

    fn once(&self) -> T {
        self.access(None)
    }
}

pub trait ObservableExt<T>: Observable<T> + Clone {
    fn map<R, F>(&self, handler: F) -> Computed<R>
    where
        F: Fn(&mut EvalContext, &T) -> R + 'static,
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
        T: 'static,
        Rt: FutureRuntime,
        Self: 'static,
    {
        let (sender, recv) = futures::channel::mpsc::unbounded(); // TODO oneshot

        let this = self.clone();
        let reaction = ComputedFuture::new({
            let this = this.clone();
            move |ctx: &mut EvalContext| {
                use futures::sink::SinkExt;
                let value = this.get(ctx);
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
    fn modify(&self, tx: Option<&mut Transaction>, value: T);
    fn set(&self, tx: &mut Transaction, value: T) {
        self.modify(Some(tx), value)
    }

    fn set_now(&self, value: T) {
        self.modify(None, value)
    }
}
