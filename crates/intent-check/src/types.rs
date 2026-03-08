//! Type checking for intent specifications.
//!
//! Validates:
//! - No duplicate entity, action, or invariant names
//! - No duplicate fields within an entity or action
//! - All referenced types resolve to built-in types or user-defined entities
//! - Quantifier bindings reference valid types

use std::collections::HashMap;

use intent_parser::ast::{self, Span, TopLevelItem};

use crate::errors::CheckError;

/// Built-in primitive and domain types that don't need entity definitions.
const BUILTIN_TYPES: &[&str] = &[
    "UUID",
    "String",
    "Int",
    "Decimal",
    "Bool",
    "DateTime",
    // Domain types
    "CurrencyCode",
    "Email",
    "URL",
];

/// Collected type information and definitions from a parsed file.
#[derive(Debug, Default)]
pub struct TypeEnv {
    /// Entity name → (span, field map).
    pub entities: HashMap<String, (Span, Vec<(String, Span)>)>,
    /// Action name → (span, param names).
    pub actions: HashMap<String, (Span, Vec<String>)>,
    /// Invariant name → span.
    pub invariants: HashMap<String, Span>,
}

/// Run all semantic checks on a parsed file. Returns a list of diagnostics.
pub fn check_file(file: &ast::File) -> Vec<CheckError> {
    check_file_with_imports(file, &[])
}

/// Run all semantic checks on a parsed file with imported module definitions.
///
/// `imported_files` are the parsed ASTs of modules referenced by `use` declarations.
/// Their entities, actions, and invariants are added to the type environment so that
/// cross-module references resolve correctly.
pub fn check_file_with_imports(file: &ast::File, imported_files: &[&ast::File]) -> Vec<CheckError> {
    let mut errors = Vec::new();
    let mut env = TypeEnv::default();

    // Pass 0: Populate env with imported definitions.
    populate_imports(file, imported_files, &mut env, &mut errors);

    // Pass 1: Collect definitions, detect duplicates.
    collect_definitions(file, &mut env, &mut errors);

    // Pass 2: Resolve type references.
    check_type_references(file, &env, &mut errors);

    // Pass 3: Check quantifier binding types.
    check_quantifier_types(file, &env, &mut errors);

    // Pass 4: Check edge case action references.
    check_edge_case_actions(file, &env, &mut errors);

    // Pass 5: Check field access on known entity types.
    check_field_access(file, &env, &mut errors);

    // Pass 6: Constraint validation (old() placement, tautological comparisons).
    errors.extend(crate::constraints::check_constraints(file));

    errors
}

/// Populate the type environment with definitions from imported modules.
///
/// For whole-module imports (`use Foo`), all entities, actions, and invariants
/// from Foo are added. For selective imports (`use Foo.Bar`), only the named
/// item is added — if it doesn't exist, an error is reported.
fn populate_imports(
    file: &ast::File,
    imported_files: &[&ast::File],
    env: &mut TypeEnv,
    errors: &mut Vec<CheckError>,
) {
    // Build a lookup: module_name → parsed file
    let module_map: HashMap<&str, &&ast::File> = imported_files
        .iter()
        .map(|f| (f.module.name.as_str(), f))
        .collect();

    for use_decl in &file.imports {
        let Some(imported) = module_map.get(use_decl.module_name.as_str()) else {
            // Module not in the provided imports — the resolver should have caught this,
            // but if check_file_with_imports is called manually, skip gracefully.
            continue;
        };

        match &use_decl.item {
            None => {
                // Whole-module import: add all definitions.
                import_all_from(imported, env);
            }
            Some(item_name) => {
                // Selective import: add only the named item.
                if !import_item_from(imported, item_name, env) {
                    errors.push(CheckError::unresolved_import(
                        item_name,
                        &use_decl.module_name,
                        use_decl.span,
                    ));
                }
            }
        }
    }
}

/// Import all entities, actions, and invariants from a module into the env.
fn import_all_from(file: &ast::File, env: &mut TypeEnv) {
    for item in &file.items {
        match item {
            TopLevelItem::Entity(entity) => {
                let fields: Vec<(String, Span)> = entity
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), f.span))
                    .collect();
                env.entities
                    .entry(entity.name.clone())
                    .or_insert((entity.span, fields));
            }
            TopLevelItem::Action(action) => {
                let params: Vec<String> = action.params.iter().map(|p| p.name.clone()).collect();
                env.actions
                    .entry(action.name.clone())
                    .or_insert((action.span, params));
            }
            TopLevelItem::Invariant(inv) => {
                env.invariants.entry(inv.name.clone()).or_insert(inv.span);
            }
            TopLevelItem::EdgeCases(_) => {}
        }
    }
}

