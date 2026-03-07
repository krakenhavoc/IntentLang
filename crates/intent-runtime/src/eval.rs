use intent_ir::{ArithOp, CmpOp, IrExpr, IrLiteral};
use serde_json::Value;

use crate::error::RuntimeError;
use crate::value::EvalContext;

/// Evaluate an IR expression against a context of concrete values.
///
/// Returns a JSON `Value` on success. Boolean expressions return `Value::Bool`,
/// numeric expressions return `Value::Number`, etc.
pub fn evaluate(expr: &IrExpr, ctx: &EvalContext) -> Result<Value, RuntimeError> {
    match expr {
        IrExpr::Literal(lit) => eval_literal(lit),
        IrExpr::Var(name) => eval_var(name, ctx),
        IrExpr::FieldAccess { root, field } => eval_field_access(root, field, ctx),
        IrExpr::Compare { left, op, right } => eval_compare(left, *op, right, ctx),
        IrExpr::Arithmetic { left, op, right } => eval_arithmetic(left, *op, right, ctx),
        IrExpr::And(left, right) => eval_and(left, right, ctx),
        IrExpr::Or(left, right) => eval_or(left, right, ctx),
        IrExpr::Not(inner) => eval_not(inner, ctx),
        IrExpr::Implies(left, right) => eval_implies(left, right, ctx),
        IrExpr::Old(inner) => eval_old(inner, ctx),
        IrExpr::Forall { binding, ty, body } => eval_forall(binding, ty, body, ctx),
        IrExpr::Exists { binding, ty, body } => eval_exists(binding, ty, body, ctx),
        IrExpr::Call { name, args } => eval_call(name, args, ctx),
        IrExpr::List(items) => eval_list(items, ctx),
    }
}

fn eval_literal(lit: &IrLiteral) -> Result<Value, RuntimeError> {
    match lit {
        IrLiteral::Null => Ok(Value::Null),
        IrLiteral::Bool(b) => Ok(Value::Bool(*b)),
        IrLiteral::Int(n) => Ok(json_number_i64(*n)),
        IrLiteral::Decimal(s) => parse_decimal_value(s),
        IrLiteral::String(s) => Ok(Value::String(s.clone())),
    }
}

fn eval_var(name: &str, ctx: &EvalContext) -> Result<Value, RuntimeError> {
    // Check bindings first
    if let Some(val) = ctx.bindings.get(name) {
        return Ok(val.clone());
    }
    // Uppercase identifiers are union variant labels — return as strings
    if name.starts_with(|c: char| c.is_uppercase()) {
        return Ok(Value::String(name.to_string()));
    }
    Err(RuntimeError::UnboundVariable(name.to_string()))
}

fn eval_field_access(root: &IrExpr, field: &str, ctx: &EvalContext) -> Result<Value, RuntimeError> {
    let root_val = evaluate(root, ctx)?;
    match &root_val {
        Value::Object(map) => map.get(field).cloned().ok_or(RuntimeError::FieldNotFound {
            field: field.to_string(),
        }),
        Value::Null => Err(RuntimeError::FieldNotFound {
            field: field.to_string(),
        }),
        _ => Err(RuntimeError::NotAnObject),
    }
}

fn eval_compare(
    left: &IrExpr,
    op: CmpOp,
    right: &IrExpr,
    ctx: &EvalContext,
) -> Result<Value, RuntimeError> {
    let lhs = evaluate(left, ctx)?;
    let rhs = evaluate(right, ctx)?;

    let result = match op {
        CmpOp::Eq => values_equal(&lhs, &rhs),
        CmpOp::Ne => !values_equal(&lhs, &rhs),
        CmpOp::Lt => values_compare(&lhs, &rhs)?.is_some_and(|o| o.is_lt()),
        CmpOp::Gt => values_compare(&lhs, &rhs)?.is_some_and(|o| o.is_gt()),
        CmpOp::Le => values_compare(&lhs, &rhs)?.is_some_and(|o| o.is_le()),
        CmpOp::Ge => values_compare(&lhs, &rhs)?.is_some_and(|o| o.is_ge()),
    };
    Ok(Value::Bool(result))
}

