use mockers::Scenario;
use observe::{Computed, Value};

use suite::spy::{SharedSpy, Spy};

#[test]
fn simple_computed() {
  let scenario = Scenario::new();
  let spy = scenario.create_mock::<SharedSpy>();

  let mut value = Value::new(10);
  let computed = Computed::new({
    let value = value.clone();
    let spy = spy.clone();
    move |ctx| {
      spy.trigger();
      *value.observe(ctx) * 2
    }
  });

  scenario.expect(spy.trigger_call().and_return_default().times(1));
  assert_eq!(*computed.once(), 20);
  scenario.checkpoint();

  scenario.expect(spy.trigger_call().and_return_default().times(1));

  value.set_now(20);
  value.set_now(30);
  value.set_now(20);

  assert_eq!(*computed.once(), 40);
}

#[test]
fn computed_chain() {
  let mut value = Value::new(10);

  let scenario = Scenario::new();
  let double_spy = scenario.create_mock::<SharedSpy>();
  let quadruple_spy = scenario.create_mock::<SharedSpy>();

  let double = Computed::new({
    let value = value.clone();
    let double_spy = double_spy.clone();
    move |ctx| {
      double_spy.trigger();
      *value.observe(ctx) * 2
    }
  });

  let quadruple = Computed::new({
    let double = double.clone();
    let quadruple_spy = quadruple_spy.clone();
    move |ctx| {
      quadruple_spy.trigger();
      *double.observe(ctx) * 2
    }
  });

  scenario.expect(double_spy.trigger_call().and_return_default().times(0));
  scenario.expect(quadruple_spy.trigger_call().and_return_default().times(0));

  scenario.checkpoint();

  scenario.expect(double_spy.trigger_call().and_return_default().times(1));
  scenario.expect(quadruple_spy.trigger_call().and_return_default().times(1));

  assert_eq!(*quadruple.once(), 40);

  scenario.checkpoint();

  value.set_now(20);
  value.set_now(20);
  value.set_now(20);

  scenario.expect(double_spy.trigger_call().and_return_default().times(1));
  scenario.expect(quadruple_spy.trigger_call().and_return_default().times(1));

  assert_eq!(*quadruple.once(), 80);
  assert_eq!(*quadruple.once(), 80);
}
