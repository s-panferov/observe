use observe::transaction;
use observe::EvalContext;
use observe::Transaction;
use observe::{Computed, Value};
use std::{fmt::Debug, hash::Hash, ops::Mul, rc::Rc};

#[observe::store]
struct Store<T: Mul + Hash + Debug + Clone + 'static>
where
    <T as std::ops::Mul>::Output: Hash + Debug + Clone + 'static,
{
    pub value: Value<T>,

    #[computed]
    pub computed: Computed<<T as std::ops::Mul>::Output>,

    #[autorun]
    pub reaction: Value<()>,
    // #[fut_autorun]
    // pub data: Value<Payload<u64>>,
}

impl<T: Mul + Hash + Debug + Clone + 'static> Store<T>
where
    <T as std::ops::Mul>::Output: Hash + Debug + Clone + 'static,
{
    #[observe::create]
    pub fn new(value: T) -> Rc<Self> {
        Rc::new(Store {
            value: Value::var(value),
            computed: Computed::init(),
            reaction: Value::init(),
            // data: Value::init(),
        })
    }

    fn computed(&self, ctx: &mut EvalContext) -> <T as std::ops::Mul>::Output {
        (*self.value.observe(ctx)).clone() * (*self.value.observe(ctx)).clone()
    }

    fn reaction(&self, ctx: &mut EvalContext) {
        println!("REACTION {:?}", *self.computed.observe(ctx))
    }

    async fn data(&self, _ctx: &mut EvalContext) -> u64 {
        futures::future::ready(10).await
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
