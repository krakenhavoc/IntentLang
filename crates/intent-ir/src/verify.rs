//! IR verification pass.
//!
//! Validates structural and logical properties of the IR:
//! - All variable references in expressions are bound (params or quantifier bindings)
//!   (uppercase identifiers are treated as union variant labels and skipped)
//! - `old()` only appears in postconditions or temporal invariants
//! - Postconditions reference at least one parameter (otherwise they're trivially unverifiable)
//! - Quantifiers reference known types (structs or functions) in this module
//! - Functions with postconditions have at least one parameter (nothing to ensure about)
//!
//! Also performs coherence analysis:
//! - Extracts verification obligations (invariant-action relationships)
//! - Tracks which entity fields each action modifies (via `old()` in postconditions)
//! - Matches modified fields against invariant constraints

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::types::*;

/// Compute Levenshtein distance between two strings for fuzzy matching.
fn levenshtein_ir(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut dp: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut prev = dp[0];
        dp[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            let temp = dp[j];
            dp[j] = (dp[j] + 1).min(dp[j - 1] + 1).min(prev + cost);
            prev = temp;
        }
    }
    dp[b.len()]
}

/// Find the most similar name from candidates within edit distance 2.
fn find_similar_ir(name: &str, candidates: &[&str]) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    for &candidate in candidates {
        let dist = levenshtein_ir(name, candidate);
        if dist > 0 && dist <= 2 && (best.is_none() || dist < best.unwrap().0) {
            best = Some((dist, candidate));
        }
    }
    best.map(|(_, s)| s.to_string())
}

/// A verification diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyError {
    pub kind: VerifyErrorKind,
    pub trace: SourceTrace,
    /// Optional suggestion or note providing additional context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
                write!(f, "unbound variable `{name}`")?;
            }
            VerifyErrorKind::OldOutsidePoscondition => {
                write!(f, "`old()` used outside of postcondition")?;
            }
            VerifyErrorKind::PostconditionWithoutParams { function } => {
                write!(
                    f,
                    "function `{function}` has postconditions but no parameters"
                )?;
            }
            VerifyErrorKind::UnknownQuantifierType { ty } => {
                write!(f, "quantifier references unknown type `{ty}`")?;
            }
            VerifyErrorKind::DisconnectedPostcondition { function } => {
                write!(
                    f,
                    "postcondition in `{function}` doesn't reference any parameter"
                )?;
            }
        }
        if let Some(note) = &self.note {
            write!(f, " (note: {note})")?;
        }
        Ok(())
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

// ── Coherence analysis ─────────────────────────────────────

/// A verification obligation — something that needs to be proven
/// for the module to be correct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Obligation {
    /// The action (function) that triggers this obligation.
    pub action: String,
    /// The invariant that must be preserved.
    pub invariant: String,
    /// The entity type involved.
    pub entity: String,
    /// The specific fields that the action modifies and the invariant constrains.
    pub fields: Vec<String>,
    /// The kind of obligation.
    pub kind: ObligationKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObligationKind {
    /// Action modifies fields that an entity invariant constrains.
    /// The invariant quantifies over the entity type (e.g., `forall a: Account => ...`).
    InvariantPreservation,
    /// A temporal invariant directly references this action via quantifier
    /// (e.g., `forall t: Transfer => old(...) == ...`).
    TemporalProperty,
}

impl std::fmt::Display for Obligation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ObligationKind::InvariantPreservation => {
                write!(
                    f,
                    "{} modifies {}.{{{}}} (constrained by {})",
                    self.action,
                    self.entity,
                    self.fields.join(", "),
                    self.invariant,
                )
            }
            ObligationKind::TemporalProperty => {
                write!(
                    f,
                    "{} must satisfy temporal property {}",
                    self.action, self.invariant,
                )
            }
        }
    }
}

