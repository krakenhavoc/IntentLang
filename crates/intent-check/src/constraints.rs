//! Constraint validation for intent specifications.
//!
//! Validates:
//! - `old()` is not used in `requires` blocks (only valid in `ensures`)
//! - No tautological self-comparisons (e.g., `x == x`)

use intent_parser::ast::{self, CmpOp, ExprKind, TopLevelItem};

use crate::errors::CheckError;

/// Run constraint validation checks on a parsed file.
pub fn check_constraints(file: &ast::File) -> Vec<CheckError> {
    let mut errors = Vec::new();

    for item in &file.items {
        match item {
            TopLevelItem::Action(action) => {
                if let Some(req) = &action.requires {
                    for cond in &req.conditions {
                        walk_for_old_in_requires(cond, &mut errors);
                        walk_for_tautological(cond, &mut errors);
                    }
                }
                if let Some(ens) = &action.ensures {
                    for ens_item in &ens.items {
                        match ens_item {
                            ast::EnsuresItem::Expr(e) => {
                                walk_for_tautological(e, &mut errors);
                            }
                            ast::EnsuresItem::When(w) => {
                                walk_for_tautological(&w.condition, &mut errors);
                                walk_for_tautological(&w.consequence, &mut errors);
                            }
                        }
                    }
                }
            }
            TopLevelItem::Invariant(inv) => {
                walk_for_tautological(&inv.body, &mut errors);
            }
            TopLevelItem::EdgeCases(ec) => {
                for rule in &ec.rules {
                    walk_for_tautological(&rule.condition, &mut errors);
                }
            }
            _ => {}
        }
    }

    errors
}

/// Walk an expression tree looking for `old()` calls in requires context.
fn walk_for_old_in_requires(expr: &ast::Expr, errors: &mut Vec<CheckError>) {
    if matches!(&expr.kind, ExprKind::Old(_)) {
        errors.push(CheckError::old_in_requires(expr.span));
    }
    expr.for_each_child(|child| walk_for_old_in_requires(child, errors));
}

/// Extract a path from a simple expression (ident or field access chain).
/// Returns None for complex expressions (calls, old(), arithmetic, etc.).
fn expr_to_path(expr: &ast::Expr) -> Option<Vec<String>> {
    match &expr.kind {
        ExprKind::Ident(name) => Some(vec![name.clone()]),
        ExprKind::FieldAccess { root, fields } => {
            let mut path = expr_to_path(root)?;
            path.extend(fields.iter().cloned());
            Some(path)
        }
        _ => None,
    }
}

/// Walk an expression tree looking for tautological self-comparisons.
fn walk_for_tautological(expr: &ast::Expr, errors: &mut Vec<CheckError>) {
    if let ExprKind::Compare { left, op, right } = &expr.kind
        && let (Some(left_path), Some(right_path)) = (expr_to_path(left), expr_to_path(right))
        && left_path == right_path
    {
        let path_str = left_path.join(".");
        let result = match op {
            CmpOp::Eq | CmpOp::Le | CmpOp::Ge => "true",
            CmpOp::Ne | CmpOp::Lt | CmpOp::Gt => "false",
        };
        errors.push(CheckError::tautological_comparison(
            &path_str, result, expr.span,
        ));
    }
    expr.for_each_child(|child| walk_for_tautological(child, errors));
}
