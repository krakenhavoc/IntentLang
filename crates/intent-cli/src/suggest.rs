//! Static analysis suggestions for `.intent` spec files.
//!
//! Walks the parsed AST and produces actionable suggestions for improving
//! spec quality: missing invariants, missing properties, unused entities,
//! missing contracts, and missing edge cases.

use intent_parser::ast::{
    ActionDecl, EnsuresItem, EntityDecl, ExprKind, File, InvariantDecl, TopLevelItem, TypeKind,
};
use serde::Serialize;

// ── Public types ──────────────────────────────────────────

/// A single suggestion produced by static analysis.
#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    /// The broad category of the suggestion.
    pub category: SuggestionCategory,
    /// Info or warning severity.
    pub severity: Severity,
    /// Short human-readable title.
    pub title: String,
    /// Longer description explaining the suggestion.
    pub description: String,
    /// Suggested fix text (valid IntentLang syntax, if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_fix: Option<String>,
}

/// Broad categories for suggestions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionCategory {
    MissingInvariant,
    MissingProperty,
    MissingEdgeCase,
    SpecCompleteness,
}

/// Severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
}

/// Result returned by `analyze`.
#[derive(Debug, Clone, Serialize)]
pub struct SuggestResult {
    pub suggestions: Vec<Suggestion>,
    pub count: usize,
}

// ── Public API ────────────────────────────────────────────

/// Analyze an AST and return all suggestions.
pub fn analyze(file: &File) -> SuggestResult {
    let entities = collect_entities(file);
    let actions = collect_actions(file);
    let invariants = collect_invariants(file);

    let mut suggestions = Vec::new();

    suggestions.extend(check_missing_numeric_invariants(&entities, &invariants));
    suggestions.extend(check_missing_uniqueness_invariants(&entities, &invariants));
    suggestions.extend(check_missing_ref_invariants(&entities, &invariants));
    suggestions.extend(check_missing_action_properties(&actions));
    suggestions.extend(check_unused_entities(&entities, &actions));
    suggestions.extend(check_missing_contracts(&actions));
    suggestions.extend(check_missing_edge_cases(&actions, file));

    let count = suggestions.len();
    SuggestResult { suggestions, count }
}

// ── Collectors ────────────────────────────────────────────

fn collect_entities(file: &File) -> Vec<&EntityDecl> {
    file.items
        .iter()
        .filter_map(|item| match item {
            TopLevelItem::Entity(e) => Some(e),
            _ => None,
        })
        .collect()
}

fn collect_actions(file: &File) -> Vec<&ActionDecl> {
    file.items
        .iter()
        .filter_map(|item| match item {
            TopLevelItem::Action(a) => Some(a),
            _ => None,
        })
        .collect()
}

fn collect_invariants(file: &File) -> Vec<&InvariantDecl> {
    file.items
        .iter()
        .filter_map(|item| match item {
            TopLevelItem::Invariant(i) => Some(i),
            _ => None,
        })
        .collect()
}

// ── Analysis passes ───────────────────────────────────────

/// Check for numeric fields (Decimal, Int) without non-negative invariants.
fn check_missing_numeric_invariants(
    entities: &[&EntityDecl],
    invariants: &[&InvariantDecl],
) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();

    for entity in entities {
        for field in &entity.fields {
            let is_numeric = is_numeric_type(&field.ty.ty);
            if !is_numeric {
                continue;
            }

            // Check if any invariant already references entity.field >= 0
            let has_invariant = invariants.iter().any(|inv| {
                invariant_constrains_field_non_negative(&inv.body, &entity.name, &field.name)
            });

            if !has_invariant {
                let inv_name = format!("NonNegative{}{}", entity.name, capitalize(&field.name));
                let var = entity
                    .name
                    .chars()
                    .next()
                    .unwrap_or('x')
                    .to_ascii_lowercase();
                suggestions.push(Suggestion {
                    category: SuggestionCategory::MissingInvariant,
                    severity: Severity::Warning,
                    title: format!("Missing invariant: {inv_name}"),
                    description: format!(
                        "Entity `{}` has numeric field `{}` with no non-negative constraint.",
                        entity.name, field.name
                    ),
                    suggested_fix: Some(format!(
                        "invariant {inv_name} {{\n  forall {var}: {} => {var}.{} >= 0\n}}",
                        entity.name, field.name
                    )),
                });
            }
        }
    }

    suggestions
}

