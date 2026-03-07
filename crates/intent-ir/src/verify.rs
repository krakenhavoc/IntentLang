//! IR verification pass.
//!
//! Validates structural and logical properties of the IR:
//! - All variable references in expressions are bound (params or quantifier bindings)
//!   (uppercase identifiers are treated as union variant labels and skipped)
//! - `old()` only appears in postconditions or temporal invariants
//! - Postconditions reference at least one parameter (otherwise they're trivially unverifiable)
//! - Quantifiers reference known types (structs or functions) in this module
//! - Functions with postconditions have at least one parameter (nothing to ensure about)

use std::collections::HashSet;

use crate::types::*;

/// A verification diagnostic.
#[derive(Debug, Clone)]
pub struct VerifyError {
    pub kind: VerifyErrorKind,
    pub trace: SourceTrace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyErrorKind {
    /// A variable is referenced but not bound as a parameter or quantifier binding.
    UnboundVariable { name: String },
    /// `old()` appears outside of a postcondition context.
    OldOutsidePoscondition,
    /// A function has postconditions but no parameters.
    PostconditionWithoutParams { function: String },
    /// A quantifier references a type not defined in this module (struct or function).
    UnknownQuantifierType { ty: String },
    /// A postcondition doesn't reference any function parameter.
    DisconnectedPostcondition { function: String },
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            VerifyErrorKind::UnboundVariable { name } => {
                write!(f, "unbound variable `{name}`")
            }
            VerifyErrorKind::OldOutsidePoscondition => {
                write!(f, "`old()` used outside of postcondition")
            }
            VerifyErrorKind::PostconditionWithoutParams { function } => {
                write!(
                    f,
                    "function `{function}` has postconditions but no parameters"
                )
            }
            VerifyErrorKind::UnknownQuantifierType { ty } => {
                write!(f, "quantifier references unknown type `{ty}`")
            }
            VerifyErrorKind::DisconnectedPostcondition { function } => {
                write!(
                    f,
                    "postcondition in `{function}` doesn't reference any parameter"
                )
            }
        }
    }
}

/// Run verification checks on an IR module.
pub fn verify_module(module: &Module) -> Vec<VerifyError> {
    let mut errors = Vec::new();

    // Known types for quantifiers: both structs and functions (actions).
    let known_types: HashSet<&str> = module
        .structs
        .iter()
        .map(|s| s.name.as_str())
        .chain(module.functions.iter().map(|f| f.name.as_str()))
        .collect();

    // Collect all names that appear in Call positions — these are domain-level
    // functions (now, lookup, etc.) and should be treated as implicitly bound.
    // Also includes names the parser may lower as Var instead of Call (e.g., `now()`
    // becomes Var("now") due to grammar limitations).
    let mut call_names = HashSet::new();
    collect_module_call_names(module, &mut call_names);

    for func in &module.functions {
        verify_function(func, &known_types, &call_names, &mut errors);
    }

    for inv in &module.invariants {
        verify_invariant(inv, &known_types, &call_names, &mut errors);
    }

    for guard in &module.edge_guards {
        verify_edge_guard(guard, &known_types, &mut errors);
    }

    errors
}

/// Collect all function names used in Call expressions across the module.
fn collect_module_call_names<'a>(module: &'a Module, names: &mut HashSet<&'a str>) {
    for func in &module.functions {
        for pre in &func.preconditions {
            collect_call_names(&pre.expr, names);
        }
        for post in &func.postconditions {
            match post {
                Postcondition::Always { expr, .. } => collect_call_names(expr, names),
                Postcondition::When { guard, expr, .. } => {
                    collect_call_names(guard, names);
                    collect_call_names(expr, names);
                }
            }
        }
    }
    for inv in &module.invariants {
        collect_call_names(&inv.expr, names);
    }
    for guard in &module.edge_guards {
        collect_call_names(&guard.condition, names);
        for (_, arg) in &guard.args {
            collect_call_names(arg, names);
        }
    }
}

