use observe::transaction;
use observe::EvalContext;
use observe::Reaction;
use observe::Transaction;
use observe::{Computed, Var};
use std::{fmt::Debug, hash::Hash, ops::Mul, sync::Arc};

#[observe::store]
struct Store<T: Mul + Hash + Send + Sync + Debug + Clone + 'static>
where
    <T as std::ops::Mul>::Output: Hash + Send + Sync + Debug + Clone + 'static,
{
    pub value: Var<T>,
    pub computed: Computed<<T as std::ops::Mul>::Output>,
    pub reaction: Reaction,
}

impl<T: Mul + Hash + Send + Sync + Debug + Clone + 'static> Store<T>
where
    <T as std::ops::Mul>::Output: Hash + Send + Sync + Debug + Clone + 'static,
{
    #[observe::create]
    pub fn new(value: T) -> Arc<Self> {
        let value = Var::new(value);
        Arc::new(Store {
            value: value.clone(),
            computed: Default::default(),
            reaction: Default::default(),
        })
    }

    fn computed(&self, ctx: &mut EvalContext) -> <T as std::ops::Mul>::Output {
        (*self.value.observe(ctx)).clone() * (*self.value.observe(ctx)).clone()
    }

    fn reaction(&self, ctx: &mut EvalContext) {
        println!("REACTION {:?}", *self.computed.observe(ctx))
    }

    pub fn action(&self, value: T, tx: &mut Transaction) {
        self.value.set(value, tx);
    }
}

#[test]
fn store() {
    let store = Store::new(0);
    transaction(None, |tx| {
        store.action(10, tx);
        store.action(20, tx);
        store.action(30, tx)
    })
}
