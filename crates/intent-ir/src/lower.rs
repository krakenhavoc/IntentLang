//! AST → IR lowering pass.
//!
//! Converts a parsed intent AST into the typed IR representation.
//! Every IR node gets a `SourceTrace` linking back to the originating spec element.

use intent_parser::ast;

use crate::types::*;

/// Lower a parsed intent file into an IR module.
pub fn lower_file(file: &ast::File) -> Module {
    let module_name = &file.module.name;

    let mut structs = Vec::new();
    let mut functions = Vec::new();
    let mut invariants = Vec::new();
    let mut edge_guards = Vec::new();

    for item in &file.items {
        match item {
            ast::TopLevelItem::Entity(e) => {
                structs.push(lower_entity(module_name, e));
            }
            ast::TopLevelItem::Action(a) => {
                functions.push(lower_action(module_name, a));
            }
            ast::TopLevelItem::Invariant(inv) => {
                invariants.push(lower_invariant(module_name, inv));
            }
            ast::TopLevelItem::EdgeCases(ec) => {
                for rule in &ec.rules {
                    edge_guards.push(lower_edge_rule(module_name, rule));
                }
            }
        }
    }

    Module {
        name: module_name.clone(),
        structs,
        functions,
        invariants,
        edge_guards,
    }
}

// ── Entity → Struct ─────────────────────────────────────────

fn lower_entity(module: &str, entity: &ast::EntityDecl) -> Struct {
    let fields = entity
        .fields
        .iter()
        .map(|f| Field {
            name: f.name.clone(),
            ty: lower_type(&f.ty),
            trace: SourceTrace {
                module: module.to_string(),
                item: entity.name.clone(),
                part: format!("field:{}", f.name),
                span: f.span,
            },
        })
        .collect();

    Struct {
        name: entity.name.clone(),
        fields,
        trace: SourceTrace {
            module: module.to_string(),
            item: entity.name.clone(),
            part: "entity".to_string(),
            span: entity.span,
        },
    }
}

// ── Action → Function ───────────────────────────────────────

fn lower_action(module: &str, action: &ast::ActionDecl) -> Function {
    let params = action
        .params
        .iter()
        .map(|p| Param {
            name: p.name.clone(),
            ty: lower_type(&p.ty),
            trace: SourceTrace {
                module: module.to_string(),
                item: action.name.clone(),
                part: format!("param:{}", p.name),
                span: p.span,
            },
        })
        .collect();

    let preconditions = action
        .requires
        .as_ref()
        .map(|req| {
            req.conditions
                .iter()
                .map(|c| Condition {
                    expr: lower_expr(c),
                    trace: SourceTrace {
                        module: module.to_string(),
                        item: action.name.clone(),
                        part: "requires".to_string(),
                        span: c.span,
                    },
                })
                .collect()
        })
        .unwrap_or_default();

    let postconditions = action
        .ensures
        .as_ref()
        .map(|ens| {
            ens.items
                .iter()
                .map(|item| lower_ensures_item(module, &action.name, item))
                .collect()
        })
        .unwrap_or_default();

    let properties = action
        .properties
        .as_ref()
        .map(|props| {
            props
                .entries
                .iter()
                .map(|e| Property {
                    key: e.key.clone(),
                    value: lower_prop_value(&e.value),
                    trace: SourceTrace {
                        module: module.to_string(),
                        item: action.name.clone(),
                        part: format!("property:{}", e.key),
                        span: e.span,
                    },
                })
                .collect()
        })
        .unwrap_or_default();

    Function {
        name: action.name.clone(),
        params,
        preconditions,
        postconditions,
        properties,
        trace: SourceTrace {
            module: module.to_string(),
            item: action.name.clone(),
            part: "action".to_string(),
            span: action.span,
        },
    }
}

