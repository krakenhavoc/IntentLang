//! Render a parsed intent specification as Markdown.
//!
//! Produces clean, readable documentation suitable for review
//! by non-engineers (PMs, designers, stakeholders).

use intent_parser::ast;

use crate::format_type;

/// Render an AST [`File`] to a Markdown string.
pub fn render(file: &ast::File) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", file.module.name));

    if let Some(doc) = &file.doc {
        for line in &doc.lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }

    if !file.imports.is_empty() {
        out.push_str("**Imports:**\n\n");
        for use_decl in &file.imports {
            if let Some(item) = &use_decl.item {
                out.push_str(&format!("- `{}.{}`\n", use_decl.module_name, item));
            } else {
                out.push_str(&format!("- `{}`\n", use_decl.module_name));
            }
        }
        out.push('\n');
    }

    for item in &file.items {
        match item {
            ast::TopLevelItem::Entity(e) => render_entity(&mut out, e),
            ast::TopLevelItem::Action(a) => render_action(&mut out, a),
            ast::TopLevelItem::Invariant(i) => render_invariant(&mut out, i),
            ast::TopLevelItem::EdgeCases(ec) => render_edge_cases(&mut out, ec),
            ast::TopLevelItem::Test(_) => {} // Tests are not rendered
        }
    }

    out
}

fn render_entity(out: &mut String, entity: &ast::EntityDecl) {
    out.push_str(&format!("## Entity: {}\n\n", entity.name));
    if let Some(doc) = &entity.doc {
        for line in &doc.lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }
    out.push_str("| Field | Type |\n|-------|------|\n");
    for field in &entity.fields {
        out.push_str(&format!(
            "| `{}` | `{}` |\n",
            field.name,
            format_type(&field.ty)
        ));
    }
    out.push('\n');
}

fn render_action(out: &mut String, action: &ast::ActionDecl) {
    out.push_str(&format!("## Action: {}\n\n", action.name));
    if let Some(doc) = &action.doc {
        for line in &doc.lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }
    if !action.params.is_empty() {
        out.push_str("**Parameters:**\n\n");
        for p in &action.params {
            out.push_str(&format!("- `{}`: `{}`\n", p.name, format_type(&p.ty)));
        }
        out.push('\n');
    }
}

fn render_invariant(out: &mut String, inv: &ast::InvariantDecl) {
    out.push_str(&format!("## Invariant: {}\n\n", inv.name));
    if let Some(doc) = &inv.doc {
        for line in &doc.lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }
}

fn render_edge_cases(out: &mut String, _ec: &ast::EdgeCasesDecl) {
    out.push_str("## Edge Cases\n\n");
}
