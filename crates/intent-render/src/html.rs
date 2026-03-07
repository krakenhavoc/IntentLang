//! Render a parsed intent specification as a self-contained HTML page.

use intent_parser::ast;

use crate::format_type;

/// Render an AST [`File`] to a complete HTML document string.
pub fn render(file: &ast::File) -> String {
    let mut body = String::new();
    body.push_str(&format!("<h1>{}</h1>\n", esc(&file.module.name)));

    if let Some(doc) = &file.doc {
        body.push_str("<p class=\"doc\">");
        body.push_str(&esc(&doc.lines.join(" ")));
        body.push_str("</p>\n");
    }

    for item in &file.items {
        match item {
            ast::TopLevelItem::Entity(e) => render_entity(&mut body, e),
            ast::TopLevelItem::Action(a) => render_action(&mut body, a),
            ast::TopLevelItem::Invariant(i) => render_invariant(&mut body, i),
            ast::TopLevelItem::EdgeCases(ec) => render_edge_cases(&mut body, ec),
        }
    }

    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <title>{title}</title>\n<style>\n{css}\n</style>\n</head>\n\
         <body>\n{body}</body>\n</html>\n",
        title = esc(&file.module.name),
        css = CSS,
        body = body,
    )
}

fn render_entity(out: &mut String, entity: &ast::EntityDecl) {
    out.push_str(&format!(
        "<section class=\"entity\">\n<h2>Entity: {}</h2>\n",
        esc(&entity.name)
    ));
    render_doc(out, &entity.doc);
    if !entity.fields.is_empty() {
        out.push_str("<table>\n<thead><tr><th>Field</th><th>Type</th></tr></thead>\n<tbody>\n");
        for field in &entity.fields {
            out.push_str(&format!(
                "<tr><td><code>{}</code></td><td><code>{}</code></td></tr>\n",
                esc(&field.name),
                esc(&format_type(&field.ty))
            ));
        }
        out.push_str("</tbody>\n</table>\n");
    }
    out.push_str("</section>\n");
}

fn render_action(out: &mut String, action: &ast::ActionDecl) {
    out.push_str(&format!(
        "<section class=\"action\">\n<h2>Action: {}</h2>\n",
        esc(&action.name)
    ));
    render_doc(out, &action.doc);
    if !action.params.is_empty() {
        out.push_str("<h3>Parameters</h3>\n<ul>\n");
        for p in &action.params {
            out.push_str(&format!(
                "<li><code>{}</code>: <code>{}</code></li>\n",
                esc(&p.name),
                esc(&format_type(&p.ty))
            ));
        }
        out.push_str("</ul>\n");
    }
    if let Some(req) = &action.requires {
        out.push_str("<h3>Requires</h3>\n<ul class=\"constraints\">\n");
        for cond in &req.conditions {
            out.push_str(&format!(
                "<li><code>{}</code></li>\n",
                esc(&format_expr(cond))
            ));
        }
        out.push_str("</ul>\n");
    }
    if let Some(ens) = &action.ensures {
        out.push_str("<h3>Ensures</h3>\n<ul class=\"constraints\">\n");
        for item in &ens.items {
            match item {
                ast::EnsuresItem::Expr(e) => {
                    out.push_str(&format!("<li><code>{}</code></li>\n", esc(&format_expr(e))));
                }
                ast::EnsuresItem::When(w) => {
                    out.push_str(&format!(
                        "<li>when <code>{}</code> &rArr; <code>{}</code></li>\n",
                        esc(&format_expr(&w.condition)),
                        esc(&format_expr(&w.consequence))
                    ));
                }
            }
        }
        out.push_str("</ul>\n");
    }
    if let Some(props) = &action.properties {
        out.push_str("<h3>Properties</h3>\n<table>\n<thead><tr><th>Key</th><th>Value</th></tr></thead>\n<tbody>\n");
        for entry in &props.entries {
            out.push_str(&format!(
                "<tr><td><code>{}</code></td><td><code>{}</code></td></tr>\n",
                esc(&entry.key),
                esc(&format_prop_value(&entry.value))
            ));
        }
        out.push_str("</tbody>\n</table>\n");
    }
    out.push_str("</section>\n");
}

fn render_invariant(out: &mut String, inv: &ast::InvariantDecl) {
    out.push_str(&format!(
        "<section class=\"invariant\">\n<h2>Invariant: {}</h2>\n",
        esc(&inv.name)
    ));
    render_doc(out, &inv.doc);
    out.push_str(&format!(
        "<pre><code>{}</code></pre>\n",
        esc(&format_expr(&inv.body))
    ));
    out.push_str("</section>\n");
}

fn render_edge_cases(out: &mut String, ec: &ast::EdgeCasesDecl) {
    out.push_str("<section class=\"edge-cases\">\n<h2>Edge Cases</h2>\n<ul>\n");
    for rule in &ec.rules {
        out.push_str(&format!(
            "<li>when <code>{}</code> &rArr; <code>{}({})</code></li>\n",
            esc(&format_expr(&rule.condition)),
            esc(&rule.action.name),
            esc(&format_call_args(&rule.action.args))
        ));
    }
    out.push_str("</ul>\n</section>\n");
}

