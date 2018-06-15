use mockers::Scenario;
use observe::{autorun, transaction, Computed, Value};

use std::mem;
use suite::spy::{SharedSpy, Spy};

#[test]
fn check_autorun() {
  let scenario = Scenario::new();
  let spy = scenario.create_mock::<SharedSpy>();

  let mut value = Value::new(10);

  let reaction = autorun({
    let value = value.clone();
    let spy = spy.clone();
    move |ctx| {
      spy.u32(*value.observe(ctx));
    }
  });

  scenario.expect(spy.u32_call(10).and_return_default().times(1));
  reaction.run();

  scenario.checkpoint();

  scenario.expect(spy.u32_call(20).and_return_default().times(1));
  scenario.expect(spy.u32_call(30).and_return_default().times(1));

  // try to set the same value twice
  value.set_now(20);

  // should not fire reactions
  value.set_now(20);

  value.set_now(30);

  scenario.checkpoint();

  transaction(None, |tx| {
    scenario.expect(spy.u32_call(40).and_return_default().times(0));

    // inner transaction should NOT fire reactions
    transaction(Some(tx), |tx| {
      // this section would trigger reactions three times without transaction
      value.set(20, tx);
      value.set(30, tx);
      value.set(40, tx);
    });

    scenario.checkpoint();
    scenario.expect(spy.u32_call(40).and_return_default().times(1));
  });
}

#[test]
fn become_observed() {
  let scenario = Scenario::new();
  let spy = scenario.create_mock::<SharedSpy>();

  let mut value = Value::new(10);
  let double = Computed::new({
    let value = value.clone();
    move |ctx| *value.observe(ctx) * 2
  });

  value.on_become_observed({
    let spy = spy.clone();
    move || spy.trigger()
  });

  value.on_become_unobserved({
    let spy = spy.clone();
    move || spy.trigger()
  });

  let reaction = autorun({
    let double = double.clone();
    move |ctx| {
      println!("{}", *double.observe(ctx));
    }
  });

  scenario.expect(spy.trigger_call().and_return_default().times(1));
  reaction.run();

  scenario.checkpoint();

  scenario.expect(spy.trigger_call().and_return_default().times(1));

  mem::drop(reaction);
}
