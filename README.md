# Observe

Lightweight Rust observables inspired by [MobX](https://mobx.js.org/).

Note: **This library is unstable and is subject to change**

The main goal of the library is to provide a generic re-usable state
management library for Rust applications and games.

Current state:

- [x] Core
  - [x] Tracker — a basic underlying structure for all other things
  - [x] Value — a simple observable box with a value
  - [x] Computed — a calculation based on `Value`s and another `Computed` values
  - [x] Reaction — allows to setup callbacks and react to state changes
  - [x] Batch — allows to batch several changes
- [ ] Extra
  - [x] Observable `Future`
  - [ ] Do we need an observable `Vec` ?
  - [ ] Do we need an observable `Map` ?

## Example

```rust
use observe::{Var, batch, Observable, MutObservable};

let mut value = Var::new(10);
let double = observe::computed!((value) ctx => *value.get(ctx) * 2);
let reaction = observe::autorun!((double) ctx => {
    println!("{}", *double.get(ctx));
});

batch(None, |b| {
  value.set(b, 20);
  value.set(b, 30);
  value.set(b, 40);
});
```
