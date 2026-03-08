//! C# skeleton code generator.
//!
//! Generates C# records for entities, enums for union types,
//! and static methods in a static Actions class. Uses file-scoped
//! namespaces and nullable reference types (C# 10+).

use intent_parser::ast;

use crate::types::map_type;
use crate::{Language, doc_text, format_ensures_item, format_expr, to_snake_case};

/// C# reserved keywords that cannot be used as identifiers.
const CSHARP_KEYWORDS: &[&str] = &[
    "abstract",
    "as",
    "base",
    "bool",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "checked",
    "class",
    "const",
    "continue",
    "decimal",
    "default",
    "delegate",
    "do",
    "double",
    "else",
    "enum",
    "event",
    "explicit",
    "extern",
    "false",
    "finally",
    "fixed",
    "float",
    "for",
    "foreach",
    "goto",
    "if",
    "implicit",
    "in",
    "int",
    "interface",
    "internal",
    "is",
    "lock",
    "long",
    "namespace",
    "new",
    "null",
    "object",
    "operator",
    "out",
    "override",
    "params",
    "private",
    "protected",
    "public",
    "readonly",
    "ref",
    "return",
    "sbyte",
    "sealed",
    "short",
    "sizeof",
    "stackalloc",
    "static",
    "string",
    "struct",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "uint",
    "ulong",
    "unchecked",
    "unsafe",
    "ushort",
    "using",
    "virtual",
    "void",
    "volatile",
    "while",
];

/// Escape a C# identifier if it collides with a reserved keyword.
fn safe_ident(name: &str) -> String {
    let camel = crate::to_camel_case(name);
    if CSHARP_KEYWORDS.contains(&camel.as_str()) {
        format!("@{camel}")
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

/// Convert a name to PascalCase (C# convention for public members).
fn to_pascal_case(s: &str) -> String {
    to_snake_case(s)
        .split('_')
        .map(capitalize)
        .collect::<String>()
}

/// Generate C# skeleton code from a parsed intent file.
pub fn generate(file: &ast::File) -> String {
    let lang = Language::CSharp;
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

    // Nullable enable
    out.push_str("#nullable enable\n\n");

    // File-scoped namespace
    out.push_str(&format!("namespace {};\n\n", file.module.name));

    // Imports
    let imports = generate_imports(file);
    if !imports.is_empty() {
        out.push_str(&imports);
        out.push('\n');
    }

    let has_actions = file
        .items
        .iter()
        .any(|item| matches!(item, ast::TopLevelItem::Action(_)));
    let has_invariants = file
        .items
        .iter()
        .any(|item| matches!(item, ast::TopLevelItem::Invariant(_)));
    let has_edge_cases = file
        .items
        .iter()
        .any(|item| matches!(item, ast::TopLevelItem::EdgeCases(_)));

    // State machines, entities, and enums (top-level)
    for item in &file.items {
        match item {
            ast::TopLevelItem::Entity(e) => generate_entity(&mut out, e, &lang),
            ast::TopLevelItem::StateMachine(sm) => generate_state_machine(&mut out, sm),
            _ => {}
        }
    }

    // Actions in a static class
    if has_actions || has_invariants || has_edge_cases {
        out.push_str(&format!(
            "public static class {}Actions\n{{\n",
            file.module.name
        ));
        for item in &file.items {
            match item {
                ast::TopLevelItem::Action(a) => generate_action(&mut out, a, &lang),
                ast::TopLevelItem::Invariant(inv) => generate_invariant(&mut out, inv),
                ast::TopLevelItem::EdgeCases(ec) => generate_edge_cases(&mut out, ec),
                _ => {}
            }
        }
        out.push_str("}\n");
    }

    out
}

fn generate_imports(file: &ast::File) -> String {
    let source = collect_type_names(file);
    let mut imports = Vec::new();

    if source.contains("List<") || source.contains("Set<") || source.contains("Map<") {
        imports.push("using System.Collections.Generic;");
    }

    if imports.is_empty() {
        return String::new();
    }

    imports.join("\n") + "\n"
}

/// Collect all type names as a single string for import detection.
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
        ast::TypeKind::List(inner) => {
            out.push_str("List<");
            collect_type_name(inner, out);
        }
        ast::TypeKind::Set(inner) => {
            out.push_str("Set<");
            collect_type_name(inner, out);
        }
        ast::TypeKind::Map(k, v) => {
            out.push_str("Map<");
            collect_type_name(k, out);
            collect_type_name(v, out);
        }
        ast::TypeKind::Union(_) => {}
    }
}

fn generate_state_machine(out: &mut String, sm: &ast::StateMachineDecl) {
    if let Some(doc) = &sm.doc {
        out.push_str(&format!(
            "/// <summary>{}</summary>\n",
            doc_text(doc).trim()
        ));
    }
    out.push_str(&format!("public enum {}\n{{\n", sm.name));
    for (i, state) in sm.states.iter().enumerate() {
        let comma = if i < sm.states.len() - 1 { "," } else { "" };
        out.push_str(&format!("    {}{}\n", state, comma));
    }
    out.push_str("}\n\n");

    // Extension method for transition validation
    out.push_str(&format!("public static class {}Extensions\n{{\n", sm.name));
    out.push_str(&format!(
        "    public static bool CanTransitionTo(this {} from, {} to) =>\n",
        sm.name, sm.name
    ));
    out.push_str("        (from, to) switch\n        {\n");
    for (from, to) in &sm.transitions {
        out.push_str(&format!(
            "            ({}.{}, {}.{}) => true,\n",
            sm.name, from, sm.name, to
        ));
    }
    out.push_str("            _ => false\n        };\n");
    out.push_str("}\n\n");
}

