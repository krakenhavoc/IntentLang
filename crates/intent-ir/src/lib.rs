pub mod audit;
pub mod diff;
pub mod lower;
pub mod types;
pub mod verify;

#[cfg(test)]
mod audit_tests;

#[cfg(test)]
mod diff_tests;

#[cfg(test)]
mod lower_tests;

#[cfg(test)]
mod verify_tests;

pub use audit::{AuditReport, SpecItemKind, generate_audit};
pub use diff::{DiffReport, diff_reports};
pub use lower::lower_file;
pub use types::*;
pub use verify::{Obligation, ObligationKind, VerifyError, analyze_obligations, verify_module};
