//! Python skeleton code generator.

use intent_parser::ast;

use crate::types::map_type;
use crate::{Language, doc_text, format_ensures_item, format_expr, to_snake_case};

/// Python reserved keywords that cannot be used as identifiers.
const PYTHON_KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

/// Escape a Python identifier if it collides with a reserved keyword.
fn safe_ident(name: &str) -> String {
    let snake = to_snake_case(name);
    if PYTHON_KEYWORDS.contains(&snake.as_str()) {
        format!("{snake}_")
    } else {
        snake
    }
}

/// Generate Python skeleton code from a parsed intent file.
pub fn generate(file: &ast::File) -> String {
    let lang = Language::Python;
    let mut out = String::new();

    // Header
    out.push_str(&format!("# Generated from {}.intent\n", file.module.name));
    if let Some(doc) = &file.doc {
        out.push_str(&format!("\"\"\"{}\"\"\"", doc_text(doc)));
        out.push('\n');
    }
    out.push('\n');

    // Imports
    out.push_str(&generate_imports(file));
    out.push('\n');

    for item in &file.items {
        match item {
            ast::TopLevelItem::Entity(e) => generate_entity(&mut out, e, &lang),
            ast::TopLevelItem::Action(a) => generate_action(&mut out, a, &lang),
            ast::TopLevelItem::Invariant(inv) => generate_invariant(&mut out, inv),
            ast::TopLevelItem::EdgeCases(ec) => generate_edge_cases(&mut out, ec),
            ast::TopLevelItem::StateMachine(sm) => generate_state_machine(&mut out, sm),
            ast::TopLevelItem::Test(_) => {}
        }
    }

    out
}

fn generate_state_machine(out: &mut String, sm: &ast::StateMachineDecl) {
    if let Some(doc) = &sm.doc {
        out.push_str(&format!("\"\"\"{}.\"\"\"\n\n\n", doc_text(doc).trim()));
    }
    // StrEnum class
    out.push_str("from enum import StrEnum\n\n\n");
    out.push_str(&format!("class {}(StrEnum):\n", sm.name));
    for state in &sm.states {
        let upper = to_screaming_snake(state);
        out.push_str(&format!("    {} = \"{}\"\n", upper, state));
    }
    out.push('\n');

    // Transition validation
    out.push_str(&format!(
        "    @staticmethod\n    def is_valid_transition(from_state: \"{}\", to_state: \"{}\") -> bool:\n",
        sm.name, sm.name
    ));
    out.push_str("        valid = {\n");
    let mut transition_map: std::collections::HashMap<&str, Vec<&str>> =
        std::collections::HashMap::new();
    for (from, to) in &sm.transitions {
        transition_map
            .entry(from.as_str())
            .or_default()
            .push(to.as_str());
    }
    for state in &sm.states {
        let targets = transition_map
            .get(state.as_str())
            .map(|v| {
                v.iter()
                    .map(|t| format!("\"{}\"", t))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        let upper = to_screaming_snake(state);
        out.push_str(&format!(
            "            {}.{}: [{}],\n",
            sm.name, upper, targets
        ));
    }
    out.push_str("        }\n");
    out.push_str("        return to_state.value in valid.get(from_state, [])\n\n\n");
}

fn to_screaming_snake(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_ascii_uppercase());
    }
    result
}

fn generate_imports(file: &ast::File) -> String {
    let mut out = String::from("from __future__ import annotations\n\n");
    let source = collect_type_names(file);

    out.push_str("from dataclasses import dataclass\n");

    if source.contains("Decimal") {
        out.push_str("from decimal import Decimal\n");
    }
    if source.contains("DateTime") {
        out.push_str("from datetime import datetime\n");
    }
    if source.contains("UUID") {
        out.push_str("import uuid\n");
    }

    // Check for union types (Literal needed)
    let has_union = file.items.iter().any(|item| {
        if let ast::TopLevelItem::Entity(e) = item {
            e.fields
                .iter()
                .any(|f| matches!(f.ty.ty, ast::TypeKind::Union(_)))
        } else {
            false
        }
    });
    if has_union {
        out.push_str("from typing import Literal\n");
    }

    out.push('\n');
    out
}

