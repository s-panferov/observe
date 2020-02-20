# Observe

Lightweight Rust observables inspired by [MobX](https://mobx.js.org/). It's hard do active the same level of ergonomics in Rust, but I tried to do my best.

Note: **This library is unstable and is subject to change**

The main goal of the library is to provide a state management library for web-based Rust applications and games.

Current state:

- [x] Core
  - [x] Tracker — a basic underlying structure for all other things
  - [x] Value — a simple observable box with a value
  - [x] Computed — a calculation based on `Value`s and another `Computed` values
  - [x] Reaction — allows to setup callbacks and react to state changes
  - [x] Transaction — allows to batch several changes
- [ ] Primitives
  - [ ] Observable `Vec`
  - [ ] Observable `Map`
  - [ ] Observable `Future`

## Example

```rust
let mut value = Value::new(10);
let double = Computed::new({
  let value = value.clone();
  move |ctx| *value.observe(ctx) * 2
});

let reaction = autorun({
  let double = double.clone();
  move |ctx| {
    println!("{}", *double.observe(ctx));
  }
});

reaction.run();

transaction(None, |tx| {
  // this section would trigger reactions three times without transaction
  value.set(20, tx);
  value.set(30, tx);
  value.set(40, tx);
});
```
