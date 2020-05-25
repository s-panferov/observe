use observe::{MutObservable, Observable, Var};

use crate::suite::spy::{SharedMock, Spy};
use enclose::enclose;

#[test]
fn simple_computed() {
    let spy = SharedMock::new();

    let value = Var::new(10);
    let computed = observe::computed!((value, spy) ctx => {
        spy.get().trigger();
        let i = value.get(ctx);
        *i * 2
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
    let value = Var::new(10);

    let double_spy = SharedMock::new();
    let quadruple_spy = SharedMock::new();

    let double = observe::computed!((value, double_spy) ctx => {
        double_spy.get().trigger();
        *value.get(ctx) * 2
    });

    let quadruple = observe::computed!((double, quadruple_spy) ctx => {
        quadruple_spy.get().trigger();
        *double.get(ctx) * 2
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