fn lower_ensures_item(module: &str, action: &str, item: &ast::EnsuresItem) -> Postcondition {
    match item {
        ast::EnsuresItem::Expr(e) => Postcondition::Always {
            expr: lower_expr(e),
            trace: SourceTrace {
                module: module.to_string(),
                item: action.to_string(),
                part: "ensures".to_string(),
                span: e.span,
            },
        },
        ast::EnsuresItem::When(w) => Postcondition::When {
            guard: lower_expr(&w.condition),
            expr: lower_expr(&w.consequence),
            trace: SourceTrace {
                module: module.to_string(),
                item: action.to_string(),
                part: "ensures:when".to_string(),
                span: w.span,
            },
        },
    }
}

// ── Invariant ───────────────────────────────────────────────

fn lower_invariant(module: &str, inv: &ast::InvariantDecl) -> Invariant {
    Invariant {
        name: inv.name.clone(),
        expr: lower_expr(&inv.body),
        trace: SourceTrace {
            module: module.to_string(),
            item: inv.name.clone(),
            part: "invariant".to_string(),
            span: inv.span,
        },
    }
}

// ── Edge rule → EdgeGuard ───────────────────────────────────

fn lower_edge_rule(module: &str, rule: &ast::EdgeRule) -> EdgeGuard {
    let args = rule
        .action
        .args
        .iter()
        .map(|arg| match arg {
            ast::CallArg::Named { key, value, .. } => (key.clone(), lower_expr(value)),
            ast::CallArg::Positional(e) => (String::new(), lower_expr(e)),
        })
        .collect();

    EdgeGuard {
        condition: lower_expr(&rule.condition),
        action: rule.action.name.clone(),
        args,
        trace: SourceTrace {
            module: module.to_string(),
            item: "edge_cases".to_string(),
            part: format!("when:{}", rule.action.name),
            span: rule.span,
        },
    }
}

// ── Type lowering ───────────────────────────────────────────

fn lower_type(ty: &ast::TypeExpr) -> IrType {
    let base = lower_type_kind(&ty.ty);
    if ty.optional {
        IrType::Optional(Box::new(base))
    } else {
        base
    }
}