/// Analyze a verified IR module for verification obligations.
///
/// Returns a list of obligations that describe what logical properties
/// need to hold for the module to be correct. These are informational —
/// not errors — representing proof goals a formal verifier would check.
pub fn analyze_obligations(module: &Module) -> Vec<Obligation> {
    let mut obligations = Vec::new();

    // Build a map of struct name → field names for lookup.
    let struct_fields: HashMap<&str, Vec<&str>> = module
        .structs
        .iter()
        .map(|s| {
            (
                s.name.as_str(),
                s.fields.iter().map(|f| f.name.as_str()).collect(),
            )
        })
        .collect();

    // Build a map of param name → entity type for each function.
    // Only includes params whose type is an entity (struct).
    let func_entity_params: HashMap<&str, Vec<(&str, &str)>> = module
        .functions
        .iter()
        .map(|func| {
            let entity_params: Vec<(&str, &str)> = func
                .params
                .iter()
                .filter_map(|p| match &p.ty {
                    IrType::Named(t) | IrType::Struct(t)
                        if struct_fields.contains_key(t.as_str()) =>
                    {
                        Some((p.name.as_str(), t.as_str()))
                    }
                    _ => None,
                })
                .collect();
            (func.name.as_str(), entity_params)
        })
        .collect();

    // For each function, collect fields modified in postconditions (via old()).
    // Result: function name → set of (entity_type, field_name).
    let mut modified_fields: HashMap<&str, HashSet<(&str, &str)>> = HashMap::new();
    for func in &module.functions {
        let entity_params = &func_entity_params[func.name.as_str()];
        let param_to_entity: HashMap<&str, &str> = entity_params.iter().copied().collect();
        let mut fields = HashSet::new();
        for post in &func.postconditions {
            let exprs: Vec<&IrExpr> = match post {
                Postcondition::Always { expr, .. } => vec![expr],
                Postcondition::When { guard, expr, .. } => vec![guard, expr],
            };
            for expr in exprs {
                collect_old_field_accesses(expr, &param_to_entity, &mut fields);
            }
        }
        modified_fields.insert(func.name.as_str(), fields);
    }

    // For each invariant, determine what it constrains.
    for inv in &module.invariants {
        if let IrExpr::Forall { binding, ty, body } = &inv.expr {
            // Check if this is a temporal invariant (quantifies over an action).
            let is_action = module.functions.iter().any(|f| f.name == *ty);
            if is_action {
                // Temporal property: directly references an action.
                obligations.push(Obligation {
                    action: ty.clone(),
                    invariant: inv.name.clone(),
                    entity: ty.clone(),
                    fields: vec![],
                    kind: ObligationKind::TemporalProperty,
                });
                continue;
            }

            // Entity invariant: quantifies over an entity type.
            // Collect fields the invariant constrains.
            let constrained = collect_field_accesses_on(body, binding);

            // Find all actions that modify any of these fields on this entity type.
            for func in &module.functions {
                if let Some(mods) = modified_fields.get(func.name.as_str()) {
                    let overlapping: Vec<String> = constrained
                        .iter()
                        .filter(|f| mods.contains(&(ty.as_str(), f.as_str())))
                        .cloned()
                        .collect();
                    if !overlapping.is_empty() {
                        obligations.push(Obligation {
                            action: func.name.clone(),
                            invariant: inv.name.clone(),
                            entity: ty.clone(),
                            fields: overlapping,
                            kind: ObligationKind::InvariantPreservation,
                        });
                    }
                }
            }
        }
    }

    obligations
}

/// Collect field accesses inside `old()` expressions, mapping them to entity types
/// via the param→entity mapping.
///
/// For an expression like `old(from.balance)`, if `from` maps to entity `Account`,
/// this records `("Account", "balance")`.
fn collect_old_field_accesses<'a>(
    expr: &'a IrExpr,
    param_to_entity: &HashMap<&str, &'a str>,
    result: &mut HashSet<(&'a str, &'a str)>,
) {
    match expr {
        IrExpr::Old(inner) => {
            collect_inner_field_accesses(inner, param_to_entity, result);
        }
        _ => {
            // Manual match for recursion (avoid lifetime issues with for_each_child closure)
            match expr {
                IrExpr::Compare { left, right, .. }
                | IrExpr::Arithmetic { left, right, .. }
                | IrExpr::And(left, right)
                | IrExpr::Or(left, right)
                | IrExpr::Implies(left, right) => {
                    collect_old_field_accesses(left, param_to_entity, result);
                    collect_old_field_accesses(right, param_to_entity, result);
                }
                IrExpr::Not(inner) => {
                    collect_old_field_accesses(inner, param_to_entity, result);
                }
                IrExpr::FieldAccess { root, .. } => {
                    collect_old_field_accesses(root, param_to_entity, result);
                }
                IrExpr::Forall { body, .. } | IrExpr::Exists { body, .. } => {
                    collect_old_field_accesses(body, param_to_entity, result);
                }
                IrExpr::Call { args, .. } | IrExpr::List(args) => {
                    for arg in args {
                        collect_old_field_accesses(arg, param_to_entity, result);
                    }
                }
                IrExpr::Var(_) | IrExpr::Literal(_) | IrExpr::Old(_) => {}
            }
        }
    }
}

/// Collect field accesses within an old() body, resolving param names to entity types.
fn collect_inner_field_accesses<'a>(
    expr: &'a IrExpr,
    param_to_entity: &HashMap<&str, &'a str>,
    result: &mut HashSet<(&'a str, &'a str)>,
) {
    match expr {
        IrExpr::FieldAccess { root, field } => {
            // Check if root is a direct param reference: old(param.field)
            if let IrExpr::Var(var) = root.as_ref()
                && let Some(&entity) = param_to_entity.get(var.as_str())
            {
                result.insert((entity, field.as_str()));
            }
            // Also check for chained access: old(param.sub.field)
            collect_inner_field_accesses(root, param_to_entity, result);
        }
        _ => match expr {
            IrExpr::Compare { left, right, .. }
            | IrExpr::Arithmetic { left, right, .. }
            | IrExpr::And(left, right)
            | IrExpr::Or(left, right)
            | IrExpr::Implies(left, right) => {
                collect_inner_field_accesses(left, param_to_entity, result);
                collect_inner_field_accesses(right, param_to_entity, result);
            }
            IrExpr::Not(inner) | IrExpr::Old(inner) => {
                collect_inner_field_accesses(inner, param_to_entity, result);
            }
            IrExpr::FieldAccess { .. } => unreachable!(),
            IrExpr::Forall { body, .. } | IrExpr::Exists { body, .. } => {
                collect_inner_field_accesses(body, param_to_entity, result);
            }
            IrExpr::Call { args, .. } | IrExpr::List(args) => {
                for arg in args {
                    collect_inner_field_accesses(arg, param_to_entity, result);
                }
            }
            IrExpr::Var(_) | IrExpr::Literal(_) => {}
        },
    }
}

