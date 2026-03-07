use serde_json::Value;
use std::collections::HashMap;

/// Context for expression evaluation.
///
/// Holds variable bindings (action parameters, quantifier bindings),
/// optional old-state bindings for `old()` references in postconditions,
/// and entity instances for quantifier iteration.
#[derive(Debug, Clone, Default)]
pub struct EvalContext {
    /// Current variable bindings (params, quantifier bindings).
    pub bindings: HashMap<String, Value>,
    /// Old-state bindings for `old()` references. If `None`, `old()` is invalid.
    pub old_bindings: Option<HashMap<String, Value>>,
    /// Entity instances by type name, for `forall`/`exists` evaluation.
    pub instances: HashMap<String, Vec<Value>>,
}

impl EvalContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a child context with an additional binding (for quantifier evaluation).
    pub fn with_binding(&self, name: String, value: Value) -> Self {
        let mut child = self.clone();
        child.bindings.insert(name, value);
        child
    }
}
