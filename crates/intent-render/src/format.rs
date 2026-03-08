//! Format a parsed intent specification back to canonical `.intent` source.
//!
//! Parses then pretty-prints with consistent indentation and spacing.

use intent_parser::ast;

use crate::format_type;

/// Format an AST [`File`] back to canonical `.intent` source.
pub fn format(file: &ast::File) -> String {
    let mut out = String::new();
    out.push_str(&format!("module {}\n", file.module.name));

    if let Some(doc) = &file.doc {
        out.push('\n');
        for line in &doc.lines {
            out.push_str(&format!("--- {}\n", line.trim()));
        }
    }

    for use_decl in &file.imports {
        out.push('\n');
        if let Some(item) = &use_decl.item {
            out.push_str(&format!("use {}.{}\n", use_decl.module_name, item));
        } else {
            out.push_str(&format!("use {}\n", use_decl.module_name));
        }
    }

    for item in &file.items {
        out.push('\n');
        match item {
            ast::TopLevelItem::Entity(e) => fmt_entity(&mut out, e),
            ast::TopLevelItem::Action(a) => fmt_action(&mut out, a),
            ast::TopLevelItem::Invariant(i) => fmt_invariant(&mut out, i),
            ast::TopLevelItem::EdgeCases(ec) => fmt_edge_cases(&mut out, ec),
            ast::TopLevelItem::StateMachine(sm) => fmt_state_machine(&mut out, sm),
            ast::TopLevelItem::Test(_) => {} // Tests are not formatted
        }
    }

    out
}

fn fmt_doc(out: &mut String, doc: &Option<ast::DocBlock>) {
    if let Some(doc) = doc {
        for line in &doc.lines {
            out.push_str(&format!("  --- {}\n", line.trim()));
        }
        out.push('\n');
    }
}

fn fmt_entity(out: &mut String, entity: &ast::EntityDecl) {
    out.push_str(&format!("entity {} {{\n", entity.name));
    fmt_doc(out, &entity.doc);
    for field in &entity.fields {
        out.push_str(&format!("  {}: {}\n", field.name, format_type(&field.ty)));
    }
    out.push_str("}\n");
}

fn fmt_action(out: &mut String, action: &ast::ActionDecl) {
    out.push_str(&format!("action {} {{\n", action.name));
    fmt_doc(out, &action.doc);

    for param in &action.params {
        out.push_str(&format!("  {}: {}\n", param.name, format_type(&param.ty)));
    }

    if let Some(req) = &action.requires {
        out.push_str("\n  requires {\n");
        for cond in &req.conditions {
            out.push_str(&format!("    {}\n", fmt_expr(cond)));
        }
        out.push_str("  }\n");
    }

    if let Some(ens) = &action.ensures {
        out.push_str("\n  ensures {\n");
        for item in &ens.items {
            match item {
                ast::EnsuresItem::Expr(expr) => {
                    out.push_str(&format!("    {}\n", fmt_expr(expr)));
                }
                ast::EnsuresItem::When(w) => {
                    out.push_str(&format!(
                        "    when {} => {}\n",
                        fmt_expr(&w.condition),
                        fmt_expr(&w.consequence)
                    ));
                }
            }
        }
        out.push_str("  }\n");
    }

    if let Some(props) = &action.properties {
        out.push_str("\n  properties {\n");
        for entry in &props.entries {
            out.push_str(&format!(
                "    {}: {}\n",
                entry.key,
                fmt_prop_value(&entry.value)
            ));
        }
        out.push_str("  }\n");
    }

    out.push_str("}\n");
}

fn fmt_invariant(out: &mut String, inv: &ast::InvariantDecl) {
    out.push_str(&format!("invariant {} {{\n", inv.name));
    fmt_doc(out, &inv.doc);
    out.push_str(&format!("  {}\n", fmt_expr(&inv.body)));
    out.push_str("}\n");
}

