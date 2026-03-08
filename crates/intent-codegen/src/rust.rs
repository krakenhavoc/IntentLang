//! Rust skeleton code generator.

use intent_parser::ast;

use crate::types::map_type;
use crate::{Language, doc_text, format_ensures_item, format_expr, to_snake_case};

/// Rust reserved keywords that cannot be used as identifiers.
const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
    "pub", "ref", "return", "self", "static", "struct", "super", "trait", "true", "type", "unsafe",
    "use", "where", "while", "yield",
];

/// Escape a Rust identifier if it collides with a reserved keyword.
fn safe_ident(name: &str) -> String {
    let snake = to_snake_case(name);
    if RUST_KEYWORDS.contains(&snake.as_str()) {
        format!("r#{snake}")
    } else {
        snake
    }
}

/// Generate Rust skeleton code from a parsed intent file.
pub fn generate(file: &ast::File) -> String {
    let lang = Language::Rust;
    let mut out = String::new();

    // Header
    out.push_str(&format!("//! Generated from {}.intent\n", file.module.name));
    if let Some(doc) = &file.doc {
        for line in &doc.lines {
            out.push_str(&format!("//! {line}\n"));
        }
    }
    out.push('\n');

    // Imports (based on types used)
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
        out.push_str(&format!("/// {}\n", doc_text(doc)));
    }
    out.push_str("#[derive(Debug, Clone, PartialEq, Eq)]\n");
    out.push_str(&format!("pub enum {} {{\n", sm.name));
    for state in &sm.states {
        out.push_str(&format!("    {},\n", state));
    }
    out.push_str("}\n\n");

    // Generate is_valid_transition method
    out.push_str(&format!("impl {} {{\n", sm.name));
    out.push_str("    pub fn is_valid_transition(&self, to: &Self) -> bool {\n");
    out.push_str("        matches!((self, to),\n");
    let arms: Vec<String> = sm
        .transitions
        .iter()
        .map(|(from, to)| format!("            (Self::{}, Self::{})", from, to))
        .collect();
    out.push_str(&arms.join(" |\n"));
    out.push_str("\n        )\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
}

fn generate_imports(file: &ast::File) -> String {
    let mut imports = Vec::new();
    let source = collect_type_names(file);

    if source.contains("UUID") {
        imports.push("use uuid::Uuid;");
    }
    if source.contains("Decimal") {
        imports.push("use rust_decimal::Decimal;");
    }
    if source.contains("DateTime") {
        imports.push("use chrono::{DateTime, Utc};");
    }
    if source.contains("Set<") {
        imports.push("use std::collections::HashSet;");
    }
    if source.contains("Map<") {
        imports.push("use std::collections::HashMap;");
    }

    if imports.is_empty() {
        String::new()
    } else {
        imports.join("\n") + "\n"
    }
}

/// Collect all type names as a single string for import detection.
fn collect_type_names(file: &ast::File) -> String {
    let mut names = String::new();
    for item in &file.items {
        match item {
            ast::TopLevelItem::Entity(e) => {
                for f in &e.fields {
                    names.push_str(&format_type_for_scan(&f.ty));
                    names.push(' ');
                }
            }
            ast::TopLevelItem::Action(a) => {
                for p in &a.params {
                    names.push_str(&format_type_for_scan(&p.ty));
                    names.push(' ');
                }
            }
            _ => {}
        }
    }
    names
}

