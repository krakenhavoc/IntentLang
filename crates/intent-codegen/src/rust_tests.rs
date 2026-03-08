//! Rust contract test harness generator.
//!
//! Translates `test` blocks from IntentLang specs into a `#[cfg(test)]`
//! module with executable Rust tests. Entity parameters are passed as
//! `&mut` references so postconditions can be verified after mutation.

use std::collections::HashMap;

use intent_parser::ast::{self, CmpOp, ExprKind, Literal, TypeKind};

use crate::test_harness::slugify;
use crate::to_snake_case;

// ── Entity metadata ──────────────────────────────────────────

/// Field-level metadata for type-aware code generation.
struct FieldMeta {
    /// Simple type name (e.g. "UUID", "Decimal", "String").
    simple_type: Option<String>,
    /// For union-typed fields: (enum_name, [variant_names]).
    union_info: Option<(String, Vec<String>)>,
}

/// Per-entity field metadata.
struct EntityInfo {
    fields: HashMap<String, FieldMeta>,
}

// ── Public entry point ───────────────────────────────────────

/// Generate a Rust `#[cfg(test)]` module from spec test blocks.
pub fn generate(file: &ast::File) -> String {
    let tests: Vec<_> = file
        .items
        .iter()
        .filter_map(|i| match i {
            ast::TopLevelItem::Test(t) => Some(t),
            _ => None,
        })
        .collect();

    if tests.is_empty() {
        return String::new();
    }

    let entities = collect_entities(file);
    let mut out = String::new();

    out.push_str("#[cfg(test)]\n");
    out.push_str("mod contract_tests {\n");
    out.push_str("    use super::*;\n");
    if uses_type(file, "Decimal") {
        out.push_str("    use std::str::FromStr;\n");
    }
    out.push('\n');

    for test in &tests {
        generate_spec_test(&mut out, test, file, &entities);
    }

    out.push_str("}\n");
    out
}

// ── Entity collection ────────────────────────────────────────