fn render_doc(out: &mut String, doc: &Option<ast::DocBlock>) {
    if let Some(d) = doc {
        out.push_str("<p class=\"doc\">");
        out.push_str(&esc(&d.lines.join(" ")));
        out.push_str("</p>\n");
    }
}

// ── Expression formatting ───────────────────────────────────

fn format_expr(expr: &ast::Expr) -> String {
    match &expr.kind {
        ast::ExprKind::Implies(l, r) => format!("{} => {}", format_expr(l), format_expr(r)),
        ast::ExprKind::Or(l, r) => format!("{} || {}", format_expr(l), format_expr(r)),
        ast::ExprKind::And(l, r) => format!("{} && {}", format_expr(l), format_expr(r)),
        ast::ExprKind::Not(e) => format!("!{}", format_expr(e)),
        ast::ExprKind::Compare { left, op, right } => {
            let op_str = match op {
                ast::CmpOp::Eq => "==",
                ast::CmpOp::Ne => "!=",
                ast::CmpOp::Lt => "<",
                ast::CmpOp::Gt => ">",
                ast::CmpOp::Le => "<=",
                ast::CmpOp::Ge => ">=",
            };
            format!("{} {} {}", format_expr(left), op_str, format_expr(right))
        }
        ast::ExprKind::Arithmetic { left, op, right } => {
            let op_str = match op {
                ast::ArithOp::Add => "+",
                ast::ArithOp::Sub => "-",
            };
            format!("{} {} {}", format_expr(left), op_str, format_expr(right))
        }
        ast::ExprKind::Old(e) => format!("old({})", format_expr(e)),
        ast::ExprKind::Quantifier {
            kind,
            binding,
            ty,
            body,
        } => {
            let kw = match kind {
                ast::QuantifierKind::Forall => "forall",
                ast::QuantifierKind::Exists => "exists",
            };
            format!("{} {}: {} => {}", kw, binding, ty, format_expr(body))
        }
        ast::ExprKind::Call { name, args } => {
            format!("{}({})", name, format_call_args(args))
        }
        ast::ExprKind::FieldAccess { root, fields } => {
            format!("{}.{}", format_expr(root), fields.join("."))
        }
        ast::ExprKind::Ident(name) => name.clone(),
        ast::ExprKind::Literal(lit) => format_literal(lit),
    }
}

fn format_literal(lit: &ast::Literal) -> String {
    match lit {
        ast::Literal::Null => "null".to_string(),
        ast::Literal::Bool(b) => b.to_string(),
        ast::Literal::Int(n) => n.to_string(),
        ast::Literal::Decimal(s) => s.clone(),
        ast::Literal::String(s) => format!("\"{}\"", s),
    }
}

fn format_call_args(args: &[ast::CallArg]) -> String {
    args.iter()
        .map(|a| match a {
            ast::CallArg::Named { key, value, .. } => {
                format!("{}: {}", key, format_expr(value))
            }
            ast::CallArg::Positional(e) => format_expr(e),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_prop_value(val: &ast::PropValue) -> String {
    match val {
        ast::PropValue::Literal(lit) => format_literal(lit),
        ast::PropValue::Ident(name) => name.clone(),
        ast::PropValue::List(items) => {
            let inner: Vec<String> = items.iter().map(format_prop_value).collect();
            format!("[{}]", inner.join(", "))
        }
        ast::PropValue::Object(fields) => {
            let inner: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_prop_value(v)))
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
    }
}

// ── HTML escaping ───────────────────────────────────────────

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const CSS: &str = "\
body { font-family: system-ui, -apple-system, sans-serif; max-width: 48rem; margin: 2rem auto; padding: 0 1rem; color: #1a1a1a; line-height: 1.6; }
h1 { border-bottom: 2px solid #333; padding-bottom: 0.3rem; }
h2 { margin-top: 2rem; color: #2c5282; }
h3 { color: #555; }
.doc { color: #555; font-style: italic; }
table { border-collapse: collapse; width: 100%; margin: 0.5rem 0; }
th, td { border: 1px solid #ddd; padding: 0.4rem 0.8rem; text-align: left; }
th { background: #f7f7f7; }
code { background: #f5f5f5; padding: 0.1rem 0.3rem; border-radius: 3px; font-size: 0.9em; }
pre { background: #f5f5f5; padding: 1rem; border-radius: 4px; overflow-x: auto; }
pre code { background: none; padding: 0; }
ul.constraints { list-style: none; padding-left: 1rem; }
ul.constraints li::before { content: '\\2713 '; color: #38a169; }
section { margin-bottom: 1.5rem; }
.entity { border-left: 3px solid #3182ce; padding-left: 1rem; }
.action { border-left: 3px solid #d69e2e; padding-left: 1rem; }
.invariant { border-left: 3px solid #38a169; padding-left: 1rem; }
.edge-cases { border-left: 3px solid #e53e3e; padding-left: 1rem; }";
