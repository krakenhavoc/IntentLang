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
            ast::TopLevelItem::StateMachine(sm) => render_state_machine(&mut out, sm),
            ast::TopLevelItem::Test(t) => render_test(&mut out, t),
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

fn render_state_machine(out: &mut String, sm: &ast::StateMachineDecl) {
    out.push_str(&format!("## State Machine: {}\n\n", sm.name));
    if let Some(doc) = &sm.doc {
        for line in &doc.lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }
    let states_str: Vec<String> = sm.states.iter().map(|s| format!("`{}`", s)).collect();
    out.push_str(&format!("**States:** {}\n\n", states_str.join(", ")));
    if !sm.transitions.is_empty() {
        out.push_str("**Transitions:**\n\n");
        for (from, to) in &sm.transitions {
            out.push_str(&format!("- `{}` → `{}`\n", from, to));
        }
        out.push('\n');
    }
}

fn render_test(out: &mut String, test: &ast::TestDecl) {
    out.push_str(&format!("## Test: \"{}\"\n\n", test.name));

    if !test.given.is_empty() {
        out.push_str("**Given:**\n\n");
        for binding in &test.given {
            out.push_str(&format!(
                "- `{}` = `{}`\n",
                binding.name,
                format_given_value(&binding.value)
            ));
        }
        out.push('\n');
    }

    out.push_str(&format!(
        "**When:** `{}{}`\n\n",
        test.when_action.action_name,
        format_constructor_fields(&test.when_action.args)
    ));

    match &test.then {
        ast::ThenClause::Asserts(exprs, _) => {
            out.push_str("**Then:**\n\n");
            for expr in exprs {
                out.push_str(&format!("- `{}`\n", format_expr(expr)));
            }
            out.push('\n');
        }
        ast::ThenClause::Fails(kind, _) => {
            if let Some(kind) = kind {
                out.push_str(&format!("**Then:** fails `{}`\n\n", kind));
            } else {
                out.push_str("**Then:** fails\n\n");
            }
        }
    }
}

fn render_edge_cases(out: &mut String, _ec: &ast::EdgeCasesDecl) {
    out.push_str("## Edge Cases\n\n");
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
            let args_str: Vec<_> = args
                .iter()
                .map(|a| match a {
                    ast::CallArg::Named { key, value, .. } => {
                        format!("{}: {}", key, format_expr(value))
                    }
                    ast::CallArg::Positional(e) => format_expr(e),
                })
                .collect();
            format!("{}({})", name, args_str.join(", "))
        }
        ast::ExprKind::FieldAccess { root, fields } => {
            format!("{}.{}", format_expr(root), fields.join("."))
        }
        ast::ExprKind::List(items) => {
            let inner: Vec<_> = items.iter().map(format_expr).collect();
            format!("[{}]", inner.join(", "))
        }
        ast::ExprKind::Ident(name) => name.clone(),
        ast::ExprKind::Literal(lit) => crate::format_literal(lit),
    }
}

fn format_given_value(value: &ast::GivenValue) -> String {
    match value {
        ast::GivenValue::EntityConstructor { type_name, fields } => {
            format!("{}{}", type_name, format_constructor_fields(fields))
        }
        ast::GivenValue::Expr(e) => format_expr(e),
    }
}

fn format_constructor_fields(fields: &[ast::ConstructorField]) -> String {
    if fields.is_empty() {
        return String::new();
    }
    let inner: Vec<_> = fields
        .iter()
        .map(|f| format!("{}: {}", f.name, format_expr(&f.value)))
        .collect();
    format!(" {{ {} }}", inner.join(", "))
}