/// Check for UUID/id fields without uniqueness invariants.
fn check_missing_uniqueness_invariants(
    entities: &[&EntityDecl],
    invariants: &[&InvariantDecl],
) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();

    for entity in entities {
        // Look for fields that are UUID type or named "id"
        let has_id_field = entity
            .fields
            .iter()
            .any(|f| f.name == "id" || matches!(&f.ty.ty, TypeKind::Simple(t) if t == "UUID"));

        if !has_id_field {
            continue;
        }

        // Check if any invariant enforces uniqueness for this entity
        let has_uniqueness = invariants
            .iter()
            .any(|inv| invariant_enforces_uniqueness(&inv.body, &entity.name));

        if !has_uniqueness {
            let inv_name = format!("Unique{}Id", entity.name);
            suggestions.push(Suggestion {
                category: SuggestionCategory::MissingInvariant,
                severity: Severity::Info,
                title: format!("Missing invariant: {inv_name}"),
                description: format!(
                    "Entity `{}` has an `id` field but no uniqueness invariant.",
                    entity.name
                ),
                suggested_fix: Some(format!(
                    "invariant {inv_name} {{\n  forall a: {e} => forall b: {e} =>\n    a.id != b.id => a != b\n}}",
                    e = entity.name
                )),
            });
        }
    }

    suggestions
}

/// Check for entity-typed fields (foreign key pattern) without referential integrity invariants.
fn check_missing_ref_invariants(
    entities: &[&EntityDecl],
    invariants: &[&InvariantDecl],
) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();
    let entity_names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

    for entity in entities {
        for field in &entity.fields {
            let referenced_entity = match &field.ty.ty {
                TypeKind::Simple(name) if entity_names.contains(&name.as_str()) => name,
                _ => continue,
            };

            // Skip self-references
            if referenced_entity == &entity.name {
                continue;
            }

            // Check if any invariant constrains this reference
            let has_ref_invariant = invariants
                .iter()
                .any(|inv| invariant_references_field(&inv.body, &entity.name, &field.name));

            if !has_ref_invariant {
                let inv_name = format!("Valid{}{}Ref", entity.name, capitalize(&field.name));
                let var = entity
                    .name
                    .chars()
                    .next()
                    .unwrap_or('x')
                    .to_ascii_lowercase();
                suggestions.push(Suggestion {
                    category: SuggestionCategory::MissingInvariant,
                    severity: Severity::Info,
                    title: format!("Missing invariant: {inv_name}"),
                    description: format!(
                        "Entity `{}` has field `{}` typed as `{}` (foreign key) but no referential integrity invariant.",
                        entity.name, field.name, referenced_entity
                    ),
                    suggested_fix: Some(format!(
                        "invariant {inv_name} {{\n  forall {var}: {} => exists ref: {} => {var}.{} == ref\n}}",
                        entity.name, referenced_entity, field.name
                    )),
                });
            }
        }
    }

    suggestions
}