fn collect_call_names<'a>(expr: &'a IrExpr, names: &mut HashSet<&'a str>) {
    if let IrExpr::Call { name, args } = expr {
        names.insert(name.as_str());
        for arg in args {
            collect_call_names(arg, names);
        }
        return;
    }
    match expr {
        IrExpr::Compare { left, right, .. }
        | IrExpr::Arithmetic { left, right, .. }
        | IrExpr::And(left, right)
        | IrExpr::Or(left, right)
        | IrExpr::Implies(left, right) => {
            collect_call_names(left, names);
            collect_call_names(right, names);
        }
        IrExpr::Not(inner) | IrExpr::Old(inner) => collect_call_names(inner, names),
        IrExpr::FieldAccess { root, .. } => collect_call_names(root, names),
        IrExpr::Forall { body, .. } | IrExpr::Exists { body, .. } => {
            collect_call_names(body, names);
        }
        IrExpr::Var(_) | IrExpr::Literal(_) | IrExpr::Call { .. } => {}
    }
}

fn verify_function(
    func: &Function,
    known_types: &HashSet<&str>,
    call_names: &HashSet<&str>,
    errors: &mut Vec<VerifyError>,
) {
    let param_names: HashSet<&str> = func.params.iter().map(|p| p.name.as_str()).collect();

    // Check: postconditions without parameters
    if !func.postconditions.is_empty() && func.params.is_empty() {
        errors.push(VerifyError {
            kind: VerifyErrorKind::PostconditionWithoutParams {
                function: func.name.clone(),
            },
            trace: func.trace.clone(),
        });
    }

    // Check preconditions: no old(), variables must be bound
    for pre in &func.preconditions {
        check_no_old(&pre.expr, &pre.trace, errors);
        check_bound_vars(&pre.expr, &param_names, &HashSet::new(), call_names, &pre.trace, errors);
    }

    // Check postconditions: variables must be bound, check param references
    for post in &func.postconditions {
        let (expr, trace) = match post {
            Postcondition::Always { expr, trace } => (expr, trace),
            Postcondition::When { guard, expr, trace } => {
                check_bound_vars(guard, &param_names, &HashSet::new(), call_names, trace, errors);
                (expr, trace)
            }
        };
        check_bound_vars(expr, &param_names, &HashSet::new(), call_names, trace, errors);

        // Check postcondition references at least one parameter
        let vars = collect_vars(expr);
        if !vars.iter().any(|v| param_names.contains(v.as_str())) {
            errors.push(VerifyError {
                kind: VerifyErrorKind::DisconnectedPostcondition {
                    function: func.name.clone(),
                },
                trace: trace.clone(),
            });
        }
    }

    // Check quantifier types in all expressions
    for pre in &func.preconditions {
        check_quantifier_types(&pre.expr, known_types, &pre.trace, errors);
    }
    for post in &func.postconditions {
        match post {
            Postcondition::Always { expr, trace } => {
                check_quantifier_types(expr, known_types, trace, errors);
            }
            Postcondition::When {
                guard, expr, trace, ..
            } => {
                check_quantifier_types(guard, known_types, trace, errors);
                check_quantifier_types(expr, known_types, trace, errors);
            }
        }
    }
}

fn verify_invariant(
    inv: &Invariant,
    known_types: &HashSet<&str>,
    call_names: &HashSet<&str>,
    errors: &mut Vec<VerifyError>,
) {
    // old() is valid in invariants for temporal properties (e.g., conservation laws)
    check_quantifier_types(&inv.expr, known_types, &inv.trace, errors);
    // Invariant body variables are bound by quantifiers, so we check with empty params
    check_bound_vars(
        &inv.expr,
        &HashSet::new(),
        &HashSet::new(),
        call_names,
        &inv.trace,
        errors,
    );
}

fn verify_edge_guard(guard: &EdgeGuard, known_types: &HashSet<&str>, errors: &mut Vec<VerifyError>) {
    check_no_old(&guard.condition, &guard.trace, errors);
    check_quantifier_types(&guard.condition, known_types, &guard.trace, errors);
    for (_, arg_expr) in &guard.args {
        check_no_old(arg_expr, &guard.trace, errors);
    }
}