fn collect_type_names(file: &ast::File) -> String {
    let mut names = String::new();
    for item in &file.items {
        match item {
            ast::TopLevelItem::Entity(e) => {
                for f in &e.fields {
                    collect_type_name(&f.ty, &mut names);
                }
            }
            ast::TopLevelItem::Action(a) => {
                for p in &a.params {
                    collect_type_name(&p.ty, &mut names);
                }
            }
            _ => {}
        }
    }
    names
}

fn collect_type_name(ty: &ast::TypeExpr, out: &mut String) {
    match &ty.ty {
        ast::TypeKind::Simple(n) => {
            out.push_str(n);
            out.push(' ');
        }
        ast::TypeKind::Parameterized { name, .. } => {
            out.push_str(name);
            out.push(' ');
        }
        ast::TypeKind::List(inner) | ast::TypeKind::Set(inner) => collect_type_name(inner, out),
        ast::TypeKind::Map(k, v) => {
            collect_type_name(k, out);
            collect_type_name(v, out);
        }
        ast::TypeKind::Union(_) => {} // union doesn't affect stdlib imports
    }
}

fn generate_entity(out: &mut String, entity: &ast::EntityDecl, lang: &Language) {
    out.push_str("@dataclass\n");
    out.push_str(&format!("class {}:\n", entity.name));

    // Docstring
    if let Some(doc) = &entity.doc {
        out.push_str(&format!("    \"\"\"{}\"\"\"\n\n", doc_text(doc)));
    }

    for field in &entity.fields {
        let ty = map_type(&field.ty, lang);
        out.push_str(&format!("    {}: {}\n", safe_ident(&field.name), ty));
    }

    out.push('\n');
    out.push('\n');
}

fn generate_action(out: &mut String, action: &ast::ActionDecl, lang: &Language) {
    let fn_name = to_snake_case(&action.name);
    let params: Vec<String> = action
        .params
        .iter()
        .map(|p| {
            let ty = map_type(&p.ty, lang);
            format!("{}: {ty}", safe_ident(&p.name))
        })
        .collect();

    out.push_str(&format!("def {fn_name}({}) -> None:\n", params.join(", ")));

    // Docstring
    let mut doc_lines = Vec::new();
    if let Some(doc) = &action.doc {
        doc_lines.push(doc_text(doc));
        doc_lines.push(String::new());
    }

    if let Some(req) = &action.requires {
        doc_lines.push("Requires:".to_string());
        for cond in &req.conditions {
            doc_lines.push(format!("    - {}", format_expr(cond)));
        }
        doc_lines.push(String::new());
    }

    if let Some(ens) = &action.ensures {
        doc_lines.push("Ensures:".to_string());
        for item in &ens.items {
            doc_lines.push(format!("    - {}", format_ensures_item(item)));
        }
        doc_lines.push(String::new());
    }

    if let Some(props) = &action.properties {
        doc_lines.push("Properties:".to_string());
        for entry in &props.entries {
            doc_lines.push(format!(
                "    - {}: {}",
                entry.key,
                crate::format_prop_value(&entry.value)
            ));
        }
        doc_lines.push(String::new());
    }

    if !doc_lines.is_empty() {
        out.push_str("    \"\"\"\n");
        for line in &doc_lines {
            if line.is_empty() {
                out.push('\n');
            } else {
                out.push_str(&format!("    {line}\n"));
            }
        }
        out.push_str("    \"\"\"\n");
    }

    out.push_str(&format!(
        "    raise NotImplementedError(\"TODO: implement {fn_name}\")\n"
    ));
    out.push('\n');
    out.push('\n');
}

fn generate_invariant(out: &mut String, inv: &ast::InvariantDecl) {
    out.push_str(&format!("# Invariant: {}\n", inv.name));
    if let Some(doc) = &inv.doc {
        for line in doc_text(doc).lines() {
            out.push_str(&format!("# {line}\n"));
        }
    }
    out.push_str(&format!("# {}\n\n", format_expr(&inv.body)));
}

fn generate_edge_cases(out: &mut String, ec: &ast::EdgeCasesDecl) {
    out.push_str("# Edge cases:\n");
    for rule in &ec.rules {
        out.push_str(&format!(
            "# when {} => {}()\n",
            format_expr(&rule.condition),
            rule.action.name,
        ));
    }
    out.push('\n');
}