/// Import a single named item from a module. Returns true if the item was found.
fn import_item_from(file: &ast::File, item_name: &str, env: &mut TypeEnv) -> bool {
    for item in &file.items {
        match item {
            TopLevelItem::Entity(entity) if entity.name == item_name => {
                let fields: Vec<(String, Span)> = entity
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), f.span))
                    .collect();
                env.entities
                    .entry(entity.name.clone())
                    .or_insert((entity.span, fields));
                return true;
            }
            TopLevelItem::Action(action) if action.name == item_name => {
                let params: Vec<String> = action.params.iter().map(|p| p.name.clone()).collect();
                env.actions
                    .entry(action.name.clone())
                    .or_insert((action.span, params));
                return true;
            }
            TopLevelItem::Invariant(inv) if inv.name == item_name => {
                env.invariants.entry(inv.name.clone()).or_insert(inv.span);
                return true;
            }
            _ => {}
        }
    }
    false
}

/// Pass 1: Collect all entity, action, and invariant definitions.
/// Reports duplicates.
fn collect_definitions(file: &ast::File, env: &mut TypeEnv, errors: &mut Vec<CheckError>) {
    for item in &file.items {
        match item {
            TopLevelItem::Entity(entity) => {
                // Check duplicate entity name.
                if let Some((first_span, _)) = env.entities.get(&entity.name) {
                    errors.push(CheckError::duplicate_entity(
                        &entity.name,
                        *first_span,
                        entity.span,
                    ));
                } else {
                    // Collect fields, checking for duplicates within the entity.
                    let mut field_map: Vec<(String, Span)> = Vec::new();
                    let mut seen_fields: HashMap<String, Span> = HashMap::new();
                    for field in &entity.fields {
                        if let Some(&first_span) = seen_fields.get(&field.name) {
                            errors.push(CheckError::duplicate_field(
                                &field.name,
                                &entity.name,
                                first_span,
                                field.span,
                            ));
                        } else {
                            seen_fields.insert(field.name.clone(), field.span);
                            field_map.push((field.name.clone(), field.span));
                        }
                    }
                    env.entities
                        .insert(entity.name.clone(), (entity.span, field_map));
                }
            }
            TopLevelItem::Action(action) => {
                // Check duplicate action name.
                if let Some((first_span, _)) = env.actions.get(&action.name) {
                    errors.push(CheckError::duplicate_action(
                        &action.name,
                        *first_span,
                        action.span,
                    ));
                } else {
                    // Collect params, checking for duplicates.
                    let mut param_names: Vec<String> = Vec::new();
                    let mut seen_params: HashMap<String, Span> = HashMap::new();
                    for param in &action.params {
                        if let Some(&first_span) = seen_params.get(&param.name) {
                            errors.push(CheckError::duplicate_field(
                                &param.name,
                                &action.name,
                                first_span,
                                param.span,
                            ));
                        } else {
                            seen_params.insert(param.name.clone(), param.span);
                            param_names.push(param.name.clone());
                        }
                    }
                    env.actions
                        .insert(action.name.clone(), (action.span, param_names));
                }
            }
            TopLevelItem::Invariant(inv) => {
                if let Some(&first_span) = env.invariants.get(&inv.name) {
                    errors.push(CheckError::duplicate_invariant(
                        &inv.name, first_span, inv.span,
                    ));
                } else {
                    env.invariants.insert(inv.name.clone(), inv.span);
                }
            }
            TopLevelItem::EdgeCases(_) => {}
        }
    }
}

/// Pass 2: Check all type references in fields and parameters resolve.
fn check_type_references(file: &ast::File, env: &TypeEnv, errors: &mut Vec<CheckError>) {
    for item in &file.items {
        match item {
            TopLevelItem::Entity(entity) => {
                for field in &entity.fields {
                    check_type_expr(&field.ty, env, errors);
                }
            }
            TopLevelItem::Action(action) => {
                for param in &action.params {
                    check_type_expr(&param.ty, env, errors);
                }
            }
            _ => {}
        }
    }
}

/// Recursively check a type expression resolves.
fn check_type_expr(ty: &ast::TypeExpr, env: &TypeEnv, errors: &mut Vec<CheckError>) {
    check_type_kind(&ty.ty, ty.span, false, env, errors);
}

/// Check a TypeKind resolves. `span` is the enclosing TypeExpr span for error reporting.
/// `in_union` is true when we're inside a union — union variants are treated as enum labels,
/// not type references, so they don't need to resolve.
fn check_type_kind(
    kind: &ast::TypeKind,
    span: Span,
    in_union: bool,
    env: &TypeEnv,
    errors: &mut Vec<CheckError>,
) {
    match kind {
        ast::TypeKind::Simple(name) => {
            // Inside a union, simple names are enum-like variant labels (Active, Frozen, etc.)
            // and don't need to resolve to a defined type.
            if !in_union && !is_known_type(name, env) {
                errors.push(CheckError::undefined_type(name, span));
            }
        }
        ast::TypeKind::Union(variants) => {
            for v in variants {
                check_type_kind(v, span, true, env, errors);
            }
        }
        ast::TypeKind::List(inner) | ast::TypeKind::Set(inner) => {
            check_type_kind(&inner.ty, inner.span, false, env, errors);
        }
        ast::TypeKind::Map(key, value) => {
            check_type_kind(&key.ty, key.span, false, env, errors);
            check_type_kind(&value.ty, value.span, false, env, errors);
        }
        ast::TypeKind::Parameterized { name, .. } => {
            // The base type must be known (e.g. Decimal).
            if !is_known_type(name, env) {
                errors.push(CheckError::undefined_type(name, span));
            }
        }
    }
}