fn eval_arithmetic(
    left: &IrExpr,
    op: ArithOp,
    right: &IrExpr,
    ctx: &EvalContext,
) -> Result<Value, RuntimeError> {
    let lhs = evaluate(left, ctx)?;
    let rhs = evaluate(right, ctx)?;

    match (&lhs, &rhs) {
        (Value::Number(a), Value::Number(b)) => {
            let af = as_f64(a);
            let bf = as_f64(b);
            let result = match op {
                ArithOp::Add => af + bf,
                ArithOp::Sub => af - bf,
            };
            Ok(json_number_f64(result))
        }
        _ => Err(RuntimeError::TypeError {
            expected: "number".into(),
            got: type_name(&lhs),
        }),
    }
}

fn eval_and(left: &IrExpr, right: &IrExpr, ctx: &EvalContext) -> Result<Value, RuntimeError> {
    let lhs = as_bool(&evaluate(left, ctx)?)?;
    if !lhs {
        return Ok(Value::Bool(false)); // short-circuit
    }
    let rhs = as_bool(&evaluate(right, ctx)?)?;
    Ok(Value::Bool(rhs))
}

fn eval_or(left: &IrExpr, right: &IrExpr, ctx: &EvalContext) -> Result<Value, RuntimeError> {
    let lhs = as_bool(&evaluate(left, ctx)?)?;
    if lhs {
        return Ok(Value::Bool(true)); // short-circuit
    }
    let rhs = as_bool(&evaluate(right, ctx)?)?;
    Ok(Value::Bool(rhs))
}

fn eval_not(inner: &IrExpr, ctx: &EvalContext) -> Result<Value, RuntimeError> {
    let val = as_bool(&evaluate(inner, ctx)?)?;
    Ok(Value::Bool(!val))
}

fn eval_implies(left: &IrExpr, right: &IrExpr, ctx: &EvalContext) -> Result<Value, RuntimeError> {
    // a => b  ≡  !a || b
    let lhs = as_bool(&evaluate(left, ctx)?)?;
    if !lhs {
        return Ok(Value::Bool(true));
    }
    let rhs = as_bool(&evaluate(right, ctx)?)?;
    Ok(Value::Bool(rhs))
}

fn eval_old(inner: &IrExpr, ctx: &EvalContext) -> Result<Value, RuntimeError> {
    let old_bindings = ctx
        .old_bindings
        .as_ref()
        .ok_or(RuntimeError::OldWithoutContext)?;

    let old_ctx = EvalContext {
        bindings: old_bindings.clone(),
        old_bindings: None,
        instances: ctx.instances.clone(),
    };
    evaluate(inner, &old_ctx)
}