fn lower_type_kind(kind: &ast::TypeKind) -> IrType {
    match kind {
        ast::TypeKind::Simple(name) => {
            // Recognize known struct-like types vs primitives.
            // During lowering we treat everything as Named; the verifier
            // resolves struct references later.
            IrType::Named(name.clone())
        }
        ast::TypeKind::Union(variants) => {
            let names: Vec<String> = variants
                .iter()
                .filter_map(|v| {
                    if let ast::TypeKind::Simple(name) = v {
                        Some(name.clone())
                    } else {
                        None
                    }
                })
                .collect();
            IrType::Union(names)
        }
        ast::TypeKind::List(inner) => IrType::List(Box::new(lower_type(inner))),
        ast::TypeKind::Set(inner) => IrType::Set(Box::new(lower_type(inner))),
        ast::TypeKind::Map(k, v) => IrType::Map(Box::new(lower_type(k)), Box::new(lower_type(v))),
        ast::TypeKind::Parameterized { name, params } => {
            if name == "Decimal" {
                let precision = params
                    .iter()
                    .find(|p| p.name == "precision")
                    .and_then(|p| {
                        if let ast::Literal::Int(n) = &p.value {
                            Some(*n as u32)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);
                IrType::Decimal(precision)
            } else {
                IrType::Named(name.clone())
            }
        }
    }
}

// ── Expression lowering ─────────────────────────────────────

fn lower_expr(expr: &ast::Expr) -> IrExpr {
    match &expr.kind {
        ast::ExprKind::Ident(name) => IrExpr::Var(name.clone()),
        ast::ExprKind::Literal(lit) => IrExpr::Literal(lower_literal(lit)),
        ast::ExprKind::FieldAccess { root, fields } => {
            let mut current = lower_expr(root);
            for field in fields {
                current = IrExpr::FieldAccess {
                    root: Box::new(current),
                    field: field.clone(),
                };
            }
            current
        }
        ast::ExprKind::Compare { left, op, right } => IrExpr::Compare {
            left: Box::new(lower_expr(left)),
            op: lower_cmp_op(*op),
            right: Box::new(lower_expr(right)),
        },
        ast::ExprKind::Arithmetic { left, op, right } => IrExpr::Arithmetic {
            left: Box::new(lower_expr(left)),
            op: lower_arith_op(*op),
            right: Box::new(lower_expr(right)),
        },
        ast::ExprKind::And(a, b) => {
            IrExpr::And(Box::new(lower_expr(a)), Box::new(lower_expr(b)))
        }
        ast::ExprKind::Or(a, b) => {
            IrExpr::Or(Box::new(lower_expr(a)), Box::new(lower_expr(b)))
        }
        ast::ExprKind::Not(inner) => IrExpr::Not(Box::new(lower_expr(inner))),
        ast::ExprKind::Implies(a, b) => {
            IrExpr::Implies(Box::new(lower_expr(a)), Box::new(lower_expr(b)))
        }
        ast::ExprKind::Old(inner) => IrExpr::Old(Box::new(lower_expr(inner))),
        ast::ExprKind::Quantifier {
            kind,
            binding,
            ty,
            body,
        } => match kind {
            ast::QuantifierKind::Forall => IrExpr::Forall {
                binding: binding.clone(),
                ty: ty.clone(),
                body: Box::new(lower_expr(body)),
            },
            ast::QuantifierKind::Exists => IrExpr::Exists {
                binding: binding.clone(),
                ty: ty.clone(),
                body: Box::new(lower_expr(body)),
            },
        },
        ast::ExprKind::Call { name, args } => IrExpr::Call {
            name: name.clone(),
            args: args
                .iter()
                .map(|a| match a {
                    ast::CallArg::Named { value, .. } => lower_expr(value),
                    ast::CallArg::Positional(e) => lower_expr(e),
                })
                .collect(),
        },
    }
}

fn lower_literal(lit: &ast::Literal) -> IrLiteral {
    match lit {
        ast::Literal::Null => IrLiteral::Null,
        ast::Literal::Bool(b) => IrLiteral::Bool(*b),
        ast::Literal::Int(n) => IrLiteral::Int(*n),
        ast::Literal::Decimal(s) => IrLiteral::Decimal(s.clone()),
        ast::Literal::String(s) => IrLiteral::String(s.clone()),
    }
}

fn lower_cmp_op(op: ast::CmpOp) -> CmpOp {
    match op {
        ast::CmpOp::Eq => CmpOp::Eq,
        ast::CmpOp::Ne => CmpOp::Ne,
        ast::CmpOp::Lt => CmpOp::Lt,
        ast::CmpOp::Gt => CmpOp::Gt,
        ast::CmpOp::Le => CmpOp::Le,
        ast::CmpOp::Ge => CmpOp::Ge,
    }
}

fn lower_arith_op(op: ast::ArithOp) -> ArithOp {
    match op {
        ast::ArithOp::Add => ArithOp::Add,
        ast::ArithOp::Sub => ArithOp::Sub,
    }
}

fn lower_prop_value(val: &ast::PropValue) -> PropertyValue {
    match val {
        ast::PropValue::Literal(ast::Literal::Bool(b)) => PropertyValue::Bool(*b),
        ast::PropValue::Literal(ast::Literal::Int(n)) => PropertyValue::Int(*n),
        ast::PropValue::Literal(ast::Literal::String(s)) => PropertyValue::String(s.clone()),
        ast::PropValue::Ident(s) => PropertyValue::Ident(s.clone()),
        // For complex prop values, fall back to string representation
        ast::PropValue::Literal(ast::Literal::Null) => PropertyValue::String("null".to_string()),
        ast::PropValue::Literal(ast::Literal::Decimal(s)) => PropertyValue::String(s.clone()),
        ast::PropValue::List(_) | ast::PropValue::Object(_) => {
            PropertyValue::String("<complex>".to_string())
        }
    }
}