/// Check if a type name is a built-in or a user-defined entity.
fn is_known_type(name: &str, env: &TypeEnv) -> bool {
    BUILTIN_TYPES.contains(&name) || env.entities.contains_key(name)
}

/// Pass 3: Walk all expressions looking for quantifier bindings with invalid types.
fn check_quantifier_types(file: &ast::File, env: &TypeEnv, errors: &mut Vec<CheckError>) {
    for item in &file.items {
        match item {
            TopLevelItem::Action(action) => {
                if let Some(req) = &action.requires {
                    for cond in &req.conditions {
                        walk_expr_quantifiers(cond, env, errors);
                    }
                }
                if let Some(ens) = &action.ensures {
                    for item in &ens.items {
                        match item {
                            ast::EnsuresItem::Expr(e) => walk_expr_quantifiers(e, env, errors),
                            ast::EnsuresItem::When(w) => {
                                walk_expr_quantifiers(&w.condition, env, errors);
                                walk_expr_quantifiers(&w.consequence, env, errors);
                            }
                        }
                    }
                }
            }
            TopLevelItem::Invariant(inv) => {
                walk_expr_quantifiers(&inv.body, env, errors);
            }
            TopLevelItem::EdgeCases(ec) => {
                for rule in &ec.rules {
                    walk_expr_quantifiers(&rule.condition, env, errors);
                }
            }
            _ => {}
        }
    }
}

/// Pass 4: Check that actions referenced in edge_cases are defined.
/// Only validates uppercase-starting names (defined actions use `type_ident`).
/// Lowercase names like `reject`, `allow`, `require_approval` are convention-based
/// handlers and don't need to resolve to a defined action.
fn check_edge_case_actions(file: &ast::File, env: &TypeEnv, errors: &mut Vec<CheckError>) {
    for item in &file.items {
        if let TopLevelItem::EdgeCases(ec) = item {
            for rule in &ec.rules {
                let name = &rule.action.name;
                let is_defined_action_name = name.chars().next().is_some_and(|c| c.is_uppercase());
                if is_defined_action_name && !env.actions.contains_key(name) {
                    errors.push(CheckError::undefined_edge_action(name, rule.action.span));
                }
            }
        }
    }
}

/// Pass 5: Check field access on parameters with known entity types.
/// For each action, build a map of param_name -> entity_name, then walk
/// expressions to verify that `param.field` accesses valid entity fields.
fn check_field_access(file: &ast::File, env: &TypeEnv, errors: &mut Vec<CheckError>) {
    for item in &file.items {
        if let TopLevelItem::Action(action) = item {
            // Build param -> entity type map for this action
            let mut param_types: HashMap<String, String> = HashMap::new();
            for param in &action.params {
                if let ast::TypeKind::Simple(type_name) = &param.ty.ty
                    && env.entities.contains_key(type_name)
                {
                    param_types.insert(param.name.clone(), type_name.clone());
                }
            }
            if param_types.is_empty() {
                continue;
            }

            // Walk requires/ensures expressions
            if let Some(req) = &action.requires {
                for cond in &req.conditions {
                    walk_expr_field_access(cond, &param_types, env, errors);
                }
            }
            if let Some(ens) = &action.ensures {
                for item in &ens.items {
                    match item {
                        ast::EnsuresItem::Expr(e) => {
                            walk_expr_field_access(e, &param_types, env, errors);
                        }
                        ast::EnsuresItem::When(w) => {
                            walk_expr_field_access(&w.condition, &param_types, env, errors);
                            walk_expr_field_access(&w.consequence, &param_types, env, errors);
                        }
                    }
                }
            }
        }
    }
}

/// Walk an expression looking for field access on known entity-typed params.
fn walk_expr_field_access(
    expr: &ast::Expr,
    param_types: &HashMap<String, String>,
    env: &TypeEnv,
    errors: &mut Vec<CheckError>,
) {
    if let ast::ExprKind::FieldAccess { root, fields } = &expr.kind
        && let ast::ExprKind::Ident(name) = &root.kind
        && let Some(entity_name) = param_types.get(name)
        && let Some((_, entity_fields)) = env.entities.get(entity_name)
        && let Some(first_field) = fields.first()
    {
        let known = entity_fields.iter().any(|(n, _)| n == first_field);
        if !known {
            errors.push(CheckError::unknown_field(
                first_field,
                entity_name,
                expr.span,
            ));
        }
    }
    expr.for_each_child(|child| walk_expr_field_access(child, param_types, env, errors));
}

/// Recursively walk an expression tree checking quantifier binding types.
fn walk_expr_quantifiers(expr: &ast::Expr, env: &TypeEnv, errors: &mut Vec<CheckError>) {
    if let ast::ExprKind::Quantifier { ty, .. } = &expr.kind
        && !env.entities.contains_key(ty.as_str())
        && !env.actions.contains_key(ty.as_str())
    {
        errors.push(CheckError::undefined_quantifier_type(ty, expr.span));
    }
    expr.for_each_child(|child| walk_expr_quantifiers(child, env, errors));
}
