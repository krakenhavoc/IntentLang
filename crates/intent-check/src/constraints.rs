//! Constraint validation for intent specifications.
//!
//! Validates:
//! - Requires/ensures blocks are not trivially contradictory
//! - All entities referenced in actions and invariants exist
//! - Completeness: no dangling references