/// Check for actions that should have properties like atomic, idempotent, audit_logged.
fn check_missing_action_properties(actions: &[&ActionDecl]) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();

    for action in actions {
        let has_atomic = has_property(action, "atomic");
        let has_audit_logged = has_property(action, "audit_logged");
        let has_idempotent = has_property(action, "idempotent");

        // Check if action modifies financial/balance fields
        let modifies_balance = action_references_financial_field(action);
        if modifies_balance {
            if !has_atomic {
                suggestions.push(Suggestion {
                    category: SuggestionCategory::MissingProperty,
                    severity: Severity::Warning,
                    title: format!("Missing property: {}.atomic", action.name),
                    description: format!(
                        "Action `{}` modifies financial fields but is not marked atomic.",
                        action.name
                    ),
                    suggested_fix: Some("Add `atomic: true` to the properties block.".to_string()),
                });
            }
            if !has_audit_logged {
                suggestions.push(Suggestion {
                    category: SuggestionCategory::MissingProperty,
                    severity: Severity::Warning,
                    title: format!("Missing property: {}.audit_logged", action.name),
                    description: format!(
                        "Action `{}` modifies financial fields but is not audit-logged.",
                        action.name
                    ),
                    suggested_fix: Some(
                        "Add `audit_logged: true` to the properties block.".to_string(),
                    ),
                });
            }
        }

        // If action has both requires and ensures, suggest idempotent
        if action.requires.is_some() && action.ensures.is_some() && !has_idempotent {
            suggestions.push(Suggestion {
                category: SuggestionCategory::MissingProperty,
                severity: Severity::Info,
                title: format!("Missing property: {}.idempotent", action.name),
                description: format!(
                    "Action `{}` has both requires and ensures blocks. Consider marking it as idempotent or explicitly non-idempotent.",
                    action.name
                ),
                suggested_fix: Some(
                    "Add `idempotent: true` (or `idempotent: false`) to the properties block."
                        .to_string(),
                ),
            });
        }
    }

    suggestions
}

/// Check for entities not referenced by any action.
fn check_unused_entities(entities: &[&EntityDecl], actions: &[&ActionDecl]) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();

    for entity in entities {
        let is_referenced = actions.iter().any(|action| {
            // Check action params
            let in_params = action
                .params
                .iter()
                .any(|p| type_references_entity(&p.ty.ty, &entity.name));
            if in_params {
                return true;
            }

            // Check ensures items for entity references
            if let Some(ensures) = &action.ensures {
                for item in &ensures.items {
                    let expr = match item {
                        EnsuresItem::Expr(e) => e,
                        EnsuresItem::When(w) => &w.consequence,
                    };
                    if expr_references_entity(expr, &entity.name) {
                        return true;
                    }
                }
            }

            // Check requires
            if let Some(requires) = &action.requires {
                for cond in &requires.conditions {
                    if expr_references_entity(cond, &entity.name) {
                        return true;
                    }
                }
            }

            false
        });

        // Also check if referenced by other entities (as a field type)
        let referenced_by_entity = entities.iter().any(|other| {
            other.name != entity.name
                && other
                    .fields
                    .iter()
                    .any(|f| type_references_entity(&f.ty.ty, &entity.name))
        });

        if !is_referenced && !referenced_by_entity {
            suggestions.push(Suggestion {
                category: SuggestionCategory::SpecCompleteness,
                severity: Severity::Warning,
                title: format!("Unused entity: {}", entity.name),
                description: format!(
                    "Entity `{}` is not referenced by any action or other entity.",
                    entity.name
                ),
                suggested_fix: None,
            });
        }
    }

    suggestions
}

/// Check for actions missing requires or ensures blocks.
fn check_missing_contracts(actions: &[&ActionDecl]) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();

    for action in actions {
        if action.ensures.is_none() {
            suggestions.push(Suggestion {
                category: SuggestionCategory::SpecCompleteness,
                severity: Severity::Warning,
                title: format!("Missing ensures: {}", action.name),
                description: format!(
                    "Action `{}` has no ensures block — its postconditions are unspecified.",
                    action.name
                ),
                suggested_fix: None,
            });
        }

        if action.requires.is_none() {
            suggestions.push(Suggestion {
                category: SuggestionCategory::SpecCompleteness,
                severity: Severity::Info,
                title: format!("Missing requires: {}", action.name),
                description: format!(
                    "Action `{}` has no requires block — consider adding preconditions.",
                    action.name
                ),
                suggested_fix: None,
            });
        }
    }

    suggestions
}

