use observe::{autorun, transaction, Computed, Value};

use crate::suite::spy::{SharedMock, Spy};
use std::mem;

use mockall::predicate::*;

#[test]
fn check_autorun() {
    let spy = SharedMock::new();

    let mut value = Value::new(10);

    let reaction = autorun({
        let value = value.clone();
        let spy = spy.clone();
        move |ctx| {
            spy.get().u32(*value.observe(ctx));
        }
    });

    spy.get()
        .expect_u32()
        .with(eq(10))
        .return_const(())
        .times(1);

    reaction.run();

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
            value.set(20, tx);
            value.set(30, tx);
            value.set(40, tx);
        });

        spy.get().checkpoint();
        spy.get()
            .expect_u32()
            .with(eq(40))
            .return_const(())
            .times(1);
    });
}

#[test]
fn become_observed() {
    let spy = SharedMock::new();

    let mut value = Value::new(10);
    let double = Computed::new({
        let value = value.clone();
        move |ctx| *value.observe(ctx) * 2
    });

    value.on_become_observed({
        let spy = spy.clone();
        move || spy.get().trigger()
    });

    value.on_become_unobserved({
        let spy = spy.clone();
        move || spy.get().trigger()
    });

    let reaction = autorun({
        let double = double.clone();
        move |ctx| {
            println!("{}", *double.observe(ctx));
        }
    });

    spy.get().expect_trigger().return_const(()).times(1);
    reaction.run();

    spy.get().checkpoint();

    spy.get().expect_trigger().return_const(()).times(1);
    mem::drop(reaction);
}
