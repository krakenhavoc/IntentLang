//! Test runner for IntentLang spec-level tests.
//!
//! Converts `test` blocks from the AST into runtime `ActionRequest`s,
//! executes them, and checks assertions against the results.

use std::collections::HashMap;

use intent_ir::Module;
use intent_parser::ast::{self, ExprKind, GivenValue, Literal, ThenClause};
use serde_json::Value;

use crate::contract::{ActionRequest, ViolationKind, execute_action};
use crate::error::RuntimeError;
use crate::eval::evaluate;
use crate::value::EvalContext;

/// Result of running a single test.
#[derive(Debug)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub message: Option<String>,
}

/// Run all test declarations against a compiled IR module.
pub fn run_tests(module: &Module, tests: &[&ast::TestDecl]) -> Vec<TestResult> {
    tests.iter().map(|t| run_single_test(module, t)).collect()
}

fn run_single_test(module: &Module, test: &ast::TestDecl) -> TestResult {
    match run_single_test_inner(module, test) {
        Ok(result) => result,
        Err(e) => TestResult {
            name: test.name.clone(),
            passed: false,
            message: Some(format!("runtime error: {e}")),
        },
    }
}

fn run_single_test_inner(
    module: &Module,
    test: &ast::TestDecl,
) -> Result<TestResult, RuntimeError> {
    // 1. Evaluate given bindings to concrete values.
    let mut bindings: HashMap<String, Value> = HashMap::new();
    let mut state: HashMap<String, Vec<Value>> = HashMap::new();

    for binding in &test.given {
        match &binding.value {
            GivenValue::EntityConstructor { type_name, fields } => {
                let obj = fields_to_json(fields, &bindings)?;
                bindings.insert(binding.name.clone(), obj.clone());
                state.entry(type_name.clone()).or_default().push(obj);
            }
            GivenValue::Expr(expr) => {
                let val = ast_expr_to_value(expr, &bindings)?;
                bindings.insert(binding.name.clone(), val);
            }
        }
    }

    // 2. Build ActionRequest from when block.
    let mut params: HashMap<String, Value> = HashMap::new();
    for arg in &test.when_action.args {
        let val = ast_expr_to_value(&arg.value, &bindings)?;
        params.insert(arg.name.clone(), val);
    }

    let request = ActionRequest {
        action: test.when_action.action_name.clone(),
        params,
        state,
    };

    // 3. Execute.
    let result = execute_action(module, &request)?;

    // 4. Check then clause.
    match &test.then {
        ThenClause::Fails(kind_filter, _) => {
            if result.ok {
                return Ok(TestResult {
                    name: test.name.clone(),
                    passed: false,
                    message: Some("expected action to fail, but it succeeded".into()),
                });
            }
            // If a specific kind is requested, check for it.
            if let Some(kind_str) = kind_filter {
                let expected_kind = match kind_str.as_str() {
                    "precondition" => Some(ViolationKind::PreconditionFailed),
                    "postcondition" => Some(ViolationKind::PostconditionFailed),
                    "invariant" => Some(ViolationKind::InvariantViolated),
                    "edge_guard" => Some(ViolationKind::EdgeGuardTriggered),
                    _ => None,
                };
                if let Some(kind) = expected_kind
                    && !result.violations.iter().any(|v| v.kind == kind)
                {
                    return Ok(TestResult {
                        name: test.name.clone(),
                        passed: false,
                        message: Some(format!(
                            "expected {kind_str} violation, got: {}",
                            result
                                .violations
                                .iter()
                                .map(|v| format!("{:?}", v.kind))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )),
                    });
                }
            }
            Ok(TestResult {
                name: test.name.clone(),
                passed: true,
                message: None,
            })
        }
        ThenClause::Asserts(exprs, _) => {
            if !result.ok {
                let msgs: Vec<_> = result
                    .violations
                    .iter()
                    .map(|v| v.message.clone())
                    .collect();
                return Ok(TestResult {
                    name: test.name.clone(),
                    passed: false,
                    message: Some(format!("action failed: {}", msgs.join("; "))),
                });
            }

            // Merge new_params over given bindings for assertion context.
            let mut assert_bindings = bindings;
            for (k, v) in &result.new_params {
                assert_bindings.insert(k.clone(), v.clone());
            }

            // Lower assertion expressions to IR and evaluate.
            for expr in exprs {
                let ir_expr = intent_ir::lower_expr(expr);
                let ctx = EvalContext {
                    bindings: assert_bindings.clone(),
                    old_bindings: None,
                    instances: request.state.clone(),
                };
                match evaluate(&ir_expr, &ctx) {
                    Ok(Value::Bool(true)) => {}
                    Ok(Value::Bool(false)) => {
                        return Ok(TestResult {
                            name: test.name.clone(),
                            passed: false,
                            message: Some(format!("assertion failed: {}", fmt_ast_expr(expr))),
                        });
                    }
                    Ok(other) => {
                        return Ok(TestResult {
                            name: test.name.clone(),
                            passed: false,
                            message: Some(format!(
                                "assertion did not evaluate to bool: {} (got {other:?})",
                                fmt_ast_expr(expr)
                            )),
                        });
                    }
                    Err(e) => {
                        return Ok(TestResult {
                            name: test.name.clone(),
                            passed: false,
                            message: Some(format!(
                                "assertion error: {} ({})",
                                fmt_ast_expr(expr),
                                e
                            )),
                        });
                    }
                }
            }

            Ok(TestResult {
                name: test.name.clone(),
                passed: true,
                message: None,
            })
        }
    }
}

