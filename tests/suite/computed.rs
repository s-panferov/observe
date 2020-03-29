use observe::Value;

use crate::suite::spy::{SharedMock, Spy};

#[test]
fn simple_computed() {
    let spy = SharedMock::new();

    let value = Value::var(10);
    let computed = Value::computed({
        let value = value.clone();
        let spy = spy.clone();
        move |ctx| {
            spy.get().trigger();
            let i = *value.observe(ctx);
            i * 2
        }
    });

    spy.get().expect_trigger().return_const(()).times(1);

    assert_eq!(*computed.once(), 20);

    spy.get().checkpoint();

    spy.get().expect_trigger().return_const(()).times(1);

    value.set_now(30);
    value.set_now(40);
    value.set_now(30);

    assert_eq!(*computed.once(), 60);
}

#[test]
fn computed_chain() {
    let value = Value::var(10);

    let double_spy = SharedMock::new();
    let quadruple_spy = SharedMock::new();

    let double = Value::computed({
        let value = value.clone();
        let double_spy = double_spy.clone();
        move |ctx| {
            double_spy.get().trigger();
            *value.observe(ctx) * 2
        }
    });

    let quadruple = Value::computed({
        let double = double.clone();
        let quadruple_spy = quadruple_spy.clone();
        move |ctx| {
            quadruple_spy.get().trigger();
            *double.observe(ctx) * 2
        }
    });

    double_spy.get().expect_trigger().return_const(()).times(0);
    quadruple_spy
        .get()
        .expect_trigger()
        .return_const(())
        .times(0);

    double_spy.get().checkpoint();
    quadruple_spy.get().checkpoint();

    double_spy.get().expect_trigger().return_const(()).times(1);
    quadruple_spy
        .get()
        .expect_trigger()
        .return_const(())
        .times(1);

    assert_eq!(*quadruple.once(), 40);

    double_spy.get().checkpoint();
    quadruple_spy.get().checkpoint();

    value.set_now(20);
    value.set_now(20);
    value.set_now(20);

    double_spy.get().expect_trigger().return_const(()).times(1);
    quadruple_spy
        .get()
        .expect_trigger()
        .return_const(())
        .times(1);

    assert_eq!(*quadruple.once(), 80);
    assert_eq!(*quadruple.once(), 80);
}