fn generate_entity(out: &mut String, entity: &ast::EntityDecl, lang: &Language) {
    // Emit enum types for union-typed fields
    for field in &entity.fields {
        if let ast::TypeKind::Union(variants) = &field.ty.ty {
            let enum_name = format!("{}{}", entity.name, capitalize(&field.name));
            generate_union_enum(out, &enum_name, variants);
        }
    }

    // XML doc comment
    if let Some(doc) = &entity.doc {
        out.push_str("/// <summary>\n");
        for line in doc_text(doc).lines() {
            out.push_str(&format!("/// {line}\n"));
        }
        out.push_str("/// </summary>\n");
    }

    // Record declaration
    let params: Vec<String> = entity
        .fields
        .iter()
        .map(|f| {
            let ty = if let ast::TypeKind::Union(_) = &f.ty.ty {
                let enum_name = format!("{}{}", entity.name, capitalize(&f.name));
                if f.ty.optional {
                    format!("{enum_name}?")
                } else {
                    enum_name
                }
            } else {
                map_type(&f.ty, lang)
            };
            format!("{ty} {}", to_pascal_case(&f.name))
        })
        .collect();

    out.push_str(&format!("public record {}(\n", entity.name));
    for (i, param) in params.iter().enumerate() {
        let comma = if i < params.len() - 1 { "," } else { "" };
        out.push_str(&format!("    {param}{comma}\n"));
    }
    out.push_str(");\n\n");
}

fn generate_union_enum(out: &mut String, name: &str, variants: &[ast::TypeKind]) {
    let names: Vec<&str> = variants
        .iter()
        .filter_map(|v| match v {
            ast::TypeKind::Simple(n) => Some(n.as_str()),
            _ => None,
        })
        .collect();

    out.push_str(&format!("public enum {name}\n{{\n"));
    for (i, n) in names.iter().enumerate() {
        let comma = if i < names.len() - 1 { "," } else { "" };
        out.push_str(&format!("    {n}{comma}\n"));
    }
    out.push_str("}\n\n");
}

fn generate_action(out: &mut String, action: &ast::ActionDecl, lang: &Language) {
    let fn_name = to_pascal_case(&action.name);

    // XML doc comment
    out.push_str("    /// <summary>\n");
    if let Some(doc) = &action.doc {
        for line in doc_text(doc).lines() {
            out.push_str(&format!("    /// {line}\n"));
        }
    }
    out.push_str("    /// </summary>\n");

    // Requires
    if let Some(req) = &action.requires {
        out.push_str("    /// <remarks>\n    /// Requires:\n");
        for cond in &req.conditions {
            out.push_str(&format!("    /// - {}\n", format_expr(cond)));
        }
    }

    // Ensures
    if let Some(ens) = &action.ensures {
        if action.requires.is_none() {
            out.push_str("    /// <remarks>\n");
        }
        out.push_str("    /// Ensures:\n");
        for item in &ens.items {
            out.push_str(&format!("    /// - {}\n", format_ensures_item(item)));
        }
    }

    if action.requires.is_some() || action.ensures.is_some() {
        out.push_str("    /// </remarks>\n");
    }

    // Properties
    if let Some(props) = &action.properties {
        out.push_str("    /// <remarks>\n    /// Properties:\n");
        for entry in &props.entries {
            out.push_str(&format!(
                "    /// - {}: {}\n",
                entry.key,
                crate::format_prop_value(&entry.value)
            ));
        }
        out.push_str("    /// </remarks>\n");
    }

    // Method signature
    let params: Vec<String> = action
        .params
        .iter()
        .map(|p| {
            let ty = map_type(&p.ty, lang);
            format!("{ty} {}", safe_ident(&p.name))
        })
        .collect();

    out.push_str(&format!(
        "    public static void {fn_name}({})\n",
        params.join(", ")
    ));
    out.push_str("    {\n");
    out.push_str(&format!(
        "        throw new NotImplementedException(\"TODO: implement {fn_name}\");\n"
    ));
    out.push_str("    }\n\n");
}

fn generate_invariant(out: &mut String, inv: &ast::InvariantDecl) {
    out.push_str(&format!("    // Invariant: {}\n", inv.name));
    if let Some(doc) = &inv.doc {
        for line in doc_text(doc).lines() {
            out.push_str(&format!("    // {line}\n"));
        }
    }
    out.push_str(&format!("    // {}\n\n", format_expr(&inv.body)));
}

fn generate_edge_cases(out: &mut String, ec: &ast::EdgeCasesDecl) {
    out.push_str("    // Edge cases:\n");
    for rule in &ec.rules {
        out.push_str(&format!(
            "    // when {} => {}()\n",
            format_expr(&rule.condition),
            rule.action.name,
        ));
    }
    out.push('\n');
}
