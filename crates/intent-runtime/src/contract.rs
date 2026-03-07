use std::collections::HashMap;

use intent_ir::{CmpOp, IrExpr, Module, Postcondition};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::RuntimeError;
use crate::eval::evaluate;
use crate::value::EvalContext;

/// Request to execute an action against current state.
#[derive(Debug, Clone, Deserialize)]
pub struct ActionRequest {
    /// Action (function) name to execute.
    #[serde(default)]
    pub action: String,
    /// Action parameters as JSON values. Keys are param names.
    pub params: HashMap<String, Value>,
    /// Entity instances by type name, for quantifier evaluation and invariant checking.
    pub state: HashMap<String, Vec<Value>>,
}

/// Result of executing an action.
#[derive(Debug, Clone, Serialize)]
pub struct ActionResult {
    /// Whether the action succeeded (no violations).
    pub ok: bool,
    /// Updated parameter values (new state after ensures).
    pub new_params: HashMap<String, Value>,
    /// Constraint violations, if any.
    pub violations: Vec<Violation>,
}

/// A constraint violation.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Violation {
    pub kind: ViolationKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ViolationKind {
    PreconditionFailed,
    PostconditionFailed,
    InvariantViolated,
    EdgeGuardTriggered,
}

/// Execute an action against the given module and state.
///
/// 1. Find the action (function) in the module
/// 2. Check preconditions
/// 3. Extract field assignments from `ensures` and compute new state
/// 4. Validate postconditions against new state
/// 5. Validate invariants against new state
pub fn execute_action(
    module: &Module,
    request: &ActionRequest,
) -> Result<ActionResult, RuntimeError> {
    let func = module
        .functions
        .iter()
        .find(|f| f.name == request.action)
        .ok_or_else(|| RuntimeError::UnknownFunction(request.action.clone()))?;

    // Build eval context with params as bindings and state as instances.
    let ctx = EvalContext {
        bindings: request.params.clone(),
        old_bindings: None,
        instances: request.state.clone(),
    };

    let mut violations = Vec::new();

    // 1. Check preconditions.
    for pre in &func.preconditions {
        match evaluate(&pre.expr, &ctx) {
            Ok(Value::Bool(true)) => {}
            Ok(Value::Bool(false)) => {
                violations.push(Violation {
                    kind: ViolationKind::PreconditionFailed,
                    message: format!("precondition failed: {}", fmt_expr(&pre.expr)),
                });
            }
            Ok(_) => {
                violations.push(Violation {
                    kind: ViolationKind::PreconditionFailed,
                    message: format!(
                        "precondition did not evaluate to bool: {}",
                        fmt_expr(&pre.expr)
                    ),
                });
            }
            Err(e) => {
                violations.push(Violation {
                    kind: ViolationKind::PreconditionFailed,
                    message: format!("precondition error: {e}"),
                });
            }
        }
    }

    // If preconditions failed, return early — don't transform state.
    if !violations.is_empty() {
        return Ok(ActionResult {
            ok: false,
            new_params: request.params.clone(),
            violations,
        });
    }

    // 2. Check edge guards.
    for guard in &module.edge_guards {
        match evaluate(&guard.condition, &ctx) {
            Ok(Value::Bool(true)) => {
                violations.push(Violation {
                    kind: ViolationKind::EdgeGuardTriggered,
                    message: format!(
                        "edge case triggered: {} => {}",
                        fmt_expr(&guard.condition),
                        guard.action
                    ),
                });
            }
            Ok(Value::Bool(false)) | Err(_) => {}
            Ok(_) => {}
        }
    }

    if !violations.is_empty() {
        return Ok(ActionResult {
            ok: false,
            new_params: request.params.clone(),
            violations,
        });
    }

    // 3. Compute new state from ensures.
    let old_params = request.params.clone();
    let mut new_params = request.params.clone();

    // Extract assignments from postconditions: patterns like `param.field == expr_with_old`.
    for post in &func.postconditions {
        let expr = match post {
            Postcondition::Always { expr, .. } => expr,
            Postcondition::When { guard, expr, .. } => {
                // Only apply when the guard is true.
                match evaluate(guard, &ctx) {
                    Ok(Value::Bool(true)) => expr,
                    _ => continue,
                }
            }
        };
        extract_and_apply_assignments(expr, &old_params, &request.state, &mut new_params)?;
    }

    // 4. Validate postconditions against new state.
    let post_ctx = EvalContext {
        bindings: new_params.clone(),
        old_bindings: Some(old_params),
        instances: request.state.clone(),
    };

    for post in &func.postconditions {
        let (expr, should_check) = match post {
            Postcondition::Always { expr, .. } => (expr, true),
            Postcondition::When { guard, expr, .. } => {
                let guard_result = evaluate(guard, &post_ctx).unwrap_or(Value::Bool(false));
                (expr, guard_result == Value::Bool(true))
            }
        };
        if !should_check {
            continue;
        }
        match evaluate(expr, &post_ctx) {
            Ok(Value::Bool(true)) => {}
            Ok(Value::Bool(false)) => {
                violations.push(Violation {
                    kind: ViolationKind::PostconditionFailed,
                    message: format!("postcondition failed: {}", fmt_expr(expr)),
                });
            }
            Ok(_) => {}
            Err(_) => {
                // Postconditions with quantifiers over new entity types may fail
                // if those instances aren't provided — skip rather than error.
            }
        }
    }

    // 5. Validate invariants.
    let inv_ctx = EvalContext {
        bindings: new_params.clone(),
        old_bindings: None,
        instances: request.state.clone(),
    };

    for inv in &module.invariants {
        match evaluate(&inv.expr, &inv_ctx) {
            Ok(Value::Bool(true)) => {}
            Ok(Value::Bool(false)) => {
                violations.push(Violation {
                    kind: ViolationKind::InvariantViolated,
                    message: format!("invariant '{}' violated", inv.name),
                });
            }
            Ok(_) | Err(_) => {}
        }
    }

    Ok(ActionResult {
        ok: violations.is_empty(),
        new_params,
        violations,
    })
}

