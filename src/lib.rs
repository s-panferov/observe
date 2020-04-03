mod computed;
mod context;
mod eval;

#[cfg(feature = "futures-03")]
pub mod future;

mod reaction;
mod tracker;
mod transaction;
mod types;
mod value;
mod variable;

#[cfg(test)]
pub mod test;

pub use observe_macro::*;

pub use crate::computed::Computed;
pub use crate::context::EvalContext;
pub use crate::eval::Evaluation;

pub use crate::reaction::Effect;
pub use crate::tracker::{Local, Shared, Tracker, WeakTracker};
pub use crate::transaction::{transaction, Transaction};
pub use crate::value::{Value, WeakValue};
pub use crate::variable::Var;

pub mod local {
    use super::EvalContext as _EvalContext;
    pub use super::Local;
    use super::Transaction as _Transaction;
    use super::Value as _Value;
    pub type Value<T> = _Value<T, Local>;
    pub type EvalContext = _EvalContext<Local>;
    pub type Transaction = _Transaction<Local>;
}

pub mod shared {
    use super::EvalContext as _EvalContext;
    pub use super::Shared;
    use super::Transaction as _Transaction;
    use super::Value as _Value;
    pub type Value<T> = _Value<T, Shared>;
    pub type EvalContext = _EvalContext<Shared>;
    pub type Transaction = _Transaction<Shared>;
}