/// Check for numeric params without bound checks in edge cases.
fn check_missing_edge_cases(actions: &[&ActionDecl], file: &File) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();

    // Collect existing edge_cases condition patterns
    let existing_edge_conditions = collect_edge_case_patterns(file);

    for action in actions {
        for param in &action.params {
            if !is_numeric_type(&param.ty.ty) {
                continue;
            }

            // Check if there's already an upper-bound edge case for this param
            let has_upper_bound = existing_edge_conditions.iter().any(|pat| {
                pat.contains(&param.name) && (pat.contains('>') || pat.contains("upper"))
            });

            // Also check if requires block already has an upper bound
            let has_requires_bound = if let Some(requires) = &action.requires {
                requires
                    .conditions
                    .iter()
                    .any(|cond| expr_has_upper_bound(cond, &param.name))
            } else {
                false
            };

            if !has_upper_bound && !has_requires_bound {
                suggestions.push(Suggestion {
                    category: SuggestionCategory::MissingEdgeCase,
                    severity: Severity::Info,
                    title: format!(
                        "Missing edge case: {} {} upper bound",
                        action.name, param.name
                    ),
                    description: format!(
                        "Action `{}` has numeric parameter `{}` with no upper bound check.",
                        action.name, param.name
                    ),
                    suggested_fix: Some(format!(
                        "when {} > 10000 => require_approval(level: \"manager\")",
                        param.name
                    )),
                });
            }
        }
    }

    suggestions
}

// ── Helpers ───────────────────────────────────────────────

