//! TypeScript skeleton code generator.

use intent_parser::ast;

use crate::types::map_type;
use crate::{Language, doc_text, format_ensures_item, format_expr, to_camel_case};

/// Generate TypeScript skeleton code from a parsed intent file.
pub fn generate(file: &ast::File) -> String {
    let lang = Language::TypeScript;
    let mut out = String::new();

    // Header
    out.push_str(&format!("// Generated from {}.intent\n", file.module.name));
    if let Some(doc) = &file.doc {
        out.push_str("/**\n");
        for line in &doc.lines {
            out.push_str(&format!(" * {line}\n"));
        }
        out.push_str(" */\n");
    }
    out.push('\n');

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
    // Doc comment
    if let Some(doc) = &entity.doc {
        out.push_str("/**\n");
        for line in doc_text(doc).lines() {
            out.push_str(&format!(" * {line}\n"));
        }
        out.push_str(" */\n");
    }

    out.push_str(&format!("export interface {} {{\n", entity.name));

    for field in &entity.fields {
        let ty = map_type(&field.ty, lang);
        out.push_str(&format!("  {}: {};\n", to_camel_case(&field.name), ty));
    }

    out.push_str("}\n\n");
}

fn generate_action(out: &mut String, action: &ast::ActionDecl, lang: &Language) {
    // JSDoc
    out.push_str("/**\n");
    if let Some(doc) = &action.doc {
        for line in doc_text(doc).lines() {
            out.push_str(&format!(" * {line}\n"));
        }
        out.push_str(" *\n");
    }

    // Parameters
    for p in &action.params {
        let ty = map_type(&p.ty, lang);
        out.push_str(&format!(" * @param {} - {ty}\n", to_camel_case(&p.name)));
    }

    // Requires
    if let Some(req) = &action.requires {
        out.push_str(" *\n * @requires\n");
        for cond in &req.conditions {
            out.push_str(&format!(" *   - {}\n", format_expr(cond)));
        }
    }

    // Ensures
    if let Some(ens) = &action.ensures {
        out.push_str(" *\n * @ensures\n");
        for item in &ens.items {
            out.push_str(&format!(" *   - {}\n", format_ensures_item(item)));
        }
    }

    // Properties
    if let Some(props) = &action.properties {
        out.push_str(" *\n * @properties\n");
        for entry in &props.entries {
            out.push_str(&format!(
                " *   - {}: {}\n",
                entry.key,
                crate::format_prop_value(&entry.value)
            ));
        }
    }

    out.push_str(" */\n");

    // Function signature
    let fn_name = to_camel_case(&action.name);
    let params: Vec<String> = action
        .params
        .iter()
        .map(|p| {
            let ty = map_type(&p.ty, lang);
            format!("{}: {ty}", to_camel_case(&p.name))
        })
        .collect();

    out.push_str(&format!(
        "export function {fn_name}({}): void {{\n",
        params.join(", ")
    ));
    out.push_str(&format!(
        "  throw new Error(\"TODO: implement {fn_name}\");\n"
    ));
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