/// Convert entity constructor fields to a JSON object.
fn fields_to_json(
    fields: &[ast::ConstructorField],
    bindings: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let mut map = serde_json::Map::new();
    for field in fields {
        let val = ast_expr_to_value(&field.value, bindings)?;
        map.insert(field.name.clone(), val);
    }
    Ok(Value::Object(map))
}

/// Convert a concrete AST expression to a JSON value.
///
/// Only handles literal values and identifier references — no complex
/// expressions (quantifiers, old(), etc.) are valid in test given/when blocks.
fn ast_expr_to_value(
    expr: &ast::Expr,
    bindings: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match &expr.kind {
        ExprKind::Literal(lit) => match lit {
            Literal::Null => Ok(Value::Null),
            Literal::Bool(b) => Ok(Value::Bool(*b)),
            Literal::Int(n) => Ok(serde_json::json!(*n as f64)),
            Literal::Decimal(s) => {
                let n: f64 = s
                    .parse()
                    .map_err(|_| RuntimeError::DecimalError(s.clone()))?;
                Ok(serde_json::json!(n))
            }
            Literal::String(s) => Ok(Value::String(s.clone())),
        },
        ExprKind::Ident(name) => {
            // Check bindings first (references to given variables).
            if let Some(val) = bindings.get(name) {
                return Ok(val.clone());
            }
            // Uppercase identifiers are union variant labels.
            if name.starts_with(|c: char| c.is_uppercase()) {
                return Ok(Value::String(name.clone()));
            }
            Err(RuntimeError::UnboundVariable(name.clone()))
        }
        ExprKind::List(items) => {
            let vals: Result<Vec<Value>, _> = items
                .iter()
                .map(|e| ast_expr_to_value(e, bindings))
                .collect();
            Ok(Value::Array(vals?))
        }
        ExprKind::Arithmetic { left, op, right } => {
            let l = ast_expr_to_value(left, bindings)?;
            let r = ast_expr_to_value(right, bindings)?;
            let lf = as_f64(&l)?;
            let rf = as_f64(&r)?;
            let result = match op {
                ast::ArithOp::Add => lf + rf,
                ast::ArithOp::Sub => lf - rf,
            };
            Ok(serde_json::json!(result))
        }
        _ => Err(RuntimeError::UnboundVariable(
            "<unsupported expression in test>".into(),
        )),
    }
}

fn as_f64(val: &Value) -> Result<f64, RuntimeError> {
    val.as_f64().ok_or(RuntimeError::TypeError {
        expected: "number".into(),
        got: format!("{val:?}"),
    })
}

