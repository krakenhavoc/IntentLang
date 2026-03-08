//! Swift skeleton code generator.
//!
//! Generates Swift structs with Codable conformance for entities,
//! enums with String raw values for union types, and throwing
//! functions for actions.

use intent_parser::ast;

use crate::types::map_type;
use crate::{Language, doc_text, format_ensures_item, format_expr, to_camel_case};

/// Swift reserved keywords that cannot be used as identifiers.
const SWIFT_KEYWORDS: &[&str] = &[
    "associatedtype",
    "class",
    "deinit",
    "enum",
    "extension",
    "fileprivate",
    "func",
    "import",
    "init",
    "inout",
    "internal",
    "let",
    "open",
    "operator",
    "private",
    "precedencegroup",
    "protocol",
    "public",
    "rethrows",
    "return",
    "static",
    "struct",
    "subscript",
    "super",
    "switch",
    "throws",
    "typealias",
    "var",
    "break",
    "case",
    "catch",
    "continue",
    "default",
    "defer",
    "do",
    "else",
    "fallthrough",
    "for",
    "guard",
    "if",
    "in",
    "repeat",
    "throw",
    "try",
    "where",
    "while",
    "as",
    "false",
    "is",
    "nil",
    "self",
    "true",
    "type",
];

/// Escape a Swift identifier if it collides with a reserved keyword.
fn safe_ident(name: &str) -> String {
    let camel = to_camel_case(name);
    if SWIFT_KEYWORDS.contains(&camel.as_str()) {
        format!("`{camel}`")
    } else {
        camel
    }
}

/// Capitalize the first character of a string.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Generate Swift skeleton code from a parsed intent file.
pub fn generate(file: &ast::File) -> String {
    let lang = Language::Swift;
    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "// Generated from {}.intent. DO NOT EDIT.\n",
        file.module.name
    ));
    if let Some(doc) = &file.doc {
        out.push('\n');
        for line in &doc.lines {
            out.push_str(&format!("// {line}\n"));
        }
    }
    out.push('\n');

    // Import Foundation for UUID, Decimal, Date
    out.push_str("import Foundation\n\n");

    for item in &file.items {
        match item {
            ast::TopLevelItem::Entity(e) => generate_entity(&mut out, e, &lang),
            ast::TopLevelItem::Action(a) => generate_action(&mut out, a, &lang),
            ast::TopLevelItem::Invariant(inv) => generate_invariant(&mut out, inv),
            ast::TopLevelItem::EdgeCases(ec) => generate_edge_cases(&mut out, ec),
            ast::TopLevelItem::Test(_) => {}
        }
    }

    out
}

fn generate_entity(out: &mut String, entity: &ast::EntityDecl, lang: &Language) {
    // Emit enum types for union-typed fields
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

    out.push_str(&format!("struct {}: Codable {{\n", entity.name));

    for field in &entity.fields {
        let ty = if let ast::TypeKind::Union(_) = &field.ty.ty {
            let enum_name = format!("{}{}", entity.name, capitalize(&field.name));
            if field.ty.optional {
                format!("{enum_name}?")
            } else {
                enum_name
            }
        } else {
            map_type(&field.ty, lang)
        };
        out.push_str(&format!("    let {}: {ty}\n", safe_ident(&field.name)));
    }

    out.push_str("}\n\n");
}

fn generate_union_enum(out: &mut String, name: &str, variants: &[ast::TypeKind]) {
    let names: Vec<&str> = variants
        .iter()
        .filter_map(|v| match v {
            ast::TypeKind::Simple(n) => Some(n.as_str()),
            _ => None,
        })
        .collect();

    out.push_str(&format!("enum {name}: String, Codable {{\n"));
    for n in &names {
        let case_name = to_camel_case(n);
        out.push_str(&format!("    case {case_name} = \"{n}\"\n"));
    }
    out.push_str("}\n\n");
}

fn generate_action(out: &mut String, action: &ast::ActionDecl, lang: &Language) {
    let fn_name = to_camel_case(&action.name);

    // Doc comment
    if let Some(doc) = &action.doc {
        for line in doc_text(doc).lines() {
            out.push_str(&format!("/// {line}\n"));
        }
    }

    // Requires
    if let Some(req) = &action.requires {
        out.push_str("///\n/// - Requires:\n");
        for cond in &req.conditions {
            out.push_str(&format!("///   - `{}`\n", format_expr(cond)));
        }
    }

    // Ensures
    if let Some(ens) = &action.ensures {
        out.push_str("///\n/// - Ensures:\n");
        for item in &ens.items {
            out.push_str(&format!("///   - `{}`\n", format_ensures_item(item)));
        }
    }

    // Properties
    if let Some(props) = &action.properties {
        out.push_str("///\n/// - Properties:\n");
        for entry in &props.entries {
            out.push_str(&format!(
                "///   - {}: {}\n",
                entry.key,
                crate::format_prop_value(&entry.value)
            ));
        }
    }

    // Function signature
    let params: Vec<String> = action
        .params
        .iter()
        .map(|p| {
            let ty = map_type(&p.ty, lang);
            format!("{}: {ty}", safe_ident(&p.name))
        })
        .collect();

    out.push_str(&format!(
        "func {fn_name}({}) throws {{\n",
        params.join(", ")
    ));
    out.push_str(&format!("    fatalError(\"TODO: implement {fn_name}\")\n"));
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