fn is_numeric_type(ty: &TypeKind) -> bool {
    match ty {
        TypeKind::Simple(name) => matches!(name.as_str(), "Int" | "Decimal"),
        TypeKind::Parameterized { name, .. } => name == "Decimal",
        _ => false,
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn has_property(action: &ActionDecl, key: &str) -> bool {
    action
        .properties
        .as_ref()
        .is_some_and(|props| props.entries.iter().any(|e| e.key == key))
}

/// Check if an action references financial fields (balance, amount, price, total, cost).
fn action_references_financial_field(action: &ActionDecl) -> bool {
    let financial_fields = ["balance", "amount", "price", "total", "cost"];

    // Check ensures block for financial field references
    if let Some(ensures) = &action.ensures {
        for item in &ensures.items {
            let expr = match item {
                EnsuresItem::Expr(e) => e,
                EnsuresItem::When(w) => &w.consequence,
            };
            if expr_references_any_field(expr, &financial_fields) {
                return true;
            }
        }
    }

    // Check action params for financial names
    if action.params.iter().any(|p| {
        financial_fields.contains(&p.name.as_str())
            || is_numeric_type(&p.ty.ty) && p.name.contains("amount")
    }) {
        return true;
    }

    false
}

/// Check if a type references a specific entity name.
fn type_references_entity(ty: &TypeKind, entity_name: &str) -> bool {
    match ty {
        TypeKind::Simple(name) => name == entity_name,
        TypeKind::Union(variants) => variants
            .iter()
            .any(|v| type_references_entity(v, entity_name)),
        TypeKind::List(inner) | TypeKind::Set(inner) => {
            type_references_entity(&inner.ty, entity_name)
        }
        TypeKind::Map(k, v) => {
            type_references_entity(&k.ty, entity_name) || type_references_entity(&v.ty, entity_name)
        }
        TypeKind::Parameterized { .. } => false,
    }
}

/// Check if an expression references an entity type (in quantifiers, type positions).
fn expr_references_entity(expr: &intent_parser::ast::Expr, entity_name: &str) -> bool {
    match &expr.kind {
        ExprKind::Quantifier { ty, body, .. } => {
            ty == entity_name || expr_references_entity(body, entity_name)
        }
        ExprKind::Ident(name) => name == entity_name,
        _ => {
            let mut found = false;
            expr.for_each_child(|child| {
                if expr_references_entity(child, entity_name) {
                    found = true;
                }
            });
            found
        }
    }
}

/// Check if an expression references any of the given field names (via field access).
fn expr_references_any_field(expr: &intent_parser::ast::Expr, field_names: &[&str]) -> bool {
    match &expr.kind {
        ExprKind::FieldAccess { fields, root, .. } => {
            if fields.iter().any(|f| field_names.contains(&f.as_str())) {
                return true;
            }
            expr_references_any_field(root, field_names)
        }
        ExprKind::Ident(name) => field_names.contains(&name.as_str()),
        _ => {
            let mut found = false;
            expr.for_each_child(|child| {
                if expr_references_any_field(child, field_names) {
                    found = true;
                }
            });
            found
        }
    }
}

/// Check if an invariant body constrains `entity.field >= 0`.
fn invariant_constrains_field_non_negative(
    expr: &intent_parser::ast::Expr,
    entity_name: &str,
    field_name: &str,
) -> bool {
    match &expr.kind {
        ExprKind::Quantifier { ty, body, .. } if ty == entity_name => {
            body_constrains_non_negative(body, field_name)
        }
        _ => {
            let mut found = false;
            expr.for_each_child(|child| {
                if invariant_constrains_field_non_negative(child, entity_name, field_name) {
                    found = true;
                }
            });
            found
        }
    }
}

/// Check if the body of a quantifier constrains `var.field >= 0`.
fn body_constrains_non_negative(expr: &intent_parser::ast::Expr, field_name: &str) -> bool {
    match &expr.kind {
        ExprKind::Compare { left, op, right } => {
            use intent_parser::ast::CmpOp;
            match op {
                CmpOp::Ge => {
                    // field >= 0
                    field_access_matches(left, field_name)
                        && matches!(
                            &right.kind,
                            ExprKind::Literal(intent_parser::ast::Literal::Int(0))
                        )
                }
                CmpOp::Le => {
                    // 0 <= field
                    matches!(
                        &left.kind,
                        ExprKind::Literal(intent_parser::ast::Literal::Int(0))
                    ) && field_access_matches(right, field_name)
                }
                _ => false,
            }
        }
        ExprKind::Implies(_, body) => body_constrains_non_negative(body, field_name),
        _ => {
            let mut found = false;
            expr.for_each_child(|child| {
                if body_constrains_non_negative(child, field_name) {
                    found = true;
                }
            });
            found
        }
    }
}

/// Check if an expression is a field access ending in `field_name`.
fn field_access_matches(expr: &intent_parser::ast::Expr, field_name: &str) -> bool {
    match &expr.kind {
        ExprKind::FieldAccess { fields, .. } => fields.last().is_some_and(|f| f == field_name),
        _ => false,
    }
}

/// Check if an invariant body enforces uniqueness for the given entity.
fn invariant_enforces_uniqueness(expr: &intent_parser::ast::Expr, entity_name: &str) -> bool {
    // Pattern: forall a: Entity => forall b: Entity => a.id != b.id => ...
    match &expr.kind {
        ExprKind::Quantifier { ty, body, .. } if ty == entity_name => {
            // Check for nested forall on same type
            match &body.kind {
                ExprKind::Quantifier { ty: ty2, .. } if ty2 == entity_name => true,
                ExprKind::Implies(_, _) => true, // Reasonable uniqueness pattern
                _ => false,
            }
        }
        _ => {
            let mut found = false;
            expr.for_each_child(|child| {
                if invariant_enforces_uniqueness(child, entity_name) {
                    found = true;
                }
            });
            found
        }
    }
}

/// Check if an invariant references a specific entity field.
fn invariant_references_field(
    expr: &intent_parser::ast::Expr,
    entity_name: &str,
    field_name: &str,
) -> bool {
    match &expr.kind {
        ExprKind::Quantifier { ty, body, .. } if ty == entity_name => {
            body_references_field(body, field_name)
        }
        _ => {
            let mut found = false;
            expr.for_each_child(|child| {
                if invariant_references_field(child, entity_name, field_name) {
                    found = true;
                }
            });
            found
        }
    }
}

fn body_references_field(expr: &intent_parser::ast::Expr, field_name: &str) -> bool {
    match &expr.kind {
        ExprKind::FieldAccess { fields, .. } => fields.iter().any(|f| f == field_name),
        _ => {
            let mut found = false;
            expr.for_each_child(|child| {
                if body_references_field(child, field_name) {
                    found = true;
                }
            });
            found
        }
    }
}

/// Collect simple string patterns from edge_cases for dedup.
fn collect_edge_case_patterns(file: &File) -> Vec<String> {
    let mut patterns = Vec::new();
    for item in &file.items {
        if let TopLevelItem::EdgeCases(ec) = item {
            for rule in &ec.rules {
                patterns.push(format_expr_rough(&rule.condition));
            }
        }
    }
    patterns
}

/// Rough textual representation of an expression for pattern matching.
fn format_expr_rough(expr: &intent_parser::ast::Expr) -> String {
    match &expr.kind {
        ExprKind::Ident(name) => name.clone(),
        ExprKind::FieldAccess { root, fields } => {
            let base = format_expr_rough(root);
            let chain = fields.join(".");
            format!("{base}.{chain}")
        }
        ExprKind::Compare { left, right, op } => {
            let op_str = match op {
                intent_parser::ast::CmpOp::Eq => "==",
                intent_parser::ast::CmpOp::Ne => "!=",
                intent_parser::ast::CmpOp::Lt => "<",
                intent_parser::ast::CmpOp::Gt => ">",
                intent_parser::ast::CmpOp::Le => "<=",
                intent_parser::ast::CmpOp::Ge => ">=",
            };
            format!(
                "{} {} {}",
                format_expr_rough(left),
                op_str,
                format_expr_rough(right)
            )
        }
        ExprKind::Literal(lit) => match lit {
            intent_parser::ast::Literal::Int(n) => n.to_string(),
            intent_parser::ast::Literal::Decimal(s) => s.clone(),
            intent_parser::ast::Literal::String(s) => format!("\"{s}\""),
            intent_parser::ast::Literal::Bool(b) => b.to_string(),
            intent_parser::ast::Literal::Null => "null".to_string(),
        },
        _ => String::new(),
    }
}

/// Check if an expression has an upper bound on a parameter.
fn expr_has_upper_bound(expr: &intent_parser::ast::Expr, param_name: &str) -> bool {
    match &expr.kind {
        ExprKind::Compare { left, op, right } => {
            use intent_parser::ast::CmpOp;
            match op {
                CmpOp::Lt | CmpOp::Le => {
                    // param < N or param <= N
                    field_or_ident_matches(left, param_name)
                }
                CmpOp::Gt | CmpOp::Ge => {
                    // N > param or N >= param
                    field_or_ident_matches(right, param_name)
                }
                _ => false,
            }
        }
        _ => {
            let mut found = false;
            expr.for_each_child(|child| {
                if expr_has_upper_bound(child, param_name) {
                    found = true;
                }
            });
            found
        }
    }
}

fn field_or_ident_matches(expr: &intent_parser::ast::Expr, name: &str) -> bool {
    matches!(&expr.kind, ExprKind::Ident(n) if n == name)
}

// ── Formatting ────────────────────────────────────────────

/// Format suggestions for human-readable output.
pub fn format_human(result: &SuggestResult, file_path: &str, module_name: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("Analyzing {file_path}...\n\n"));

    if result.suggestions.is_empty() {
        out.push_str(&format!("No suggestions for {module_name}\n"));
        return out;
    }

    out.push_str(&format!("Found {} suggestion(s):\n\n", result.count));

    for (i, s) in result.suggestions.iter().enumerate() {
        out.push_str(&format!("[{}] {}\n", i + 1, s.title));
        out.push_str(&format!("    {}\n", s.description));
        if let Some(fix) = &s.suggested_fix {
            out.push_str("    Suggested fix:\n");
            for line in fix.lines() {
                out.push_str(&format!("    {line}\n"));
            }
        }
        out.push('\n');
    }

    out.push_str(&format!(
        "OK: {} suggestion(s) for {module_name}\n",
        result.count
    ));
    out
}

// ── Tests ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> File {
        intent_parser::parse_file(source).expect("parse failed")
    }

    #[test]
    fn detects_missing_numeric_invariant() {
        let ast = parse(
            "module Test\n\
             entity Account {\n  id: UUID\n  balance: Decimal(precision: 2)\n}\n\
             action Noop {\n  x: Int\n}\n",
        );
        let result = analyze(&ast);
        assert!(
            result
                .suggestions
                .iter()
                .any(|s| s.category == SuggestionCategory::MissingInvariant
                    && s.title.contains("NonNegativeAccountBalance")),
            "expected NonNegativeAccountBalance suggestion, got: {:?}",
            result
                .suggestions
                .iter()
                .map(|s| &s.title)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn no_numeric_invariant_when_already_present() {
        let ast = parse(
            "module Test\n\
             entity Account {\n  id: UUID\n  balance: Decimal(precision: 2)\n}\n\
             invariant NonNeg {\n  forall a: Account => a.balance >= 0\n}\n\
             action Noop {\n  x: Int\n}\n",
        );
        let result = analyze(&ast);
        assert!(
            !result
                .suggestions
                .iter()
                .any(|s| s.title.contains("NonNegativeAccountBalance")),
            "should not suggest non-negative invariant when one exists"
        );
    }

    #[test]
    fn detects_missing_uniqueness_invariant() {
        let ast = parse(
            "module Test\n\
             entity Item {\n  id: UUID\n  name: String\n}\n\
             action Noop {\n  x: Int\n}\n",
        );
        let result = analyze(&ast);
        assert!(
            result
                .suggestions
                .iter()
                .any(|s| s.category == SuggestionCategory::MissingInvariant
                    && s.title.contains("UniqueItemId")),
            "expected UniqueItemId suggestion"
        );
    }

    #[test]
    fn detects_unused_entity() {
        let ast = parse(
            "module Test\n\
             entity Used {\n  id: UUID\n}\n\
             entity Unused {\n  id: UUID\n}\n\
             action DoStuff {\n  item: Used\n}\n",
        );
        let result = analyze(&ast);
        assert!(
            result
                .suggestions
                .iter()
                .any(|s| s.category == SuggestionCategory::SpecCompleteness
                    && s.title.contains("Unused entity: Unused")),
            "expected unused entity suggestion for Unused"
        );
        assert!(
            !result
                .suggestions
                .iter()
                .any(|s| s.title.contains("Unused entity: Used")),
            "Used entity should not be flagged as unused"
        );
    }

    #[test]
    fn detects_missing_ensures() {
        let ast = parse(
            "module Test\n\
             entity Item {\n  id: UUID\n}\n\
             action NoPost {\n  item: Item\n  requires {\n    item.id != \"\"\n  }\n}\n",
        );
        let result = analyze(&ast);
        assert!(
            result
                .suggestions
                .iter()
                .any(|s| s.category == SuggestionCategory::SpecCompleteness
                    && s.title.contains("Missing ensures: NoPost")),
            "expected missing ensures suggestion"
        );
    }

    #[test]
    fn detects_missing_requires() {
        let ast = parse(
            "module Test\n\
             entity Item {\n  id: UUID\n}\n\
             action NoPre {\n  item: Item\n  ensures {\n    item.id != \"\"\n  }\n}\n",
        );
        let result = analyze(&ast);
        assert!(
            result
                .suggestions
                .iter()
                .any(|s| s.category == SuggestionCategory::SpecCompleteness
                    && s.title.contains("Missing requires: NoPre")),
            "expected missing requires suggestion"
        );
    }

    #[test]
    fn detects_missing_idempotent_property() {
        let ast = parse(
            "module Test\n\
             entity Item {\n  id: UUID\n}\n\
             action DoStuff {\n  item: Item\n  \
             requires {\n    item.id != \"\"\n  }\n  \
             ensures {\n    item.id == \"done\"\n  }\n}\n",
        );
        let result = analyze(&ast);
        assert!(
            result
                .suggestions
                .iter()
                .any(|s| s.category == SuggestionCategory::MissingProperty
                    && s.title.contains("idempotent")),
            "expected idempotent property suggestion"
        );
    }

    #[test]
    fn detects_missing_atomic_for_financial() {
        let ast = parse(
            "module Test\n\
             entity Account {\n  id: UUID\n  balance: Decimal(precision: 2)\n}\n\
             action Transfer {\n  from: Account\n  to: Account\n  amount: Decimal(precision: 2)\n  \
             requires {\n    from.balance >= amount\n  }\n  \
             ensures {\n    from.balance == old(from.balance) - amount\n  }\n}\n",
        );
        let result = analyze(&ast);
        assert!(
            result
                .suggestions
                .iter()
                .any(|s| s.category == SuggestionCategory::MissingProperty
                    && s.title.contains("atomic")),
            "expected atomic property suggestion for financial action"
        );
    }

    #[test]
    fn well_specified_module_produces_fewer_suggestions() {
        let ast = parse(
            "module Test\n\
             entity Account {\n  id: UUID\n  balance: Decimal(precision: 2)\n}\n\
             invariant NonNeg {\n  forall a: Account => a.balance >= 0\n}\n\
             invariant UniqueId {\n  forall a: Account => forall b: Account => a.id != b.id => a != b\n}\n\
             action Transfer {\n  from: Account\n  amount: Decimal(precision: 2)\n  \
             requires {\n    from.balance >= amount\n    amount <= 100000\n  }\n  \
             ensures {\n    from.balance == old(from.balance) - amount\n  }\n  \
             properties {\n    atomic: true\n    audit_logged: true\n    idempotent: false\n  }\n}\n",
        );
        let result = analyze(&ast);
        // Should not have numeric invariant or uniqueness suggestions
        assert!(
            !result
                .suggestions
                .iter()
                .any(|s| s.title.contains("NonNegative")),
            "should not suggest non-negative when invariant exists"
        );
        assert!(
            !result
                .suggestions
                .iter()
                .any(|s| s.title.contains("Unique")),
            "should not suggest uniqueness when invariant exists"
        );
    }

    #[test]
    fn json_output_format() {
        let ast = parse(
            "module Test\n\
             entity Item {\n  id: UUID\n}\n\
             action Noop {\n  x: Int\n}\n",
        );
        let result = analyze(&ast);
        let json = serde_json::to_string(&result).expect("serialize failed");
        assert!(json.contains("\"suggestions\""));
        assert!(json.contains("\"count\""));
    }

    #[test]
    fn detects_missing_ref_invariant() {
        let ast = parse(
            "module Test\n\
             entity User {\n  id: UUID\n}\n\
             entity Order {\n  id: UUID\n  user: User\n}\n\
             action Noop {\n  x: Int\n}\n",
        );
        let result = analyze(&ast);
        assert!(
            result
                .suggestions
                .iter()
                .any(|s| s.category == SuggestionCategory::MissingInvariant
                    && s.title.contains("Ref")),
            "expected referential integrity invariant suggestion"
        );
    }

    #[test]
    fn detects_missing_edge_case_upper_bound() {
        let ast = parse(
            "module Test\n\
             entity Account {\n  id: UUID\n}\n\
             action Transfer {\n  from: Account\n  amount: Decimal(precision: 2)\n  \
             requires {\n    amount > 0\n  }\n  \
             ensures {\n    from.id != \"\"\n  }\n}\n",
        );
        let result = analyze(&ast);
        assert!(
            result
                .suggestions
                .iter()
                .any(|s| s.category == SuggestionCategory::MissingEdgeCase
                    && s.title.contains("upper bound")),
            "expected upper bound edge case suggestion"
        );
    }

    #[test]
    fn format_human_output() {
        let ast = parse(
            "module Test\n\
             entity Item {\n  id: UUID\n  count: Int\n}\n\
             action Noop {\n  x: Int\n}\n",
        );
        let result = analyze(&ast);
        let output = format_human(&result, "test.intent", "Test");
        assert!(output.contains("Analyzing test.intent"));
        assert!(output.contains("[1]"));
        assert!(output.contains("suggestion(s) for Test"));
    }

    #[test]
    fn empty_module_no_crash() {
        let ast = parse("module Empty\n");
        let result = analyze(&ast);
        assert_eq!(result.count, 0);
    }
}
