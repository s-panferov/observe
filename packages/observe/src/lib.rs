mod computed;
mod context;
mod effect;
mod hashed;
mod observable;

mod batch;
mod cons;
pub mod macros;
mod tracker;
mod value;
mod var;

#[cfg(feature = "futures-03")]
pub mod future;

#[cfg(test)]
mod test;

pub use batch::{batch, Batch};
pub use computed::Computed;
pub use cons::Const;
pub use context::EvalContext;
pub use effect::Effect;
pub use observable::{MutObservable, Observable};
pub use tracker::{Evaluation, Tracker, WeakTracker};
pub use value::Value;
pub use var::Var;

pub use observe_macro::*;

#[cfg(doctest)]
doc_comment::doctest!("../../../README.md");
