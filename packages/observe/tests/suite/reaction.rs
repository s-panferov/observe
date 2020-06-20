use mockall::predicate::*;

use observe::{transaction, Computed, MutObservable, Observable, Var};

use crate::suite::spy::{SharedMock, Spy};
use enclose::enc;

#[test]
fn check_autorun() {
    let spy = SharedMock::new();
    let value = Var::new(10);

    let reaction = Computed::new(enc!((value, spy) move |ctx| {
        spy.get().u32(*value.get(ctx));
    }));

    reaction.autorun();

    spy.get()
        .expect_u32()
        .with(eq(10))
        .return_const(())
        .times(1);

    reaction.update();

    spy.get().checkpoint();

    spy.get()
        .expect_u32()
        .with(eq(20))
        .return_const(())
        .times(1);

    spy.get()
        .expect_u32()
        .with(eq(30))
        .return_const(())
        .times(1);

    // try to set the same value twice
    value.set_now(20);

    // should not fire reactions
    value.set_now(20);

    value.set_now(30);

    spy.get().checkpoint();

    transaction(None, |tx| {
        spy.get()
            .expect_u32()
            .with(eq(40))
            .return_const(())
            .times(0);

        // inner transaction should NOT fire reactions
        transaction(Some(tx), |tx| {
            // this section would trigger reactions three times without transaction
            value.set(tx, 20);
            value.set(tx, 30);
            value.set(tx, 40);
        });

        spy.get().checkpoint();

        spy.get()
            .expect_u32()
            .with(eq(40))
            .return_const(())
            .times(1);
    });
}
