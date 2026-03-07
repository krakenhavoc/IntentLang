use thiserror::Error;

/// Errors that can occur during expression evaluation.
#[derive(Debug, Error, PartialEq)]
pub enum RuntimeError {
    #[error("unbound variable: {0}")]
    UnboundVariable(String),

    #[error("field '{field}' not found on value")]
    FieldNotFound { field: String },

    #[error("field access on non-object value")]
    NotAnObject,

    #[error("type error: expected {expected}, got {got}")]
    TypeError { expected: String, got: String },

    #[error("old() used outside postcondition context")]
    OldWithoutContext,

    #[error("no instances provided for type '{0}' in quantifier")]
    NoInstances(String),

    #[error("division by zero")]
    DivisionByZero,

    #[error("unknown function: {0}")]
    UnknownFunction(String),

    #[error("decimal arithmetic error: {0}")]
    DecimalError(String),
}
