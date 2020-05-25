use futures::Future;

use observe::{
    future::{ComputedFuture, TokioLocal},
    transaction, Computed, EvalContext, MutObservable, Observable, Transaction, Var,
};

use enclose::enclose;
use std::{fmt::Debug, hash::Hash, ops::Mul};

struct Store<T: Mul + Hash + Debug + Clone + 'static>
where
    <T as std::ops::Mul>::Output: Hash + Debug + Clone + 'static,
{
    pub value: Var<T>,
    pub computed: Computed<<T as std::ops::Mul>::Output>,
    pub reaction: Computed<()>,
    pub data: ComputedFuture<u64, TokioLocal>,
}

impl<T: Mul + Hash + Debug + Clone + 'static> Store<T>
where
    <T as std::ops::Mul>::Output: Hash + Debug + Clone + 'static,
{
    pub fn new(value: T) -> Self {
        let value = Var::new(value);
        let computed = observe::computed!((value) ctx => Self::computed(&value, ctx));
        let reaction = observe::autorun!((computed) ctx => Self::reaction(&computed, ctx));
        let data = observe::future!(() ctx => Self::data(ctx));

        Store {
            value,
            computed,
            reaction,
            data,
        }
    }

    fn computed(value: &Var<T>, ctx: &mut EvalContext) -> <T as std::ops::Mul>::Output {
        value.get(ctx).clone() * value.get(ctx).clone()
    }

    fn reaction(computed: &Computed<<T as std::ops::Mul>::Output>, ctx: &mut EvalContext) {
        println!("REACTION {:?}", *computed.get(ctx))
    }

    fn data(_ctx: &mut EvalContext) -> impl Future<Output = u64> {
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