/// Extract field assignments from an ensures expression and apply them.
///
/// Recognizes the pattern: `param.field == rhs` where rhs may contain `old()`.
/// Evaluates rhs using old state and sets param.field to the result.
/// For compound expressions (And), recurses into both sides.
fn extract_and_apply_assignments(
    expr: &IrExpr,
    old_params: &HashMap<String, Value>,
    instances: &HashMap<String, Vec<Value>>,
    new_params: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    match expr {
        // a.field == rhs → set a.field = eval(rhs, old_context)
        IrExpr::Compare {
            left,
            op: CmpOp::Eq,
            right,
        } => {
            if let Some((var, field)) = extract_field_path(left)
                && contains_old(right)
            {
                let old_ctx = EvalContext {
                    bindings: old_params.clone(),
                    old_bindings: Some(old_params.clone()),
                    instances: instances.clone(),
                };
                let value = evaluate(right, &old_ctx)?;
                set_field(new_params, &var, &field, value);
            }
            // Also check reversed: rhs == a.field
            if let Some((var, field)) = extract_field_path(right)
                && contains_old(left)
            {
                let old_ctx = EvalContext {
                    bindings: old_params.clone(),
                    old_bindings: Some(old_params.clone()),
                    instances: instances.clone(),
                };
                let value = evaluate(left, &old_ctx)?;
                set_field(new_params, &var, &field, value);
            }
        }
        // Recurse into And (multiple ensures conditions).
        IrExpr::And(left, right) => {
            extract_and_apply_assignments(left, old_params, instances, new_params)?;
            extract_and_apply_assignments(right, old_params, instances, new_params)?;
        }
        // Other patterns (exists, forall, etc.) — not assignable, skip.
        _ => {}
    }
    Ok(())
}

