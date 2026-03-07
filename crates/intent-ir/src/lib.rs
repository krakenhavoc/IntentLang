pub mod audit;
pub mod lower;
pub mod types;
pub mod verify;

#[cfg(test)]
mod audit_tests;

#[cfg(test)]
mod lower_tests;

#[cfg(test)]
mod verify_tests;

pub use audit::{generate_audit, AuditReport};
pub use lower::lower_file;
pub use types::*;
pub use verify::{analyze_obligations, verify_module, Obligation, ObligationKind};