fn format_type_for_scan(ty: &ast::TypeExpr) -> String {
    match &ty.ty {
        ast::TypeKind::Simple(n) => n.clone(),
        ast::TypeKind::List(inner) => format!("List<{}>", format_type_for_scan(inner)),
        ast::TypeKind::Set(inner) => format!("Set<{}>", format_type_for_scan(inner)),
        ast::TypeKind::Map(k, v) => {
            format!(
                "Map<{}, {}>",
                format_type_for_scan(k),
                format_type_for_scan(v)
            )
        }
        ast::TypeKind::Union(variants) => variants
            .iter()
            .filter_map(|v| match v {
                ast::TypeKind::Simple(n) => Some(n.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" "),
        ast::TypeKind::Parameterized { name, .. } => name.clone(),
    }
}

fn generate_entity(out: &mut String, entity: &ast::EntityDecl, lang: &Language) {
    // Emit union enums for any union-typed fields
    for field in &entity.fields {
        if let ast::TypeKind::Union(variants) = &field.ty.ty {
            let enum_name = format!("{}{}", entity.name, capitalize(&field.name));
            generate_union_enum(out, &enum_name, variants);
        }
    }

    // Doc comment
    if let Some(doc) = &entity.doc {
        for line in doc_text(doc).lines() {
            out.push_str(&format!("/// {line}\n"));
        }
    }

    out.push_str("#[derive(Debug, Clone)]\n");
    out.push_str(&format!("pub struct {} {{\n", entity.name));

    for field in &entity.fields {
        let ty = if let ast::TypeKind::Union(_) = &field.ty.ty {
            let enum_name = format!("{}{}", entity.name, capitalize(&field.name));
            if field.ty.optional {
                format!("Option<{enum_name}>")
            } else {
                enum_name
            }
        } else {
            map_type(&field.ty, lang)
        };
        out.push_str(&format!("    pub {}: {},\n", safe_ident(&field.name), ty));
    }

    out.push_str("}\n\n");
}

fn generate_union_enum(out: &mut String, name: &str, variants: &[ast::TypeKind]) {
    out.push_str("#[derive(Debug, Clone, PartialEq, Eq)]\n");
    out.push_str(&format!("pub enum {name} {{\n"));
    for v in variants {
        if let ast::TypeKind::Simple(n) = v {
            out.push_str(&format!("    {n},\n"));
        }
    }
    out.push_str("}\n\n");
}

fn generate_action(out: &mut String, action: &ast::ActionDecl, lang: &Language) {
    // Doc comment
    if let Some(doc) = &action.doc {
        for line in doc_text(doc).lines() {
            out.push_str(&format!("/// {line}\n"));
        }
    }
    out.push_str("///\n");

    // Requires
    if let Some(req) = &action.requires {
        out.push_str("/// # Requires\n");
        for cond in &req.conditions {
            out.push_str(&format!("/// - `{}`\n", format_expr(cond)));
        }
        out.push_str("///\n");
    }

    // Ensures
    if let Some(ens) = &action.ensures {
        out.push_str("/// # Ensures\n");
        for item in &ens.items {
            out.push_str(&format!("/// - `{}`\n", format_ensures_item(item)));
        }
        out.push_str("///\n");
    }

    // Properties
    if let Some(props) = &action.properties {
        out.push_str("/// # Properties\n");
        for entry in &props.entries {
            out.push_str(&format!(
                "/// - {}: {}\n",
                entry.key,
                crate::format_prop_value(&entry.value)
            ));
        }
        out.push_str("///\n");
    }

    // Function signature
    let fn_name = to_snake_case(&action.name);
    let params: Vec<String> = action
        .params
        .iter()
        .map(|p| {
            let ty = map_type(&p.ty, lang);
            format!("{}: {ty}", safe_ident(&p.name))
        })
        .collect();

    out.push_str(&format!(
        "pub fn {fn_name}({}) -> Result<(), Box<dyn std::error::Error>> {{\n",
        params.join(", ")
    ));
    out.push_str(&format!("    todo!(\"Implement {fn_name}\")\n"));
    out.push_str("}\n\n");
}

fn generate_invariant(out: &mut String, inv: &ast::InvariantDecl) {
    out.push_str(&format!("// Invariant: {}\n", inv.name));
    if let Some(doc) = &inv.doc {
        for line in doc_text(doc).lines() {
            out.push_str(&format!("// {line}\n"));
        }
    }
    out.push_str(&format!("// {}\n\n", format_expr(&inv.body)));
}

fn generate_edge_cases(out: &mut String, ec: &ast::EdgeCasesDecl) {
    out.push_str("// Edge cases:\n");
    for rule in &ec.rules {
        out.push_str(&format!(
            "// when {} => {}()\n",
            format_expr(&rule.condition),
            rule.action.name,
        ));
    }
    out.push('\n');
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
