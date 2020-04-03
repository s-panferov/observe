use observe::transaction;
use observe::{
    shared::{EvalContext, Transaction, Value},
    Var,
};

use futures::Future;
use std::{fmt::Debug, hash::Hash, ops::Mul, sync::Arc, task::Poll};

#[observe::store(shared)]
struct Store<T: Mul + Hash + Debug + Send + Sync + Clone + 'static>
where
    <T as std::ops::Mul>::Output: Hash + Debug + Send + Sync + Clone + 'static,
{
    pub value: Value<T>,

    #[computed]
    pub computed: Value<<T as std::ops::Mul>::Output>,

    #[autorun]
    pub reaction: Value<()>,

    #[autorun(future = "tokio")]
    pub data: Value<Poll<u64>>,
}

impl<T: Mul + Hash + Debug + Clone + Send + Sync + 'static> Store<T>
where
    <T as std::ops::Mul>::Output: Hash + Debug + Send + Sync + Clone + 'static,
{
    #[observe::constructor]
    pub fn new(value: T) -> Arc<Self> {
        Arc::new(Store {
            value: Value::from(Var::new(value)),
            computed: Value::uninit(),
            reaction: Value::uninit(),
            data: Value::uninit(),
        })
    }

    fn computed(&self, ctx: &mut EvalContext) -> <T as std::ops::Mul>::Output {
        (*self.value.observe(ctx)).clone() * (*self.value.observe(ctx)).clone()
    }

    fn reaction(&self, ctx: &mut EvalContext) {
        println!("REACTION {:?}", *self.computed.observe(ctx))
    }

    fn data(&self, _ctx: &mut EvalContext) -> impl Future<Output = u64> {
        futures::future::ready(10)
    }

    pub fn action(&self, tx: &mut Transaction, value: T) {
        self.value.set(tx, value);
    }
}

#[test]
fn store() {
    let store = Store::new(0);
    transaction(None, |tx| {
        store.action(tx, 10);
        store.action(tx, 20);
        store.action(tx, 30)
    })
}
