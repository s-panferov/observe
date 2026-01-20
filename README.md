# observe

[![Crates.io](https://img.shields.io/crates/v/observe.svg)](https://crates.io/crates/observe)
[![Documentation](https://docs.rs/observe/badge.svg)](https://docs.rs/observe)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Declarative reactive programming for Rust, inspired by [MobX](https://mobx.js.org/).

## Features

- **Automatic dependency tracking** - Dependencies are tracked at runtime, no manual subscriptions
- **Fine-grained reactivity** - Only affected computations re-run when state changes
- **Hash-based change detection** - Efficient change detection using value hashes
- **Batching** - Group multiple state changes, reactions run once at the end
- **Zero boilerplate** - Simple, intuitive API

## Installation

```bash
cargo add observe
```

## Quick Start

```rust
use observe::rc::{batch, Computed, Reaction, Var};

// Create reactive state
let count = Var::new(0);

// Create a derived value that automatically tracks dependencies
let doubled = Computed::new(Box::new({
    let count = count.clone();
    move |cx| count.get(cx) * 2
}));

// Create a reaction that runs when dependencies change
let reaction = Reaction::new(Box::new({
    let doubled = doubled.clone();
    move |cx| {
        println!("Doubled value: {}", *doubled.get(cx));
    }
}));

// Initial run
reaction.update();  // Prints: "Doubled value: 0"

// Update state inside a batch - reaction runs automatically
batch(|| {
    count.set(5);
});
// Prints: "Doubled value: 10"
```

## Core Concepts

### Var - Reactive State

`Var<T>` holds mutable reactive state. When its value changes, all dependent computations and reactions are notified.

```rust
use observe::rc::Var;

// Create a new reactive variable
let name = Var::new(String::from("Alice"));

// Read value without tracking (useful outside reactive context)
assert_eq!(name.get_once(), "Alice");

// Update the value
name.set(String::from("Bob"));

// Update with a function
name.update(|s| s.push_str("!"));

// Replace and get old value
let old = name.replace(String::from("Charlie"));
```

**Important:** Values must implement `Hash`. The hash is used to detect whether the value actually changed - if you set the same value, dependents won't be notified.

### Computed - Derived Values

`Computed<T>` represents a value derived from other reactive values. It automatically tracks which `Var` or `Computed` values were accessed and recomputes only when those dependencies change.

```rust
use observe::rc::{Computed, Var};

let first_name = Var::new(String::from("John"));
let last_name = Var::new(String::from("Doe"));

let full_name = Computed::new(Box::new({
    let first_name = first_name.clone();
    let last_name = last_name.clone();
    move |cx| {
        format!("{} {}", first_name.get(cx), last_name.get(cx))
    }
}));

// Read the computed value
assert_eq!(*full_name.get_once(), "John Doe");

// When a dependency changes, the computed value updates
first_name.set(String::from("Jane"));
assert_eq!(*full_name.get_once(), "Jane Doe");
```

Computed values are **lazy** - they only recompute when accessed after a dependency changes.

### Reaction - Side Effects

`Reaction` executes side effects when its dependencies change. Unlike `Computed`, reactions don't return a value - they perform actions like updating the DOM, logging, or making network requests.

```rust
use observe::rc::{batch, Reaction, Var};

let temperature = Var::new(20);

let reaction = Reaction::new(Box::new({
    let temperature = temperature.clone();
    move |cx| {
        let temp = temperature.get(cx);
        if temp > 30 {
            println!("Warning: High temperature!");
        }
    }
}));

// Run the reaction initially
reaction.update();

// Reactions are triggered inside batch()
batch(|| {
    temperature.set(35);
});
// Prints: "Warning: High temperature!"
```

### The Evaluation Context

The `cx` parameter (of type `&Evaluation`) passed to closures is the key to automatic dependency tracking. When you call `.get(cx)` on a `Var` or `Computed`, it registers that value as a dependency.

```rust
// Dependency tracking happens through cx
let computed = Computed::new(Box::new(|cx| {
    let a = var_a.get(cx);  // var_a is now a dependency
    let b = var_b.get(cx);  // var_b is now a dependency
    a + b
}));

// Using get_once() does NOT track dependencies
let computed = Computed::new(Box::new(|cx| {
    let a = var_a.get_once();  // NOT tracked as dependency
    a + 10
}));
```

## Batching

The `batch()` function groups multiple state changes together. Reactions only run once after the batch completes, even if multiple dependencies changed.

```rust
use observe::rc::{batch, Reaction, Var};

let a = Var::new(1);
let b = Var::new(2);

let reaction = Reaction::new(Box::new({
    let a = a.clone();
    let b = b.clone();
    move |cx| {
        println!("Sum: {}", a.get(cx) + b.get(cx));
    }
}));

reaction.update();  // Prints: "Sum: 3"

// Without batching, this would trigger the reaction twice
// With batching, it only runs once at the end
batch(|| {
    a.set(10);
    b.set(20);
});
// Prints: "Sum: 30" (only once!)
```

**Note:** Reactions must be triggered inside a `batch()`. Calling `reaction.update()` outside a batch is allowed for initial setup, but subsequent automatic updates require batching.

## Change Detection

observe uses hash-based change detection. When you call `set()`, the new value's hash is compared to the old hash. If they match, no notifications are sent.

```rust
use observe::rc::{batch, Var};

let value = Var::new(42);

batch(|| {
    value.set(42);  // Same value - no reactions triggered
    value.set(42);  // Still the same - still no reactions
    value.set(43);  // Different! Reactions will run
});
```

This means your types must implement `Hash`:

```rust
use std::hash::Hash;

#[derive(Hash)]
struct User {
    id: u64,
    name: String,
}

let user = Var::new(User { id: 1, name: String::from("Alice") });
```

## API Reference

### Var<T>

| Method | Description |
|--------|-------------|
| `Var::new(value)` | Create a new reactive variable |
| `var.get(cx)` | Read value with dependency tracking (clones the value) |
| `var.get_ref(cx)` | Read value with dependency tracking (returns `Ref<T>`) |
| `var.get_once()` | Read value without tracking (clones the value) |
| `var.get_ref_once()` | Read value without tracking (returns `Ref<T>`) |
| `var.set(value)` | Set a new value |
| `var.replace(value)` | Set a new value, return the old one |
| `var.update(fn)` | Mutate the value with a function |
| `var.toggle()` | Toggle boolean values |
| `var.map(fn)` | Create a `Computed` that maps this value |

### Computed<T>

| Method | Description |
|--------|-------------|
| `Computed::new(fn)` | Create a new computed value |
| `computed.get(cx)` | Read value with dependency tracking |
| `computed.get_once()` | Read value without tracking |

### Reaction

| Method | Description |
|--------|-------------|
| `Reaction::new(fn)` | Create a new reaction |
| `Reaction::new_with_name(name, fn)` | Create a named reaction (useful for debugging) |
| `reaction.update()` | Run the reaction if invalid |
| `reaction.update_unchecked()` | Run the reaction unconditionally |

### Functions

| Function | Description |
|----------|-------------|
| `batch(fn)` | Execute a function, run affected reactions once at the end |
| `in_batch()` | Check if currently inside a batch |

## Thread Safety

The `observe::rc` module uses `Rc` and `RefCell`, making it suitable for single-threaded applications and WASM.

For multi-threaded applications, use `observe::arc` which provides the same API but uses `Arc` and `parking_lot` locks for thread safety. The `arc` module also includes `Async<T>` for async computations with tokio.

## License

MIT