fn eval_forall(
    binding: &str,
    ty: &str,
    body: &IrExpr,
    ctx: &EvalContext,
) -> Result<Value, RuntimeError> {
    let instances = ctx
        .instances
        .get(ty)
        .ok_or_else(|| RuntimeError::NoInstances(ty.to_string()))?;

    for instance in instances {
        let child_ctx = ctx.with_binding(binding.to_string(), instance.clone());
        let result = as_bool(&evaluate(body, &child_ctx)?)?;
        if !result {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn eval_exists(
    binding: &str,
    ty: &str,
    body: &IrExpr,
    ctx: &EvalContext,
) -> Result<Value, RuntimeError> {
    let instances = ctx
        .instances
        .get(ty)
        .ok_or_else(|| RuntimeError::NoInstances(ty.to_string()))?;

    for instance in instances {
        let child_ctx = ctx.with_binding(binding.to_string(), instance.clone());
        let result = as_bool(&evaluate(body, &child_ctx)?)?;
        if result {
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

fn eval_call(name: &str, args: &[IrExpr], ctx: &EvalContext) -> Result<Value, RuntimeError> {
    let evaluated_args: Vec<Value> = args
        .iter()
        .map(|a| evaluate(a, ctx))
        .collect::<Result<_, _>>()?;

    // Built-in functions
    match name {
        "len" => {
            if let Some(val) = evaluated_args.first() {
                match val {
                    Value::Array(arr) => Ok(json_number_i64(arr.len() as i64)),
                    Value::String(s) => Ok(json_number_i64(s.len() as i64)),
                    _ => Err(RuntimeError::TypeError {
                        expected: "array or string".into(),
                        got: type_name(val),
                    }),
                }
            } else {
                Err(RuntimeError::TypeError {
                    expected: "1 argument".into(),
                    got: "0 arguments".into(),
                })
            }
        }
        "now" => Ok(Value::String("now()".to_string())),
        _ => Err(RuntimeError::UnknownFunction(name.to_string())),
    }
}

fn eval_list(items: &[IrExpr], ctx: &EvalContext) -> Result<Value, RuntimeError> {
    let values: Vec<Value> = items
        .iter()
        .map(|i| evaluate(i, ctx))
        .collect::<Result<_, _>>()?;
    Ok(Value::Array(values))
}

// ── Helpers ──────────────────────────────────────────────────

fn as_bool(val: &Value) -> Result<bool, RuntimeError> {
    match val {
        Value::Bool(b) => Ok(*b),
        _ => Err(RuntimeError::TypeError {
            expected: "bool".into(),
            got: type_name(val),
        }),
    }
}

fn as_f64(n: &serde_json::Number) -> f64 {
    n.as_f64().unwrap_or(0.0)
}

fn json_number_i64(n: i64) -> Value {
    Value::Number(serde_json::Number::from(n))
}

fn json_number_f64(n: f64) -> Value {
    serde_json::Number::from_f64(n).map_or(Value::Null, Value::Number)
}

fn parse_decimal_value(s: &str) -> Result<Value, RuntimeError> {
    s.parse::<f64>()
        .map(json_number_f64)
        .map_err(|_| RuntimeError::DecimalError(format!("invalid decimal: {s}")))
}

fn type_name(val: &Value) -> String {
    match val {
        Value::Null => "null".into(),
        Value::Bool(_) => "bool".into(),
        Value::Number(_) => "number".into(),
        Value::String(_) => "string".into(),
        Value::Array(_) => "array".into(),
        Value::Object(_) => "object".into(),
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => as_f64(a) == as_f64(b),
        _ => a == b,
    }
}

fn values_compare(a: &Value, b: &Value) -> Result<Option<std::cmp::Ordering>, RuntimeError> {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => Ok(as_f64(a).partial_cmp(&as_f64(b))),
        (Value::String(a), Value::String(b)) => Ok(Some(a.cmp(b))),
        _ => Err(RuntimeError::TypeError {
            expected: "comparable types (number or string)".into(),
            got: format!("{} vs {}", type_name(a), type_name(b)),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn empty_ctx() -> EvalContext {
        EvalContext::new()
    }

    fn ctx_with(bindings: Vec<(&str, Value)>) -> EvalContext {
        let mut ctx = EvalContext::new();
        for (k, v) in bindings {
            ctx.bindings.insert(k.to_string(), v);
        }
        ctx
    }

    // ── Literals ──────────────────────────────────────────

    #[test]
    fn eval_literal_null() {
        let expr = IrExpr::Literal(IrLiteral::Null);
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), Value::Null);
    }

    #[test]
    fn eval_literal_bool() {
        let expr = IrExpr::Literal(IrLiteral::Bool(true));
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), json!(true));
    }

    #[test]
    fn eval_literal_int() {
        let expr = IrExpr::Literal(IrLiteral::Int(42));
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), json!(42));
    }

    #[test]
    fn eval_literal_decimal() {
        let expr = IrExpr::Literal(IrLiteral::Decimal("10.50".into()));
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), json!(10.50));
    }

    #[test]
    fn eval_literal_string() {
        let expr = IrExpr::Literal(IrLiteral::String("hello".into()));
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), json!("hello"));
    }

    // ── Variables ─────────────────────────────────────────

    #[test]
    fn eval_bound_variable() {
        let ctx = ctx_with(vec![("amount", json!(100))]);
        let expr = IrExpr::Var("amount".into());
        assert_eq!(evaluate(&expr, &ctx).unwrap(), json!(100));
    }

    #[test]
    fn eval_unbound_variable_error() {
        let expr = IrExpr::Var("unknown".into());
        assert_eq!(
            evaluate(&expr, &empty_ctx()),
            Err(RuntimeError::UnboundVariable("unknown".into()))
        );
    }

    #[test]
    fn eval_union_variant_as_string() {
        let expr = IrExpr::Var("Active".into());
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), json!("Active"));
    }

    // ── Field Access ──────────────────────────────────────

    #[test]
    fn eval_field_access() {
        let ctx = ctx_with(vec![("account", json!({"balance": 500, "name": "Alice"}))]);
        let expr = IrExpr::FieldAccess {
            root: Box::new(IrExpr::Var("account".into())),
            field: "balance".into(),
        };
        assert_eq!(evaluate(&expr, &ctx).unwrap(), json!(500));
    }

    #[test]
    fn eval_nested_field_access() {
        let ctx = ctx_with(vec![("user", json!({"profile": {"email": "a@b.com"}}))]);
        let expr = IrExpr::FieldAccess {
            root: Box::new(IrExpr::FieldAccess {
                root: Box::new(IrExpr::Var("user".into())),
                field: "profile".into(),
            }),
            field: "email".into(),
        };
        assert_eq!(evaluate(&expr, &ctx).unwrap(), json!("a@b.com"));
    }

    #[test]
    fn eval_field_not_found() {
        let ctx = ctx_with(vec![("obj", json!({"a": 1}))]);
        let expr = IrExpr::FieldAccess {
            root: Box::new(IrExpr::Var("obj".into())),
            field: "missing".into(),
        };
        assert_eq!(
            evaluate(&expr, &ctx),
            Err(RuntimeError::FieldNotFound {
                field: "missing".into()
            })
        );
    }

    #[test]
    fn eval_field_access_on_non_object() {
        let ctx = ctx_with(vec![("x", json!(42))]);
        let expr = IrExpr::FieldAccess {
            root: Box::new(IrExpr::Var("x".into())),
            field: "f".into(),
        };
        assert_eq!(evaluate(&expr, &ctx), Err(RuntimeError::NotAnObject));
    }

    // ── Comparison ────────────────────────────────────────

    #[test]
    fn eval_compare_eq() {
        let ctx = ctx_with(vec![("a", json!(5)), ("b", json!(5))]);
        let expr = IrExpr::Compare {
            left: Box::new(IrExpr::Var("a".into())),
            op: CmpOp::Eq,
            right: Box::new(IrExpr::Var("b".into())),
        };
        assert_eq!(evaluate(&expr, &ctx).unwrap(), json!(true));
    }

    #[test]
    fn eval_compare_ne() {
        let ctx = ctx_with(vec![("a", json!(5)), ("b", json!(3))]);
        let expr = IrExpr::Compare {
            left: Box::new(IrExpr::Var("a".into())),
            op: CmpOp::Ne,
            right: Box::new(IrExpr::Var("b".into())),
        };
        assert_eq!(evaluate(&expr, &ctx).unwrap(), json!(true));
    }

    #[test]
    fn eval_compare_gt() {
        let ctx = ctx_with(vec![("a", json!(10)), ("b", json!(5))]);
        let expr = IrExpr::Compare {
            left: Box::new(IrExpr::Var("a".into())),
            op: CmpOp::Gt,
            right: Box::new(IrExpr::Var("b".into())),
        };
        assert_eq!(evaluate(&expr, &ctx).unwrap(), json!(true));
    }

    #[test]
    fn eval_compare_le() {
        let expr = IrExpr::Compare {
            left: Box::new(IrExpr::Literal(IrLiteral::Int(3))),
            op: CmpOp::Le,
            right: Box::new(IrExpr::Literal(IrLiteral::Int(3))),
        };
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), json!(true));
    }

    #[test]
    fn eval_compare_strings() {
        let expr = IrExpr::Compare {
            left: Box::new(IrExpr::Literal(IrLiteral::String("apple".into()))),
            op: CmpOp::Lt,
            right: Box::new(IrExpr::Literal(IrLiteral::String("banana".into()))),
        };
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), json!(true));
    }

    #[test]
    fn eval_compare_union_variant() {
        // status == Active where status is stored as string "Active"
        let ctx = ctx_with(vec![("account", json!({"status": "Active"}))]);
        let expr = IrExpr::Compare {
            left: Box::new(IrExpr::FieldAccess {
                root: Box::new(IrExpr::Var("account".into())),
                field: "status".into(),
            }),
            op: CmpOp::Eq,
            right: Box::new(IrExpr::Var("Active".into())),
        };
        assert_eq!(evaluate(&expr, &ctx).unwrap(), json!(true));
    }

    // ── Arithmetic ────────────────────────────────────────

    #[test]
    fn eval_arithmetic_add() {
        let expr = IrExpr::Arithmetic {
            left: Box::new(IrExpr::Literal(IrLiteral::Int(3))),
            op: ArithOp::Add,
            right: Box::new(IrExpr::Literal(IrLiteral::Int(4))),
        };
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), json!(7.0));
    }

    #[test]
    fn eval_arithmetic_sub() {
        let expr = IrExpr::Arithmetic {
            left: Box::new(IrExpr::Literal(IrLiteral::Int(10))),
            op: ArithOp::Sub,
            right: Box::new(IrExpr::Literal(IrLiteral::Int(3))),
        };
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), json!(7.0));
    }

    #[test]
    fn eval_arithmetic_type_error() {
        let expr = IrExpr::Arithmetic {
            left: Box::new(IrExpr::Literal(IrLiteral::String("x".into()))),
            op: ArithOp::Add,
            right: Box::new(IrExpr::Literal(IrLiteral::Int(1))),
        };
        assert!(matches!(
            evaluate(&expr, &empty_ctx()),
            Err(RuntimeError::TypeError { .. })
        ));
    }

    // ── Logical ───────────────────────────────────────────

    #[test]
    fn eval_and_true() {
        let expr = IrExpr::And(
            Box::new(IrExpr::Literal(IrLiteral::Bool(true))),
            Box::new(IrExpr::Literal(IrLiteral::Bool(true))),
        );
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), json!(true));
    }

    #[test]
    fn eval_and_short_circuit() {
        // false && (error) should not evaluate the right side
        let expr = IrExpr::And(
            Box::new(IrExpr::Literal(IrLiteral::Bool(false))),
            Box::new(IrExpr::Var("nonexistent".into())),
        );
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), json!(false));
    }

    #[test]
    fn eval_or_short_circuit() {
        // true || (error) should not evaluate the right side
        let expr = IrExpr::Or(
            Box::new(IrExpr::Literal(IrLiteral::Bool(true))),
            Box::new(IrExpr::Var("nonexistent".into())),
        );
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), json!(true));
    }

    #[test]
    fn eval_not() {
        let expr = IrExpr::Not(Box::new(IrExpr::Literal(IrLiteral::Bool(false))));
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), json!(true));
    }

    #[test]
    fn eval_implies_true_antecedent() {
        // true => true == true
        let expr = IrExpr::Implies(
            Box::new(IrExpr::Literal(IrLiteral::Bool(true))),
            Box::new(IrExpr::Literal(IrLiteral::Bool(true))),
        );
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), json!(true));
    }

    #[test]
    fn eval_implies_false_antecedent() {
        // false => anything == true (vacuously true)
        let expr = IrExpr::Implies(
            Box::new(IrExpr::Literal(IrLiteral::Bool(false))),
            Box::new(IrExpr::Var("nonexistent".into())),
        );
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), json!(true));
    }

    // ── old() ─────────────────────────────────────────────

    #[test]
    fn eval_old_with_context() {
        let mut ctx = ctx_with(vec![("balance", json!(500))]);
        let mut old = HashMap::new();
        old.insert("balance".to_string(), json!(1000));
        ctx.old_bindings = Some(old);

        let expr = IrExpr::Old(Box::new(IrExpr::Var("balance".into())));
        assert_eq!(evaluate(&expr, &ctx).unwrap(), json!(1000));
    }

    #[test]
    fn eval_old_without_context() {
        let expr = IrExpr::Old(Box::new(IrExpr::Var("x".into())));
        assert_eq!(
            evaluate(&expr, &empty_ctx()),
            Err(RuntimeError::OldWithoutContext)
        );
    }

    #[test]
    fn eval_old_field_access() {
        // old(account.balance)
        let mut ctx = ctx_with(vec![("account", json!({"balance": 200}))]);
        let mut old = HashMap::new();
        old.insert("account".to_string(), json!({"balance": 500}));
        ctx.old_bindings = Some(old);

        let expr = IrExpr::Old(Box::new(IrExpr::FieldAccess {
            root: Box::new(IrExpr::Var("account".into())),
            field: "balance".into(),
        }));
        assert_eq!(evaluate(&expr, &ctx).unwrap(), json!(500));
    }

    // ── Quantifiers ───────────────────────────────────────

    #[test]
    fn eval_forall_true() {
        // forall a: Account => a.balance >= 0
        let mut ctx = EvalContext::new();
        ctx.instances.insert(
            "Account".into(),
            vec![
                json!({"balance": 100}),
                json!({"balance": 0}),
                json!({"balance": 50}),
            ],
        );

        let expr = IrExpr::Forall {
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
        };
        assert_eq!(evaluate(&expr, &ctx).unwrap(), json!(true));
    }

    #[test]
    fn eval_forall_false() {
        let mut ctx = EvalContext::new();
        ctx.instances.insert(
            "Account".into(),
            vec![
                json!({"balance": 100}),
                json!({"balance": -5}), // violates
            ],
        );

        let expr = IrExpr::Forall {
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
        };
        assert_eq!(evaluate(&expr, &ctx).unwrap(), json!(false));
    }

    #[test]
    fn eval_exists_true() {
        let mut ctx = EvalContext::new();
        ctx.instances.insert(
            "Record".into(),
            vec![json!({"status": "Pending"}), json!({"status": "Completed"})],
        );

        let expr = IrExpr::Exists {
            binding: "r".into(),
            ty: "Record".into(),
            body: Box::new(IrExpr::Compare {
                left: Box::new(IrExpr::FieldAccess {
                    root: Box::new(IrExpr::Var("r".into())),
                    field: "status".into(),
                }),
                op: CmpOp::Eq,
                right: Box::new(IrExpr::Var("Completed".into())),
            }),
        };
        assert_eq!(evaluate(&expr, &ctx).unwrap(), json!(true));
    }

    #[test]
    fn eval_exists_false() {
        let mut ctx = EvalContext::new();
        ctx.instances
            .insert("Record".into(), vec![json!({"status": "Pending"})]);

        let expr = IrExpr::Exists {
            binding: "r".into(),
            ty: "Record".into(),
            body: Box::new(IrExpr::Compare {
                left: Box::new(IrExpr::FieldAccess {
                    root: Box::new(IrExpr::Var("r".into())),
                    field: "status".into(),
                }),
                op: CmpOp::Eq,
                right: Box::new(IrExpr::Var("Completed".into())),
            }),
        };
        assert_eq!(evaluate(&expr, &ctx).unwrap(), json!(false));
    }

    #[test]
    fn eval_forall_no_instances() {
        let expr = IrExpr::Forall {
            binding: "x".into(),
            ty: "Missing".into(),
            body: Box::new(IrExpr::Literal(IrLiteral::Bool(true))),
        };
        assert_eq!(
            evaluate(&expr, &empty_ctx()),
            Err(RuntimeError::NoInstances("Missing".into()))
        );
    }

    // ── List ──────────────────────────────────────────────

    #[test]
    fn eval_list_literal() {
        let expr = IrExpr::List(vec![
            IrExpr::Literal(IrLiteral::Int(1)),
            IrExpr::Literal(IrLiteral::Int(2)),
            IrExpr::Literal(IrLiteral::Int(3)),
        ]);
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), json!([1, 2, 3]));
    }

    // ── Call ──────────────────────────────────────────────

    #[test]
    fn eval_call_len_array() {
        let ctx = ctx_with(vec![("items", json!([1, 2, 3]))]);
        let expr = IrExpr::Call {
            name: "len".into(),
            args: vec![IrExpr::Var("items".into())],
        };
        assert_eq!(evaluate(&expr, &ctx).unwrap(), json!(3));
    }

    #[test]
    fn eval_call_len_string() {
        let expr = IrExpr::Call {
            name: "len".into(),
            args: vec![IrExpr::Literal(IrLiteral::String("hello".into()))],
        };
        assert_eq!(evaluate(&expr, &empty_ctx()).unwrap(), json!(5));
    }

    #[test]
    fn eval_call_unknown() {
        let expr = IrExpr::Call {
            name: "bogus".into(),
            args: vec![],
        };
        assert_eq!(
            evaluate(&expr, &empty_ctx()),
            Err(RuntimeError::UnknownFunction("bogus".into()))
        );
    }

    // ── Complex expressions ───────────────────────────────

    #[test]
    fn eval_postcondition_with_old() {
        // from.balance == old(from.balance) - amount
        let mut ctx = ctx_with(vec![
            ("from", json!({"balance": 800})),
            ("amount", json!(200)),
        ]);
        let mut old = HashMap::new();
        old.insert("from".to_string(), json!({"balance": 1000}));
        ctx.old_bindings = Some(old);

        let expr = IrExpr::Compare {
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
        };
        assert_eq!(evaluate(&expr, &ctx).unwrap(), json!(true));
    }

    #[test]
    fn eval_nested_forall_implies() {
        // forall a: Account => forall b: Account =>
        //   a.id != b.id => a.name != b.name
        let mut ctx = EvalContext::new();
        ctx.instances.insert(
            "Account".into(),
            vec![
                json!({"id": "1", "name": "Alice"}),
                json!({"id": "2", "name": "Bob"}),
            ],
        );

        let expr = IrExpr::Forall {
            binding: "a".into(),
            ty: "Account".into(),
            body: Box::new(IrExpr::Forall {
                binding: "b".into(),
                ty: "Account".into(),
                body: Box::new(IrExpr::Implies(
                    Box::new(IrExpr::Compare {
                        left: Box::new(IrExpr::FieldAccess {
                            root: Box::new(IrExpr::Var("a".into())),
                            field: "id".into(),
                        }),
                        op: CmpOp::Ne,
                        right: Box::new(IrExpr::FieldAccess {
                            root: Box::new(IrExpr::Var("b".into())),
                            field: "id".into(),
                        }),
                    }),
                    Box::new(IrExpr::Compare {
                        left: Box::new(IrExpr::FieldAccess {
                            root: Box::new(IrExpr::Var("a".into())),
                            field: "name".into(),
                        }),
                        op: CmpOp::Ne,
                        right: Box::new(IrExpr::FieldAccess {
                            root: Box::new(IrExpr::Var("b".into())),
                            field: "name".into(),
                        }),
                    }),
                )),
            }),
        };
        assert_eq!(evaluate(&expr, &ctx).unwrap(), json!(true));
    }
}