fn fmt_state_machine(out: &mut String, sm: &ast::StateMachineDecl) {
    if let Some(doc) = &sm.doc {
        for line in &doc.lines {
            out.push_str(&format!("--- {}\n", line.trim()));
        }
    }
    out.push_str(&format!("state {} {{\n", sm.name));
    for chain in &sm.chains {
        out.push_str(&format!("  {}\n", chain.join(" -> ")));
    }
    out.push_str("}\n");
}

fn fmt_edge_cases(out: &mut String, ec: &ast::EdgeCasesDecl) {
    out.push_str("edge_cases {\n");
    for rule in &ec.rules {
        out.push_str(&format!(
            "  when {} => {}({})\n",
            fmt_expr(&rule.condition),
            rule.action.name,
            fmt_call_args(&rule.action.args),
        ));
    }
    out.push_str("}\n");
}

fn fmt_expr(expr: &ast::Expr) -> String {
    match &expr.kind {
        ast::ExprKind::Implies(l, r) => format!("{} => {}", fmt_expr(l), fmt_expr(r)),
        ast::ExprKind::Or(l, r) => format!("{} || {}", fmt_expr(l), fmt_expr(r)),
        ast::ExprKind::And(l, r) => format!("{} && {}", fmt_expr(l), fmt_expr(r)),
        ast::ExprKind::Not(e) => format!("!{}", fmt_expr(e)),
        ast::ExprKind::Compare { left, op, right } => {
            let op_str = match op {
                ast::CmpOp::Eq => "==",
                ast::CmpOp::Ne => "!=",
                ast::CmpOp::Lt => "<",
                ast::CmpOp::Gt => ">",
                ast::CmpOp::Le => "<=",
                ast::CmpOp::Ge => ">=",
            };
            format!("{} {} {}", fmt_expr(left), op_str, fmt_expr(right))
        }
        ast::ExprKind::Arithmetic { left, op, right } => {
            let op_str = match op {
                ast::ArithOp::Add => "+",
                ast::ArithOp::Sub => "-",
            };
            format!("{} {} {}", fmt_expr(left), op_str, fmt_expr(right))
        }
        ast::ExprKind::Old(e) => format!("old({})", fmt_expr(e)),
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
            format!("{} {}: {} => {}", kw, binding, ty, fmt_expr(body))
        }
        ast::ExprKind::Call { name, args } => {
            format!("{}({})", name, fmt_call_args(args))
        }
        ast::ExprKind::FieldAccess { root, fields } => {
            format!("{}.{}", fmt_expr(root), fields.join("."))
        }
        ast::ExprKind::List(items) => {
            let inner: Vec<_> = items.iter().map(fmt_expr).collect();
            format!("[{}]", inner.join(", "))
        }
        ast::ExprKind::Ident(name) => name.clone(),
        ast::ExprKind::Literal(lit) => fmt_literal(lit),
    }
}

fn fmt_literal(lit: &ast::Literal) -> String {
    match lit {
        ast::Literal::Null => "null".to_string(),
        ast::Literal::Bool(b) => b.to_string(),
        ast::Literal::Int(n) => n.to_string(),
        ast::Literal::Decimal(s) => s.clone(),
        ast::Literal::String(s) => format!("\"{}\"", s),
    }
}

fn fmt_call_args(args: &[ast::CallArg]) -> String {
    args.iter()
        .map(|a| match a {
            ast::CallArg::Named { key, value, .. } => {
                format!("{}: {}", key, fmt_expr(value))
            }
            ast::CallArg::Positional(e) => fmt_expr(e),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn fmt_prop_value(val: &ast::PropValue) -> String {
    match val {
        ast::PropValue::Literal(lit) => fmt_literal(lit),
        ast::PropValue::Ident(s) => s.clone(),
        ast::PropValue::List(items) => {
            let inner: Vec<_> = items.iter().map(fmt_prop_value).collect();
            format!("[{}]", inner.join(", "))
        }
        ast::PropValue::Object(fields) => {
            let inner: Vec<_> = fields
                .iter()
                .map(|(k, v)| format!("{}: {}", k, fmt_prop_value(v)))
                .collect();
            format!("{{ {} }}", inner.join(", "))
        }
    }
}