/// Extract `(var_name, field_name)` from a simple field access like `Var("x").field`.
fn extract_field_path(expr: &IrExpr) -> Option<(String, String)> {
    if let IrExpr::FieldAccess { root, field } = expr
        && let IrExpr::Var(var) = root.as_ref()
    {
        return Some((var.clone(), field.clone()));
    }
    None
}

/// Check if an expression tree contains any `old()` reference.
fn contains_old(expr: &IrExpr) -> bool {
    match expr {
        IrExpr::Old(_) => true,
        IrExpr::Compare { left, right, .. }
        | IrExpr::Arithmetic { left, right, .. }
        | IrExpr::And(left, right)
        | IrExpr::Or(left, right)
        | IrExpr::Implies(left, right) => contains_old(left) || contains_old(right),
        IrExpr::Not(inner) => contains_old(inner),
        IrExpr::FieldAccess { root, .. } => contains_old(root),
        IrExpr::Forall { body, .. } | IrExpr::Exists { body, .. } => contains_old(body),
        IrExpr::Call { args, .. } | IrExpr::List(args) => args.iter().any(contains_old),
        IrExpr::Var(_) | IrExpr::Literal(_) => false,
    }
}

/// Set a field on a param value (must be an object).
fn set_field(params: &mut HashMap<String, Value>, var: &str, field: &str, value: Value) {
    if let Some(Value::Object(map)) = params.get_mut(var) {
        map.insert(field.to_string(), value);
    }
}