/// Simple AST expression formatter for error messages.
fn fmt_ast_expr(expr: &ast::Expr) -> String {
    match &expr.kind {
        ExprKind::Ident(name) => name.clone(),
        ExprKind::Literal(lit) => match lit {
            Literal::Null => "null".into(),
            Literal::Bool(b) => b.to_string(),
            Literal::Int(n) => n.to_string(),
            Literal::Decimal(s) => s.clone(),
            Literal::String(s) => format!("\"{s}\""),
        },
        ExprKind::FieldAccess { root, fields } => {
            format!("{}.{}", fmt_ast_expr(root), fields.join("."))
        }
        ExprKind::Compare { left, op, right } => {
            let op_str = match op {
                ast::CmpOp::Eq => "==",
                ast::CmpOp::Ne => "!=",
                ast::CmpOp::Lt => "<",
                ast::CmpOp::Gt => ">",
                ast::CmpOp::Le => "<=",
                ast::CmpOp::Ge => ">=",
            };
            format!("{} {op_str} {}", fmt_ast_expr(left), fmt_ast_expr(right))
        }
        _ => "...".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runner_pass_via_parse() {
        let src = r#"module Test

entity Account {
  id: UUID
  balance: Decimal(precision: 2)
  status: Active | Frozen
}

action Transfer {
  from: Account
  to: Account
  amount: Decimal(precision: 2)

  requires {
    from.status == Active
    amount > 0
    from.balance >= amount
  }

  ensures {
    from.balance == old(from.balance) - amount
    to.balance == old(to.balance) + amount
  }
}

test "successful transfer" {
  given {
    from = Account { id: "1", balance: 1000.0, status: Active }
    to = Account { id: "2", balance: 500.0, status: Active }
  }
  when Transfer {
    from: from,
    to: to,
    amount: 200.0
  }
  then {
    from.balance == 800.0
    to.balance == 700.0
  }
}

test "insufficient funds" {
  given {
    from = Account { id: "1", balance: 50.0, status: Active }
    to = Account { id: "2", balance: 500.0, status: Active }
  }
  when Transfer {
    from: from,
    to: to,
    amount: 200.0
  }
  then fails
}
"#;
        let file = intent_parser::parse_file(src).unwrap();
        let ir = intent_ir::lower_file(&file);

        let tests: Vec<_> = file
            .items
            .iter()
            .filter_map(|i| {
                if let ast::TopLevelItem::Test(t) = i {
                    Some(t)
                } else {
                    None
                }
            })
            .collect();

        let results = run_tests(&ir, &tests);
        assert_eq!(results.len(), 2);
        assert!(results[0].passed, "test 0 failed: {:?}", results[0].message);
        assert!(results[1].passed, "test 1 failed: {:?}", results[1].message);
    }

    #[test]
    fn test_runner_then_fails_precondition() {
        let src = r#"module Test

entity Account {
  id: UUID
  balance: Decimal(precision: 2)
  status: Active | Frozen
}

action Transfer {
  from: Account
  to: Account
  amount: Decimal(precision: 2)

  requires {
    from.status == Active
    amount > 0
    from.balance >= amount
  }

  ensures {
    from.balance == old(from.balance) - amount
    to.balance == old(to.balance) + amount
  }
}

test "frozen account" {
  given {
    from = Account { id: "1", balance: 1000.0, status: Frozen }
    to = Account { id: "2", balance: 500.0, status: Active }
  }
  when Transfer {
    from: from,
    to: to,
    amount: 200.0
  }
  then fails precondition
}
"#;
        let file = intent_parser::parse_file(src).unwrap();
        let ir = intent_ir::lower_file(&file);

        let tests: Vec<_> = file
            .items
            .iter()
            .filter_map(|i| {
                if let ast::TopLevelItem::Test(t) = i {
                    Some(t)
                } else {
                    None
                }
            })
            .collect();

        let results = run_tests(&ir, &tests);
        assert_eq!(results.len(), 1);
        assert!(results[0].passed, "test 0 failed: {:?}", results[0].message);
    }
}
