//! Tests for the semantic checker.

use intent_parser::parse_file;

use crate::check_file;
use crate::errors::CheckError;

fn check(src: &str) -> Vec<CheckError> {
    let file = parse_file(src).expect("parse should succeed");
    check_file(&file)
}

// ── Valid files produce no errors ────────────────────────────

#[test]
fn valid_minimal() {
    let src = include_str!("../../../tests/valid/minimal.intent");
    assert!(check(src).is_empty());
}

#[test]
fn valid_entity_only() {
    let src = include_str!("../../../tests/valid/entity_only.intent");
    assert!(check(src).is_empty());
}

#[test]
fn valid_all_types() {
    let src = include_str!("../../../tests/valid/all_types.intent");
    assert!(check(src).is_empty());
}

#[test]
fn valid_full_action() {
    let src = include_str!("../../../tests/valid/full_action.intent");
    assert!(check(src).is_empty());
}

#[test]
fn valid_transfer_example() {
    let src = include_str!("../../../examples/transfer.intent");
    assert!(check(src).is_empty());
}

#[test]
fn valid_auth_example() {
    let src = include_str!("../../../examples/auth.intent");
    assert!(check(src).is_empty());
}

#[test]
fn valid_shopping_cart_example() {
    let src = include_str!("../../../examples/shopping_cart.intent");
    assert!(check(src).is_empty());
}

// ── Duplicate detection ──────────────────────────────────────

#[test]
fn duplicate_entity() {
    let src = include_str!("../../../tests/invalid/duplicate_entity.intent");
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    assert!(matches!(&errs[0], CheckError::DuplicateEntity { name, .. } if name == "User"));
}

#[test]
fn duplicate_field() {
    let src = include_str!("../../../tests/invalid/duplicate_field.intent");
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    assert!(
        matches!(&errs[0], CheckError::DuplicateField { field, parent, .. } if field == "name" && parent == "Account")
    );
}

#[test]
fn duplicate_action() {
    let src = include_str!("../../../tests/invalid/duplicate_action.intent");
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    assert!(matches!(&errs[0], CheckError::DuplicateAction { name, .. } if name == "Transfer"));
}

// ── Type resolution ──────────────────────────────────────────

#[test]
fn undefined_type() {
    let src = include_str!("../../../tests/invalid/undefined_type.intent");
    let errs = check(src);
    // Customer and LineItem are both undefined
    assert_eq!(errs.len(), 2);
    let names: Vec<&str> = errs
        .iter()
        .filter_map(|e| match e {
            CheckError::UndefinedType { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert!(names.contains(&"Customer"));
    assert!(names.contains(&"LineItem"));
}

#[test]
fn builtin_types_are_valid() {
    // All built-in types should be accepted without entity definitions.
    let src = r#"module BuiltinTest

entity Everything {
  id: UUID
  name: String
  count: Int
  amount: Decimal(precision: 2)
  flag: Bool
  when: DateTime
  currency: CurrencyCode
  mail: Email
  link: URL
}
"#;
    assert!(check(src).is_empty());
}

#[test]
fn entity_as_field_type() {
    // An entity name used as a field type should be valid.
    let src = r#"module EntityRefTest

entity User {
  id: UUID
}

entity Order {
  id: UUID
  buyer: User
}
"#;
    assert!(check(src).is_empty());
}

#[test]
fn union_variants_are_enum_labels() {
    // Union variants like Active | Frozen are enum-like labels, not type references.
    let src = r#"module UnionTest

entity Thing {
  status: Active | Broken | Missing
}
"#;
    assert!(check(src).is_empty());
}

// ── Quantifier type checking ─────────────────────────────────

#[test]
fn bad_quantifier_type() {
    let src = include_str!("../../../tests/invalid/bad_quantifier_type.intent");
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    assert!(
        matches!(&errs[0], CheckError::UndefinedQuantifierType { name, .. } if name == "Nonexistent")
    );
}

#[test]
fn quantifier_with_entity_type() {
    let src = r#"module QuantOk

entity Account {
  id: UUID
  balance: Int
}

invariant Positive {
  forall a: Account => a.balance >= 0
}
"#;
    assert!(check(src).is_empty());
}

#[test]
fn quantifier_with_action_type() {
    // Actions should also be valid quantifier binding types (see transfer.intent).
    let src = r#"module QuantAction

entity Account {
  id: UUID
}

action Transfer {
  from: Account

  requires {
    from.id != from.id
  }
}

invariant Conservation {
  forall t: Transfer => t.from.id != t.from.id
}
"#;
    assert!(check(src).is_empty());
}

// ── Edge case action references ─────────────────────────────

#[test]
fn undefined_edge_action() {
    let src = include_str!("../../../tests/invalid/undefined_edge_action.intent");
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    assert!(
        matches!(&errs[0], CheckError::UndefinedEdgeAction { name, .. } if name == "RequireApproval")
    );
}

#[test]
fn valid_edge_action() {
    let src = r#"module EdgeOk

entity Account {
  id: UUID
  balance: Int
}

action Transfer {
  from: Account
  amount: Int

  requires {
    amount > 0
  }
}

action Reject {
  reason: String
}

edge_cases {
  when amount > 10000 => Transfer(from: from, amount: amount)
  when amount == 0 => Reject(reason: "zero amount")
}
"#;
    assert!(check(src).is_empty());
}

// ── Field access validation ─────────────────────────────────

#[test]
fn unknown_field_access() {
    let src = include_str!("../../../tests/invalid/unknown_field.intent");
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    assert!(
        matches!(&errs[0], CheckError::UnknownField { field, entity, .. }
            if field == "credit_limit" && entity == "Account")
    );
}

#[test]
fn valid_field_access() {
    let src = r#"module FieldOk

entity Account {
  id: UUID
  balance: Int
  status: Active | Frozen
}

action Transfer {
  from: Account
  amount: Int

  requires {
    from.balance >= amount
    from.status == Active
  }

  ensures {
    from.balance == old(from.balance) - amount
  }
}
"#;
    assert!(check(src).is_empty());
}

#[test]
fn unknown_field_in_ensures() {
    let src = r#"module FieldEns

entity Account {
  id: UUID
  balance: Int
}

action Withdraw {
  account: Account
  amount: Int

  ensures {
    account.remaining == old(account.balance) - amount
  }
}
"#;
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    assert!(
        matches!(&errs[0], CheckError::UnknownField { field, entity, .. }
            if field == "remaining" && entity == "Account")
    );
}