/// Simple expression formatter for error messages.
fn fmt_expr(expr: &IrExpr) -> String {
    match expr {
        IrExpr::Var(name) => name.clone(),
        IrExpr::Literal(lit) => format!("{lit:?}"),
        IrExpr::FieldAccess { root, field } => format!("{}.{field}", fmt_expr(root)),
        IrExpr::Compare { left, op, right } => {
            let op_str = match op {
                CmpOp::Eq => "==",
                CmpOp::Ne => "!=",
                CmpOp::Lt => "<",
                CmpOp::Gt => ">",
                CmpOp::Le => "<=",
                CmpOp::Ge => ">=",
            };
            format!("{} {op_str} {}", fmt_expr(left), fmt_expr(right))
        }
        IrExpr::And(l, r) => format!("{} && {}", fmt_expr(l), fmt_expr(r)),
        IrExpr::Or(l, r) => format!("{} || {}", fmt_expr(l), fmt_expr(r)),
        IrExpr::Not(inner) => format!("!{}", fmt_expr(inner)),
        IrExpr::Old(inner) => format!("old({})", fmt_expr(inner)),
        _ => "...".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use intent_ir::*;
    use intent_parser::ast::Span;
    use serde_json::json;

    fn empty_trace() -> SourceTrace {
        SourceTrace {
            module: String::new(),
            item: String::new(),
            part: String::new(),
            span: Span { start: 0, end: 0 },
        }
    }

    fn simple_module() -> Module {
        Module {
            name: "Test".into(),
            structs: vec![Struct {
                name: "Account".into(),
                fields: vec![
                    Field {
                        name: "id".into(),
                        ty: IrType::Named("UUID".into()),
                        trace: empty_trace(),
                    },
                    Field {
                        name: "balance".into(),
                        ty: IrType::Decimal(2),
                        trace: empty_trace(),
                    },
                    Field {
                        name: "status".into(),
                        ty: IrType::Union(vec!["Active".into(), "Frozen".into()]),
                        trace: empty_trace(),
                    },
                ],
                trace: empty_trace(),
            }],
            functions: vec![Function {
                name: "Transfer".into(),
                params: vec![
                    Param {
                        name: "from".into(),
                        ty: IrType::Struct("Account".into()),
                        trace: empty_trace(),
                    },
                    Param {
                        name: "to".into(),
                        ty: IrType::Struct("Account".into()),
                        trace: empty_trace(),
                    },
                    Param {
                        name: "amount".into(),
                        ty: IrType::Decimal(2),
                        trace: empty_trace(),
                    },
                ],
                preconditions: vec![
                    // from.status == Active
                    Condition {
                        expr: IrExpr::Compare {
                            left: Box::new(IrExpr::FieldAccess {
                                root: Box::new(IrExpr::Var("from".into())),
                                field: "status".into(),
                            }),
                            op: CmpOp::Eq,
                            right: Box::new(IrExpr::Var("Active".into())),
                        },
                        trace: empty_trace(),
                    },
                    // amount > 0
                    Condition {
                        expr: IrExpr::Compare {
                            left: Box::new(IrExpr::Var("amount".into())),
                            op: CmpOp::Gt,
                            right: Box::new(IrExpr::Literal(IrLiteral::Int(0))),
                        },
                        trace: empty_trace(),
                    },
                    // from.balance >= amount
                    Condition {
                        expr: IrExpr::Compare {
                            left: Box::new(IrExpr::FieldAccess {
                                root: Box::new(IrExpr::Var("from".into())),
                                field: "balance".into(),
                            }),
                            op: CmpOp::Ge,
                            right: Box::new(IrExpr::Var("amount".into())),
                        },
                        trace: empty_trace(),
                    },
                ],
                postconditions: vec![
                    // from.balance == old(from.balance) - amount
                    Postcondition::Always {
                        expr: IrExpr::And(
                            Box::new(IrExpr::Compare {
                                left: Box::new(IrExpr::FieldAccess {
                                    root: Box::new(IrExpr::Var("from".into())),
                                    field: "balance".into(),
                                }),
                                op: CmpOp::Eq,
                                right: Box::new(IrExpr::Arithmetic {
                                    left: Box::new(IrExpr::Old(Box::new(IrExpr::FieldAccess {
                                        root: Box::new(IrExpr::Var("from".into())),
                                        field: "balance".into(),
                                    }))),
                                    op: ArithOp::Sub,
                                    right: Box::new(IrExpr::Var("amount".into())),
                                }),
                            }),
                            // to.balance == old(to.balance) + amount
                            Box::new(IrExpr::Compare {
                                left: Box::new(IrExpr::FieldAccess {
                                    root: Box::new(IrExpr::Var("to".into())),
                                    field: "balance".into(),
                                }),
                                op: CmpOp::Eq,
                                right: Box::new(IrExpr::Arithmetic {
                                    left: Box::new(IrExpr::Old(Box::new(IrExpr::FieldAccess {
                                        root: Box::new(IrExpr::Var("to".into())),
                                        field: "balance".into(),
                                    }))),
                                    op: ArithOp::Add,
                                    right: Box::new(IrExpr::Var("amount".into())),
                                }),
                            }),
                        ),
                        trace: empty_trace(),
                    },
                ],
                properties: vec![],
                trace: empty_trace(),
            }],
            invariants: vec![Invariant {
                name: "NoNegativeBalances".into(),
                expr: IrExpr::Forall {
                    binding: "a".into(),
                    ty: "Account".into(),
                    body: Box::new(IrExpr::Compare {
                        left: Box::new(IrExpr::FieldAccess {
                            root: Box::new(IrExpr::Var("a".into())),
                            field: "balance".into(),
                        }),
                        op: CmpOp::Ge,
                        right: Box::new(IrExpr::Literal(IrLiteral::Int(0))),
                    }),
                },
                trace: empty_trace(),
            }],
            edge_guards: vec![],
        }
    }

    #[test]
    fn execute_valid_transfer() {
        let module = simple_module();
        let request = ActionRequest {
            action: "Transfer".into(),
            params: HashMap::from([
                (
                    "from".into(),
                    json!({"id": "1", "balance": 1000.0, "status": "Active"}),
                ),
                (
                    "to".into(),
                    json!({"id": "2", "balance": 500.0, "status": "Active"}),
                ),
                ("amount".into(), json!(200.0)),
            ]),
            state: HashMap::from([(
                "Account".into(),
                vec![
                    json!({"id": "1", "balance": 1000.0, "status": "Active"}),
                    json!({"id": "2", "balance": 500.0, "status": "Active"}),
                ],
            )]),
        };

        let result = execute_action(&module, &request).unwrap();
        assert!(result.ok, "violations: {:?}", result.violations);
        assert_eq!(result.new_params["from"]["balance"], json!(800.0));
        assert_eq!(result.new_params["to"]["balance"], json!(700.0));
    }

    #[test]
    fn precondition_fails_frozen_account() {
        let module = simple_module();
        let request = ActionRequest {
            action: "Transfer".into(),
            params: HashMap::from([
                (
                    "from".into(),
                    json!({"id": "1", "balance": 1000.0, "status": "Frozen"}),
                ),
                (
                    "to".into(),
                    json!({"id": "2", "balance": 500.0, "status": "Active"}),
                ),
                ("amount".into(), json!(200.0)),
            ]),
            state: HashMap::new(),
        };

        let result = execute_action(&module, &request).unwrap();
        assert!(!result.ok);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].kind, ViolationKind::PreconditionFailed);
        assert!(result.violations[0].message.contains("from.status"));
    }

    #[test]
    fn precondition_fails_insufficient_balance() {
        let module = simple_module();
        let request = ActionRequest {
            action: "Transfer".into(),
            params: HashMap::from([
                (
                    "from".into(),
                    json!({"id": "1", "balance": 50.0, "status": "Active"}),
                ),
                (
                    "to".into(),
                    json!({"id": "2", "balance": 500.0, "status": "Active"}),
                ),
                ("amount".into(), json!(200.0)),
            ]),
            state: HashMap::new(),
        };

        let result = execute_action(&module, &request).unwrap();
        assert!(!result.ok);
        assert!(
            result
                .violations
                .iter()
                .any(|v| v.kind == ViolationKind::PreconditionFailed)
        );
    }

    #[test]
    fn precondition_fails_zero_amount() {
        let module = simple_module();
        let request = ActionRequest {
            action: "Transfer".into(),
            params: HashMap::from([
                (
                    "from".into(),
                    json!({"id": "1", "balance": 1000.0, "status": "Active"}),
                ),
                (
                    "to".into(),
                    json!({"id": "2", "balance": 500.0, "status": "Active"}),
                ),
                ("amount".into(), json!(0)),
            ]),
            state: HashMap::new(),
        };

        let result = execute_action(&module, &request).unwrap();
        assert!(!result.ok);
        assert!(
            result
                .violations
                .iter()
                .any(|v| v.kind == ViolationKind::PreconditionFailed)
        );
    }

    #[test]
    fn invariant_violation_detected() {
        // Module with an invariant that all balances >= 100 (artificially high).
        let module = Module {
            name: "Test".into(),
            structs: vec![],
            functions: vec![Function {
                name: "Withdraw".into(),
                params: vec![Param {
                    name: "account".into(),
                    ty: IrType::Struct("Account".into()),
                    trace: empty_trace(),
                }],
                preconditions: vec![],
                postconditions: vec![],
                properties: vec![],
                trace: empty_trace(),
            }],
            invariants: vec![Invariant {
                name: "MinBalance".into(),
                expr: IrExpr::Forall {
                    binding: "a".into(),
                    ty: "Account".into(),
                    body: Box::new(IrExpr::Compare {
                        left: Box::new(IrExpr::FieldAccess {
                            root: Box::new(IrExpr::Var("a".into())),
                            field: "balance".into(),
                        }),
                        op: CmpOp::Ge,
                        right: Box::new(IrExpr::Literal(IrLiteral::Int(100))),
                    }),
                },
                trace: empty_trace(),
            }],
            edge_guards: vec![],
        };

        let request = ActionRequest {
            action: "Withdraw".into(),
            params: HashMap::from([("account".into(), json!({"balance": 50}))]),
            state: HashMap::from([("Account".into(), vec![json!({"balance": 50})])]),
        };

        let result = execute_action(&module, &request).unwrap();
        assert!(!result.ok);
        assert!(result.violations.iter().any(|v| {
            v.kind == ViolationKind::InvariantViolated && v.message.contains("MinBalance")
        }));
    }

    #[test]
    fn unknown_action_error() {
        let module = simple_module();
        let request = ActionRequest {
            action: "NonExistent".into(),
            params: HashMap::new(),
            state: HashMap::new(),
        };

        assert!(matches!(
            execute_action(&module, &request),
            Err(RuntimeError::UnknownFunction(_))
        ));
    }

    #[test]
    fn edge_guard_blocks_execution() {
        let module = Module {
            name: "Test".into(),
            structs: vec![],
            functions: vec![Function {
                name: "Transfer".into(),
                params: vec![Param {
                    name: "amount".into(),
                    ty: IrType::Decimal(2),
                    trace: empty_trace(),
                }],
                preconditions: vec![],
                postconditions: vec![],
                properties: vec![],
                trace: empty_trace(),
            }],
            invariants: vec![],
            edge_guards: vec![EdgeGuard {
                condition: IrExpr::Compare {
                    left: Box::new(IrExpr::Var("amount".into())),
                    op: CmpOp::Gt,
                    right: Box::new(IrExpr::Literal(IrLiteral::Int(10000))),
                },
                action: "require_approval".into(),
                args: vec![],
                trace: empty_trace(),
            }],
        };

        let request = ActionRequest {
            action: "Transfer".into(),
            params: HashMap::from([("amount".into(), json!(50000))]),
            state: HashMap::new(),
        };

        let result = execute_action(&module, &request).unwrap();
        assert!(!result.ok);
        assert_eq!(result.violations[0].kind, ViolationKind::EdgeGuardTriggered);
    }

    #[test]
    fn when_postcondition_guarded() {
        let module = Module {
            name: "Test".into(),
            structs: vec![],
            functions: vec![Function {
                name: "SetStatus".into(),
                params: vec![
                    Param {
                        name: "account".into(),
                        ty: IrType::Struct("Account".into()),
                        trace: empty_trace(),
                    },
                    Param {
                        name: "freeze".into(),
                        ty: IrType::Named("Bool".into()),
                        trace: empty_trace(),
                    },
                ],
                preconditions: vec![],
                postconditions: vec![Postcondition::When {
                    guard: IrExpr::Var("freeze".into()),
                    // account.status == "Frozen" (no old, so just validation)
                    expr: IrExpr::Compare {
                        left: Box::new(IrExpr::FieldAccess {
                            root: Box::new(IrExpr::Var("account".into())),
                            field: "status".into(),
                        }),
                        op: CmpOp::Eq,
                        right: Box::new(IrExpr::Var("Frozen".into())),
                    },
                    trace: empty_trace(),
                }],
                properties: vec![],
                trace: empty_trace(),
            }],
            invariants: vec![],
            edge_guards: vec![],
        };

        // When guard is false, postcondition is skipped — ok.
        let request = ActionRequest {
            action: "SetStatus".into(),
            params: HashMap::from([
                ("account".into(), json!({"status": "Active"})),
                ("freeze".into(), json!(false)),
            ]),
            state: HashMap::new(),
        };
        let result = execute_action(&module, &request).unwrap();
        assert!(result.ok);

        // When guard is true, postcondition is checked — fails because status is Active.
        let request2 = ActionRequest {
            action: "SetStatus".into(),
            params: HashMap::from([
                ("account".into(), json!({"status": "Active"})),
                ("freeze".into(), json!(true)),
            ]),
            state: HashMap::new(),
        };
        let result2 = execute_action(&module, &request2).unwrap();
        assert!(!result2.ok);
        assert_eq!(
            result2.violations[0].kind,
            ViolationKind::PostconditionFailed
        );
    }
}