/// Collect field names accessed on a specific binding variable in an expression.
///
/// For `forall a: Account => a.balance >= 0`, calling this with binding="a"
/// returns `["balance"]`.
fn collect_field_accesses_on(expr: &IrExpr, binding: &str) -> Vec<String> {
    let mut fields = Vec::new();
    collect_fields_on_inner(expr, binding, &mut fields);
    fields.sort();
    fields.dedup();
    fields
}

fn collect_fields_on_inner(expr: &IrExpr, binding: &str, fields: &mut Vec<String>) {
    match expr {
        IrExpr::FieldAccess { root, field } => {
            if let IrExpr::Var(var) = root.as_ref()
                && var == binding
            {
                fields.push(field.clone());
            }
            collect_fields_on_inner(root, binding, fields);
        }
        _ => for_each_child(expr, |child| {
            collect_fields_on_inner(child, binding, fields)
        }),
    }
}

// ── Structural verification helpers ────────────────────────

/// Collect all function names used in Call expressions across the module.
pub(crate) fn collect_module_call_names<'a>(module: &'a Module, names: &mut HashSet<&'a str>) {
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
        IrExpr::List(items) => {
            for item in items {
                collect_call_names(item, names);
            }
        }
        IrExpr::Var(_) | IrExpr::Literal(_) | IrExpr::Call { .. } => {}
    }
}

pub(crate) fn verify_function(
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
            note: None,
        });
    }

    // Check preconditions: no old(), variables must be bound
    for pre in &func.preconditions {
        check_no_old(&pre.expr, &pre.trace, errors);
        check_bound_vars(
            &pre.expr,
            &param_names,
            &HashSet::new(),
            call_names,
            &pre.trace,
            errors,
        );
    }

    // Check postconditions: variables must be bound, check param references
    for post in &func.postconditions {
        let (expr, trace) = match post {
            Postcondition::Always { expr, trace } => (expr, trace),
            Postcondition::When { guard, expr, trace } => {
                check_bound_vars(
                    guard,
                    &param_names,
                    &HashSet::new(),
                    call_names,
                    trace,
                    errors,
                );
                (expr, trace)
            }
        };
        check_bound_vars(
            expr,
            &param_names,
            &HashSet::new(),
            call_names,
            trace,
            errors,
        );

        // Check postcondition references at least one parameter
        let vars = collect_vars(expr);
        if !vars.iter().any(|v| param_names.contains(v.as_str())) {
            let param_list: Vec<&str> = param_names.iter().copied().collect();
            let note = if param_list.is_empty() {
                "a postcondition must reference at least one action parameter".to_string()
            } else {
                format!(
                    "a postcondition must describe how action parameters change — available parameters: {}",
                    param_list.join(", ")
                )
            };
            errors.push(VerifyError {
                kind: VerifyErrorKind::DisconnectedPostcondition {
                    function: func.name.clone(),
                },
                trace: trace.clone(),
                note: Some(note),
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

pub(crate) fn verify_invariant(
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

pub(crate) fn verify_edge_guard(
    guard: &EdgeGuard,
    known_types: &HashSet<&str>,
    errors: &mut Vec<VerifyError>,
) {
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
                note: None,
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
                // Build suggestion from available bindings
                let mut candidates: Vec<&str> = params.iter().copied().collect();
                candidates.extend(quantifier_bindings.iter().copied());
                let note =
                    find_similar_ir(name, &candidates).map(|s| format!("did you mean `{s}`?"));
                errors.push(VerifyError {
                    kind: VerifyErrorKind::UnboundVariable { name: name.clone() },
                    trace: trace.clone(),
                    note,
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
                check_bound_vars(
                    child,
                    params,
                    quantifier_bindings,
                    call_names,
                    trace,
                    errors,
                );
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
                let candidates: Vec<&str> = known_types.iter().copied().collect();
                let note = find_similar_ir(ty, &candidates).map(|s| format!("did you mean `{s}`?"));
                errors.push(VerifyError {
                    kind: VerifyErrorKind::UnknownQuantifierType { ty: ty.clone() },
                    trace: trace.clone(),
                    note,
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
        IrExpr::Call { args, .. } | IrExpr::List(args) => {
            for arg in args {
                f(arg);
            }
        }
        IrExpr::Var(_) | IrExpr::Literal(_) => {}
    }
}
