//! Stateless runtime for IntentLang specifications.
//!
//! Evaluates IR expressions against concrete JSON values, enabling
//! precondition/postcondition checking and invariant verification
//! at runtime.

mod contract;
mod error;
mod eval;
#[cfg(feature = "server")]
mod server;
mod value;

pub use contract::{ActionRequest, ActionResult, Violation, ViolationKind, execute_action};
pub use error::RuntimeError;
pub use eval::evaluate;
#[cfg(feature = "server")]
pub use server::serve;
pub use value::EvalContext;
