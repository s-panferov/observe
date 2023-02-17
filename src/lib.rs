pub mod macros;

mod addr;
mod batch;
mod computed;
mod r#const;
mod dependencies;
mod evaluation;
mod hashed;
mod reaction;
mod value;
mod var;

use std::rc::{Rc, Weak};

pub use batch::{batch, in_batch};
pub use computed::Computed;
pub use dependencies::Dependencies;
pub use evaluation::Evaluation;
pub use hashed::Hashed;
pub use reaction::{Reaction, Reactions, Reactive, CHANGED};
pub use value::Value;
pub use var::Var;

pub trait Derived: 'static {
	fn invalidate(self: Rc<Self>, invalid: Invalid);
}

pub trait Observable: 'static {
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
