use observe::transaction;
use observe::EvalContext;
use observe::Reaction;
use observe::Transaction;
use observe::{Computed, Value};
use std::sync::Arc;

#[observe::store]
struct Store {
    pub value: Value<u64>,
    pub computed: Computed<u64>,
    pub reaction: Reaction,
}

impl Store {
    #[observe::create]
    pub fn new() -> Arc<Self> {
        let value = Value::new(0);
        Arc::new(Store {
            value: value.clone(),
            computed: Default::default(),
            reaction: Default::default(),
        })
    }

    fn computed(&self, ctx: &mut EvalContext) -> u64 {
        *self.value.observe(ctx) * 2
    }

    fn reaction(&self, ctx: &mut EvalContext) {
        println!("REACTION {:?}", *self.computed.observe(ctx))
    }

    pub fn action(&self, value: u64, tx: &mut Transaction) {
        self.value.set(value, tx);
    }
}

#[test]
fn store() {
    let store = Store::new();
    transaction(None, |tx| {
        store.action(10, tx);
        store.action(20, tx);
        store.action(30, tx)
    })
}
