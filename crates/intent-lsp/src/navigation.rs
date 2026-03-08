//! Go-to-definition: resolve symbol at cursor to its declaration.

use std::collections::HashMap;

use intent_parser::ast::{self, ExprKind, TopLevelItem, TypeKind};
use tower_lsp::lsp_types::{GotoDefinitionResponse, Location, Range, Url};

use crate::document::Document;

/// Symbol definition entry.
struct SymbolDef {
    range: Range,
}

/// Try to find the definition of the symbol at the given byte offset.
pub fn goto_definition(doc: &Document, offset: usize, uri: &Url) -> Option<GotoDefinitionResponse> {
    let ast = doc.ast.as_ref()?;

    // Build a symbol table of all definitions.
    let symbols = collect_definitions(ast, &doc.source, doc);

    // Find what name is at the cursor position.
    let name = find_reference_at(ast, offset, &doc.source)?;

    // Look up the definition.
    let def = symbols.get(&name)?;

    Some(GotoDefinitionResponse::Scalar(Location {
        uri: uri.clone(),
        range: def.range,
    }))
}

/// Collect all entity, action, and invariant name definitions.
fn collect_definitions(
    ast: &ast::File,
    source: &str,
    doc: &Document,
) -> HashMap<String, SymbolDef> {
    let mut symbols = HashMap::new();

    for item in &ast.items {
        match item {
            TopLevelItem::Entity(e) => {
                symbols.insert(
                    e.name.clone(),
                    SymbolDef {
                        range: doc.line_index.span_to_range(e.span, source),
                    },
                );
            }
            TopLevelItem::Action(a) => {
                symbols.insert(
                    a.name.clone(),
                    SymbolDef {
                        range: doc.line_index.span_to_range(a.span, source),
                    },
                );
            }
            TopLevelItem::Invariant(i) => {
                symbols.insert(
                    i.name.clone(),
                    SymbolDef {
                        range: doc.line_index.span_to_range(i.span, source),
                    },
                );
            }
            TopLevelItem::EdgeCases(_) => {}
        }
    }

    symbols
}

/// Find the name of the symbol referenced at the given byte offset.
///
/// Walks the AST looking for type references, identifiers, and action calls
/// whose span contains the cursor offset.
fn find_reference_at(ast: &ast::File, offset: usize, source: &str) -> Option<String> {
    // Check type references in entity fields and action params.
    for item in &ast.items {
        match item {
            TopLevelItem::Entity(e) => {
                for field in &e.fields {
                    if let Some(name) = find_type_at(&field.ty, offset) {
                        return Some(name);
                    }
                }
            }
            TopLevelItem::Action(a) => {
                for param in &a.params {
                    if let Some(name) = find_type_at(&param.ty, offset) {
                        return Some(name);
                    }
                }
                // Check expressions in requires/ensures blocks.
                if let Some(ref req) = a.requires {
                    for expr in &req.conditions {
                        if let Some(name) = find_ident_in_expr(expr, offset) {
                            return Some(name);
                        }
                    }
                }
                if let Some(ref ens) = a.ensures {
                    for item in &ens.items {
                        match item {
                            ast::EnsuresItem::Expr(expr) => {
                                if let Some(name) = find_ident_in_expr(expr, offset) {
                                    return Some(name);
                                }
                            }
                            ast::EnsuresItem::When(w) => {
                                if let Some(name) = find_ident_in_expr(&w.condition, offset) {
                                    return Some(name);
                                }
                                if let Some(name) = find_ident_in_expr(&w.consequence, offset) {
                                    return Some(name);
                                }
                            }
                        }
                    }
                }
            }
            TopLevelItem::Invariant(inv) => {
                if let Some(name) = find_ident_in_expr(&inv.body, offset) {
                    return Some(name);
                }
            }
            TopLevelItem::EdgeCases(ec) => {
                for rule in &ec.rules {
                    // Check the action call name.
                    if contains_offset(rule.action.span, offset) {
                        return Some(rule.action.name.clone());
                    }
                    if let Some(name) = find_ident_in_expr(&rule.condition, offset) {
                        return Some(name);
                    }
                }
            }
        }
    }

    // Check entity/action/invariant name references at declarations —
    // clicking a declaration name should also work.
    for item in &ast.items {
        let (name, span) = match item {
            TopLevelItem::Entity(e) => (&e.name, e.span),
            TopLevelItem::Action(a) => (&a.name, a.span),
            TopLevelItem::Invariant(i) => (&i.name, i.span),
            TopLevelItem::EdgeCases(_) => continue,
        };
        // Check if cursor is on the declaration name (approximate: within the
        // first line of the span, after the keyword).
        let name_start = source[span.start..].find(name.as_str());
        if let Some(rel) = name_start {
            let abs_start = span.start + rel;
            let abs_end = abs_start + name.len();
            if offset >= abs_start && offset <= abs_end {
                return Some(name.clone());
            }
        }
    }

    None
}

/// Check if a type expression at the given offset contains a named type.
fn find_type_at(ty: &ast::TypeExpr, offset: usize) -> Option<String> {
    if !contains_offset(ty.span, offset) {
        return None;
    }
    match &ty.ty {
        TypeKind::Simple(name) => Some(name.clone()),
        TypeKind::List(inner) | TypeKind::Set(inner) => find_type_at(inner, offset),
        TypeKind::Map(k, v) => find_type_at(k, offset).or_else(|| find_type_at(v, offset)),
        TypeKind::Union(variants) => {
            // Union variants are labels, not entity references.
            // Still return the name under cursor for potential lookup.
            for v in variants {
                if let TypeKind::Simple(name) = v {
                    return Some(name.clone());
                }
            }
            None
        }
        TypeKind::Parameterized { name, .. } => Some(name.clone()),
    }
}

/// Find a PascalCase identifier in an expression at the given offset.
/// Only returns names that start with uppercase (type/entity references).
fn find_ident_in_expr(expr: &ast::Expr, offset: usize) -> Option<String> {
    if !contains_offset(expr.span, offset) {
        return None;
    }

    match &expr.kind {
        ExprKind::Ident(name) if name.starts_with(char::is_uppercase) => {
            return Some(name.clone());
        }
        ExprKind::Quantifier { ty, .. } => {
            // Check if cursor is on the type name in the binding.
            // The type name is somewhere in the span of this quantifier.
            if ty.starts_with(char::is_uppercase) {
                return Some(ty.clone());
            }
        }
        ExprKind::Call { name, .. } if name.starts_with(char::is_uppercase) => {
            return Some(name.clone());
        }
        _ => {}
    }

    // Recurse into children.
    let mut result = None;
    expr.for_each_child(|child| {
        if result.is_none() {
            result = find_ident_in_expr(child, offset);
        }
    });
    result
}

fn contains_offset(span: ast::Span, offset: usize) -> bool {
    offset >= span.start && offset < span.end
}
