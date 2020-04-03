use mockall::predicate::*;

use observe::{transaction, Computed, Shared, Value, Var};

use crate::suite::spy::{SharedMock, Spy};

#[test]
fn check_autorun() {
    let spy = SharedMock::new();
    let value = Value::<_, Shared>::from(Var::new(10));

    let reaction = Value::from(Computed::new({
        let value = value.clone();
        let spy = spy.clone();
        move |ctx| {
            spy.get().u32(*value.observe(ctx));
        }
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