fn collect_entities(file: &ast::File) -> HashMap<String, EntityInfo> {
    let mut map = HashMap::new();

    for item in &file.items {
        if let ast::TopLevelItem::Entity(entity) = item {
            let mut fields = HashMap::new();
            for field in &entity.fields {
                let simple_type = simple_type_name(&field.ty.ty);
                let union_info = if let TypeKind::Union(variants) = &field.ty.ty {
                    let enum_name = format!("{}{}", entity.name, capitalize(&field.name));
                    let names: Vec<String> = variants
                        .iter()
                        .filter_map(|v| {
                            if let TypeKind::Simple(n) = v {
                                Some(n.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    Some((enum_name, names))
                } else {
                    None
                };
                fields.insert(
                    field.name.clone(),
                    FieldMeta {
                        simple_type,
                        union_info,
                    },
                );
            }
            map.insert(entity.name.clone(), EntityInfo { fields });
        }
    }

    map
}

fn simple_type_name(kind: &TypeKind) -> Option<String> {
    match kind {
        TypeKind::Simple(name) => Some(name.clone()),
        TypeKind::Parameterized { name, .. } => Some(name.clone()),
        _ => None,
    }
}

fn uses_type(file: &ast::File, target: &str) -> bool {
    for item in &file.items {
        match item {
            ast::TopLevelItem::Entity(e) => {
                for f in &e.fields {
                    if type_matches(&f.ty.ty, target) {
                        return true;
                    }
                }
            }
            ast::TopLevelItem::Action(a) => {
                for p in &a.params {
                    if type_matches(&p.ty.ty, target) {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}

fn type_matches(kind: &TypeKind, target: &str) -> bool {
    match kind {
        TypeKind::Simple(n) | TypeKind::Parameterized { name: n, .. } => n == target,
        _ => false,
    }
}

// ── Test generation ──────────────────────────────────────────

fn generate_spec_test(
    out: &mut String,
    test: &ast::TestDecl,
    file: &ast::File,
    entities: &HashMap<String, EntityInfo>,
) {
    let test_name = slugify(&test.name);
    let mut given_types: HashMap<String, String> = HashMap::new();

    out.push_str(&format!("    /// Spec test: \"{}\"\n", test.name));
    out.push_str("    #[test]\n");
    out.push_str(&format!("    fn test_{test_name}() {{\n"));

    // Given bindings
    for binding in &test.given {
        generate_binding(out, binding, entities, &mut given_types);
    }
    out.push('\n');

    // When — look up the action for parameter info
    let action = file.items.iter().find_map(|i| match i {
        ast::TopLevelItem::Action(a) if a.name == test.when_action.action_name => Some(a),
        _ => None,
    });

    generate_call(out, &test.when_action, action, &given_types, entities);

    // Then
    match &test.then {
        ast::ThenClause::Asserts(exprs, _) => {
            out.push_str("        assert!(result.is_ok(), \"expected action to succeed\");\n");
            for expr in exprs {
                generate_assertion(out, expr, &given_types, entities);
            }
        }
        ast::ThenClause::Fails(kind, _) => {
            let msg = match kind {
                Some(k) => format!("expected action to fail: {k}"),
                None => "expected action to fail".to_string(),
            };
            out.push_str(&format!("        assert!(result.is_err(), \"{msg}\");\n"));
        }
    }

    out.push_str("    }\n\n");
}

// ── Given bindings ───────────────────────────────────────────

fn generate_binding(
    out: &mut String,
    binding: &ast::GivenBinding,
    entities: &HashMap<String, EntityInfo>,
    given_types: &mut HashMap<String, String>,
) {
    match &binding.value {
        ast::GivenValue::EntityConstructor { type_name, fields } => {
            given_types.insert(binding.name.clone(), type_name.clone());
            let entity = entities.get(type_name);

            out.push_str(&format!(
                "        let mut {} = {} {{\n",
                binding.name, type_name
            ));
            for field in fields {
                let field_meta = entity.and_then(|e| e.fields.get(&field.name));
                let value = field_value_to_rust(&field.value, field_meta);
                out.push_str(&format!(
                    "            {}: {},\n",
                    safe_field(&field.name),
                    value
                ));
            }
            out.push_str("        };\n");
        }
        ast::GivenValue::Expr(expr) => {
            let value = expr_to_rust(expr);
            out.push_str(&format!("        let {} = {};\n", binding.name, value));
        }
    }
}

/// Convert a field value expression to Rust using type metadata.
fn field_value_to_rust(expr: &ast::Expr, meta: Option<&FieldMeta>) -> String {
    // Union variant identifiers
    if let ExprKind::Ident(name) = &expr.kind
        && let Some(meta) = meta
        && let Some((enum_name, variants)) = &meta.union_info
        && variants.contains(name)
    {
        return format!("{enum_name}::{name}");
    }

    let type_hint = meta.and_then(|m| m.simple_type.as_deref());
    match &expr.kind {
        ExprKind::Literal(lit) => literal_to_rust(lit, type_hint),
        ExprKind::Ident(name) => name.clone(),
        _ => expr_to_rust(expr),
    }
}

// ── When call ────────────────────────────────────────────────

fn generate_call(
    out: &mut String,
    when: &ast::WhenAction,
    action: Option<&ast::ActionDecl>,
    given_types: &HashMap<String, String>,
    entities: &HashMap<String, EntityInfo>,
) {
    let fn_name = to_snake_case(&when.action_name);
    let mut args = Vec::new();

    if let Some(action) = action {
        // Match when args to action params in declaration order
        for param in &action.params {
            let when_arg = when.args.iter().find(|a| a.name == param.name);
            if let Some(arg) = when_arg {
                let param_type = simple_type_name(&param.ty.ty).unwrap_or_default();
                let is_entity = entities.contains_key(&param_type);

                match &arg.value.kind {
                    ExprKind::Ident(name) if given_types.contains_key(name) => {
                        if is_entity {
                            args.push(format!("&mut {name}"));
                        } else {
                            args.push(name.clone());
                        }
                    }
                    _ => {
                        let hint = simple_type_name(&param.ty.ty);
                        let value = param_value_to_rust(&arg.value, hint.as_deref());
                        args.push(value);
                    }
                }
            }
        }
    } else {
        // No action declaration found — use positional args
        for arg in &when.args {
            args.push(expr_to_rust(&arg.value));
        }
    }

    out.push_str(&format!(
        "        let result = {}({});\n",
        fn_name,
        args.join(", ")
    ));
}

/// Convert a parameter value to Rust with a type hint.
fn param_value_to_rust(expr: &ast::Expr, type_hint: Option<&str>) -> String {
    match &expr.kind {
        ExprKind::Literal(lit) => literal_to_rust(lit, type_hint),
        _ => expr_to_rust(expr),
    }
}

// ── Then assertions ──────────────────────────────────────────

fn generate_assertion(
    out: &mut String,
    expr: &ast::Expr,
    given_types: &HashMap<String, String>,
    entities: &HashMap<String, EntityInfo>,
) {
    if let ExprKind::Compare { left, op, right } = &expr.kind {
        let lhs = expr_to_rust(left);

        // Resolve the field type for proper RHS conversion
        let type_hint = resolve_field_type(left, given_types, entities);
        let union_ctx = resolve_union_context(left, given_types, entities);

        let rhs = match &right.kind {
            ExprKind::Ident(name) if union_ctx.is_some() => {
                format!("{}::{name}", union_ctx.unwrap())
            }
            ExprKind::Literal(lit) => literal_to_rust(lit, type_hint.as_deref()),
            _ => expr_to_rust(right),
        };

        match op {
            CmpOp::Eq => out.push_str(&format!("        assert_eq!({lhs}, {rhs});\n")),
            CmpOp::Ne => out.push_str(&format!("        assert_ne!({lhs}, {rhs});\n")),
            _ => {
                let op_str = match op {
                    CmpOp::Lt => "<",
                    CmpOp::Gt => ">",
                    CmpOp::Le => "<=",
                    CmpOp::Ge => ">=",
                    _ => unreachable!(),
                };
                out.push_str(&format!("        assert!({lhs} {op_str} {rhs});\n"));
            }
        }
    } else {
        out.push_str(&format!("        assert!({});\n", expr_to_rust(expr)));
    }
}

/// Resolve the type of a field access (e.g. `from.balance` -> "Decimal").
fn resolve_field_type(
    expr: &ast::Expr,
    given_types: &HashMap<String, String>,
    entities: &HashMap<String, EntityInfo>,
) -> Option<String> {
    if let ExprKind::FieldAccess { root, fields } = &expr.kind
        && let ExprKind::Ident(var) = &root.kind
    {
        let entity_name = given_types.get(var)?;
        let entity = entities.get(entity_name)?;
        let field_name = fields.first()?;
        let meta = entity.fields.get(field_name)?;
        return meta.simple_type.clone();
    }
    None
}

/// Resolve the union enum name for a field access (e.g. `from.status` -> "AccountStatus").
fn resolve_union_context(
    expr: &ast::Expr,
    given_types: &HashMap<String, String>,
    entities: &HashMap<String, EntityInfo>,
) -> Option<String> {
    if let ExprKind::FieldAccess { root, fields } = &expr.kind
        && let ExprKind::Ident(var) = &root.kind
    {
        let entity_name = given_types.get(var)?;
        let entity = entities.get(entity_name)?;
        let field_name = fields.first()?;
        let meta = entity.fields.get(field_name)?;
        return meta.union_info.as_ref().map(|(name, _)| name.clone());
    }
    None
}

// ── Value conversion ─────────────────────────────────────────

/// Convert a literal to Rust code, using a type hint for disambiguation.
fn literal_to_rust(lit: &Literal, type_hint: Option<&str>) -> String {
    match (lit, type_hint) {
        // UUID fields — spec test values like "acc-1" aren't real UUIDs
        (Literal::String(_), Some("UUID")) => "Uuid::new_v4()".to_string(),
        // DateTime fields — exact value rarely matters for contracts
        (Literal::String(_), Some("DateTime")) => "Utc::now()".to_string(),
        // String/CurrencyCode/Email/URL
        (Literal::String(s), _) => format!("\"{s}\".to_string()"),
        // Decimal from decimal literal
        (Literal::Decimal(s), _) => format!("Decimal::from_str(\"{s}\").unwrap()"),
        // Int that should be Decimal
        (Literal::Int(n), Some("Decimal")) => format!("Decimal::from({n}_i64)"),
        (Literal::Int(n), _) => format!("{n}"),
        (Literal::Bool(b), _) => format!("{b}"),
        (Literal::Null, _) => "None".to_string(),
    }
}

/// Convert an expression to Rust without type context (best effort).
fn expr_to_rust(expr: &ast::Expr) -> String {
    match &expr.kind {
        ExprKind::Literal(lit) => literal_to_rust(lit, None),
        ExprKind::Ident(name) => name.clone(),
        ExprKind::FieldAccess { root, fields } => {
            format!("{}.{}", expr_to_rust(root), fields.join("."))
        }
        ExprKind::Compare { left, op, right } => {
            let op_str = match op {
                CmpOp::Eq => "==",
                CmpOp::Ne => "!=",
                CmpOp::Lt => "<",
                CmpOp::Gt => ">",
                CmpOp::Le => "<=",
                CmpOp::Ge => ">=",
            };
            format!("{} {op_str} {}", expr_to_rust(left), expr_to_rust(right))
        }
        ExprKind::Arithmetic { left, op, right } => {
            let op_str = match op {
                ast::ArithOp::Add => "+",
                ast::ArithOp::Sub => "-",
            };
            format!("{} {op_str} {}", expr_to_rust(left), expr_to_rust(right))
        }
        ExprKind::And(l, r) => format!("{} && {}", expr_to_rust(l), expr_to_rust(r)),
        ExprKind::Or(l, r) => format!("{} || {}", expr_to_rust(l), expr_to_rust(r)),
        ExprKind::Not(e) => format!("!{}", expr_to_rust(e)),
        ExprKind::Old(e) => format!("/* old */ {}", expr_to_rust(e)),
        ExprKind::Call { name, args } => {
            let args_str: Vec<String> = args
                .iter()
                .map(|a| match a {
                    ast::CallArg::Named { value, .. } => expr_to_rust(value),
                    ast::CallArg::Positional(e) => expr_to_rust(e),
                })
                .collect();
            format!("{name}({})", args_str.join(", "))
        }
        ExprKind::Implies(l, r) => {
            format!("!({}) || ({})", expr_to_rust(l), expr_to_rust(r))
        }
        ExprKind::Quantifier { .. } => "true /* quantifier */".to_string(),
        ExprKind::List(items) => {
            let inner: Vec<String> = items.iter().map(expr_to_rust).collect();
            format!("vec![{}]", inner.join(", "))
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────

fn safe_field(name: &str) -> String {
    let snake = to_snake_case(name);
    const KEYWORDS: &[&str] = &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "false", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "match", "mod",
        "move", "mut", "pub", "ref", "return", "self", "static", "struct", "super", "trait",
        "true", "type", "unsafe", "use", "where", "while", "yield",
    ];
    if KEYWORDS.contains(&snake.as_str()) {
        format!("r#{snake}")
    } else {
        snake
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ast::File {
        intent_parser::parse_file(src).expect("parse failed")
    }

    #[test]
    fn test_generate_empty_for_no_tests() {
        let src = "module Test\n\nentity Foo { id: UUID }\n";
        assert!(generate(&parse(src)).is_empty());
    }

    #[test]
    fn test_generate_failure_test() {
        let src = r#"module Test

entity Acc {
  id: UUID
  balance: Int
}

action Withdraw {
  account: Acc
  amount: Int

  requires {
    amount > 0
    account.balance >= amount
  }
}

test "overdraft rejected" {
  given {
    acc = Acc { id: "x", balance: 50 }
  }
  when Withdraw { account: acc, amount: 100 }
  then fails precondition
}
"#;
        let harness = generate(&parse(src));
        assert!(harness.contains("#[cfg(test)]"));
        assert!(harness.contains("mod contract_tests"));
        assert!(harness.contains("fn test_overdraft_rejected()"));
        assert!(harness.contains("assert!(result.is_err()"));
        assert!(harness.contains("&mut acc"));
    }

    #[test]
    fn test_generate_success_test_with_assertions() {
        let src = r#"module Test

entity Counter {
  id: UUID
  value: Int
}

action Increment {
  counter: Counter

  ensures {
    counter.value == old(counter.value) + 1
  }
}

test "increment works" {
  given {
    c = Counter { id: "c1", value: 5 }
  }
  when Increment { counter: c }
  then {
    c.value == 6
  }
}
"#;
        let harness = generate(&parse(src));
        assert!(harness.contains("fn test_increment_works()"));
        assert!(harness.contains("assert!(result.is_ok()"));
        assert!(harness.contains("assert_eq!(c.value, 6)"));
        assert!(harness.contains("&mut c"));
    }

    #[test]
    fn test_union_enum_in_given() {
        let src = r#"module Test

entity Acc {
  id: UUID
  status: Active | Frozen
}

action Freeze {
  account: Acc
}

test "freeze active" {
  given {
    a = Acc { id: "a1", status: Active }
  }
  when Freeze { account: a }
  then {
    a.status == Frozen
  }
}
"#;
        let harness = generate(&parse(src));
        // Given should use enum variant
        assert!(harness.contains("AccStatus::Active"));
        // Then assertion should also use enum variant
        assert!(harness.contains("AccStatus::Frozen"));
    }

    #[test]
    fn test_decimal_values() {
        let src = r#"module Test

entity Acc {
  id: UUID
  balance: Decimal(precision: 2)
}

action Deposit {
  account: Acc
  amount: Decimal(precision: 2)
}

test "deposit adds" {
  given {
    a = Acc { id: "a1", balance: 100.00 }
  }
  when Deposit { account: a, amount: 50.00 }
  then {
    a.balance == 150.00
  }
}
"#;
        let harness = generate(&parse(src));
        assert!(harness.contains("use std::str::FromStr"));
        assert!(harness.contains("Decimal::from_str(\"100.00\").unwrap()"));
        assert!(harness.contains("Decimal::from_str(\"50.00\").unwrap()"));
        assert!(harness.contains("Decimal::from_str(\"150.00\").unwrap()"));
    }

    #[test]
    fn test_non_entity_params() {
        let src = r#"module Test

entity Item { id: UUID }

action SetPrice {
  item: Item
  price: Int
}

test "set price" {
  given {
    i = Item { id: "i1" }
    p = 42
  }
  when SetPrice { item: i, price: p }
  then {
    p == 42
  }
}
"#;
        let harness = generate(&parse(src));
        // Entity param gets &mut, non-entity given var does not
        assert!(harness.contains("&mut i"));
        // price is Int, not an entity
        assert!(harness.contains(", p)"));
    }

    #[test]
    fn test_multiple_tests() {
        let src = r#"module Test

entity X { id: UUID }

action DoIt { x: X }

test "first" {
  given { x = X { id: "1" } }
  when DoIt { x: x }
  then fails
}

test "second" {
  given { x = X { id: "2" } }
  when DoIt { x: x }
  then fails
}
"#;
        let harness = generate(&parse(src));
        assert!(harness.contains("fn test_first()"));
        assert!(harness.contains("fn test_second()"));
    }

    #[test]
    fn test_transfer_example() {
        let src = r#"module TransferFunds

entity Account {
  id: UUID
  owner: String
  balance: Decimal(precision: 2)
  currency: CurrencyCode
  status: Active | Frozen | Closed
  created_at: DateTime
}

action Transfer {
  from: Account
  to: Account
  amount: Decimal(precision: 2)
  request_id: UUID

  requires {
    from.status == Active
    to.status == Active
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
    from = Account { id: "acc-1", owner: "Alice", balance: 1000.00, currency: "USD", status: Active, created_at: "2024-01-01" }
    to = Account { id: "acc-2", owner: "Bob", balance: 500.00, currency: "USD", status: Active, created_at: "2024-01-01" }
  }
  when Transfer {
    from: from,
    to: to,
    amount: 200.00,
    request_id: "req-1"
  }
  then {
    from.balance == 800.00
    to.balance == 700.00
  }
}

test "insufficient funds" {
  given {
    from = Account { id: "acc-1", owner: "Alice", balance: 50.00, currency: "USD", status: Active, created_at: "2024-01-01" }
    to = Account { id: "acc-2", owner: "Bob", balance: 500.00, currency: "USD", status: Active, created_at: "2024-01-01" }
  }
  when Transfer {
    from: from,
    to: to,
    amount: 200.00,
    request_id: "req-2"
  }
  then fails precondition
}

test "frozen account rejected" {
  given {
    from = Account { id: "acc-1", owner: "Alice", balance: 1000.00, currency: "USD", status: Frozen, created_at: "2024-01-01" }
    to = Account { id: "acc-2", owner: "Bob", balance: 500.00, currency: "USD", status: Active, created_at: "2024-01-01" }
  }
  when Transfer {
    from: from,
    to: to,
    amount: 100.00,
    request_id: "req-3"
  }
  then fails precondition
}
"#;
        let harness = generate(&parse(src));

        // All three tests present
        assert!(harness.contains("fn test_successful_transfer()"));
        assert!(harness.contains("fn test_insufficient_funds()"));
        assert!(harness.contains("fn test_frozen_account_rejected()"));

        // Entity params as &mut
        assert!(harness.contains("&mut from"));
        assert!(harness.contains("&mut to"));

        // Decimal values
        assert!(harness.contains("Decimal::from_str(\"1000.00\").unwrap()"));
        assert!(harness.contains("Decimal::from_str(\"200.00\").unwrap()"));

        // UUID fields use Uuid::new_v4()
        assert!(harness.contains("Uuid::new_v4()"));

        // DateTime fields use Utc::now()
        assert!(harness.contains("Utc::now()"));

        // Union enums
        assert!(harness.contains("AccountStatus::Active"));
        assert!(harness.contains("AccountStatus::Frozen"));

        // String fields
        assert!(harness.contains("\"Alice\".to_string()"));
        assert!(harness.contains("\"USD\".to_string()"));

        // Success test has postcondition assertions
        assert!(
            harness.contains("assert_eq!(from.balance, Decimal::from_str(\"800.00\").unwrap())")
        );
        assert!(harness.contains("assert_eq!(to.balance, Decimal::from_str(\"700.00\").unwrap())"));

        // Failure tests assert error
        assert!(harness.contains("assert!(result.is_err()"));
    }
}