// ── Expression walkers ──────────────────────────────────────

/// Check that `old()` does not appear in this expression.
fn check_no_old(expr: &IrExpr, trace: &SourceTrace, errors: &mut Vec<VerifyError>) {
    match expr {
        IrExpr::Old(_) => {
            errors.push(VerifyError {
                kind: VerifyErrorKind::OldOutsidePoscondition,
                trace: trace.clone(),
            });
        }
        _ => {
            for_each_child(expr, |child| check_no_old(child, trace, errors));
        }
    }
}

/// Check that all variable references are bound.
fn check_bound_vars(
    expr: &IrExpr,
    params: &HashSet<&str>,
    quantifier_bindings: &HashSet<&str>,
    call_names: &HashSet<&str>,
    trace: &SourceTrace,
    errors: &mut Vec<VerifyError>,
) {
    match expr {
        IrExpr::Var(name) => {
            // Uppercase identifiers are union variant labels (Active, Frozen, etc.)
            let is_variant = name.starts_with(|c: char| c.is_ascii_uppercase());
            // Names that appear as function calls elsewhere are domain-level references
            let is_call = call_names.contains(name.as_str());
            if !is_variant
                && !is_call
                && !params.contains(name.as_str())
                && !quantifier_bindings.contains(name.as_str())
            {
                errors.push(VerifyError {
                    kind: VerifyErrorKind::UnboundVariable { name: name.clone() },
                    trace: trace.clone(),
                });
            }
        }
        IrExpr::Forall { binding, body, .. } | IrExpr::Exists { binding, body, .. } => {
            let mut extended = quantifier_bindings.clone();
            extended.insert(binding.as_str());
            check_bound_vars(body, params, &extended, call_names, trace, errors);
        }
        _ => {
            for_each_child(expr, |child| {
                check_bound_vars(child, params, quantifier_bindings, call_names, trace, errors);
            });
        }
    }
}

/// Check that quantifier types reference known types (structs or functions).
fn check_quantifier_types(
    expr: &IrExpr,
    known_types: &HashSet<&str>,
    trace: &SourceTrace,
    errors: &mut Vec<VerifyError>,
) {
    match expr {
        IrExpr::Forall { ty, body, .. } | IrExpr::Exists { ty, body, .. } => {
            if !known_types.contains(ty.as_str()) {
                errors.push(VerifyError {
                    kind: VerifyErrorKind::UnknownQuantifierType { ty: ty.clone() },
                    trace: trace.clone(),
                });
            }
            check_quantifier_types(body, known_types, trace, errors);
        }
        _ => {
            for_each_child(expr, |child| {
                check_quantifier_types(child, known_types, trace, errors);
            });
        }
    }
}

/// Collect all variable names referenced in an expression.
fn collect_vars(expr: &IrExpr) -> Vec<String> {
    let mut vars = Vec::new();
    collect_vars_inner(expr, &mut vars);
    vars
}

fn collect_vars_inner(expr: &IrExpr, vars: &mut Vec<String>) {
    match expr {
        IrExpr::Var(name) => vars.push(name.clone()),
        _ => for_each_child(expr, |child| collect_vars_inner(child, vars)),
    }
}

/// Visit each immediate child of an IR expression.
fn for_each_child(expr: &IrExpr, mut f: impl FnMut(&IrExpr)) {
    match expr {
        IrExpr::Compare { left, right, .. }
        | IrExpr::Arithmetic { left, right, .. }
        | IrExpr::And(left, right)
        | IrExpr::Or(left, right)
        | IrExpr::Implies(left, right) => {
            f(left);
            f(right);
        }
        IrExpr::Not(inner) | IrExpr::Old(inner) => f(inner),
        IrExpr::FieldAccess { root, .. } => f(root),
        IrExpr::Forall { body, .. } | IrExpr::Exists { body, .. } => f(body),
        IrExpr::Call { args, .. } => {
            for arg in args {
                f(arg);
            }
        }
        IrExpr::Var(_) | IrExpr::Literal(_) => {}
    }
}
