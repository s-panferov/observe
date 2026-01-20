mod addr;
mod r#async;
mod async2;
mod batch;
mod computed;
mod r#const;
mod dependencies;
mod evaluation;
mod reaction;
mod value;
mod var;

#[cfg(target_arch = "wasm32")]
mod microtask;

use std::sync::{Arc, Weak};

pub use batch::{batch, batch_microtask, in_batch};
pub use computed::Computed;
pub use dependencies::Dependencies;
pub use evaluation::Evaluation;
pub use r#async::Async;
pub use r#async2::Async as Async2;
pub use reaction::{Reaction, Reactions, Reactive, CHANGED};
pub use value::Value;
pub use var::Var;

pub trait Derived: Send + Sync + 'static {
	fn invalidate(self: Arc<Self>, invalid: Invalid);
}

pub trait Observable: 'static + Send + Sync {
	/// This function is called when we want
	/// this observable to recompute itself.
	fn update(&self) -> Version;

	/// This function should return the current
	/// computed version.
	fn version(&self) -> Version;

	/// Notify this observable that `derived` started
	/// to listen.
	fn used_by(&self, derived: Weak<dyn Derived>);

	/// Notify this observable that `derived` stopped
	/// to listen.
	fn not_used_by(&self, derived: &Weak<dyn Derived>);
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum State {
	Valid,
	Invalid(Invalid),
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Invalid {
	Maybe,
	Definitely,
}

#[derive(PartialEq, Eq)]
pub enum Version {
	Hash(u64),
}
