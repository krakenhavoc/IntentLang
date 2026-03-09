//! Tests for the semantic checker.

use intent_parser::parse_file;

use crate::errors::CheckError;
use crate::{check_file, check_file_with_imports};

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
    from.id != null
  }
}

invariant Conservation {
  forall t: Transfer => t.from.id != null
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

// ── Constraint validation ────────────────────────────────────

#[test]
fn old_in_requires() {
    let src = include_str!("../../../tests/invalid/old_in_requires.intent");
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    assert!(matches!(&errs[0], CheckError::OldInRequires { .. }));
}

#[test]
fn tautological_comparison() {
    let src = include_str!("../../../tests/invalid/tautological_comparison.intent");
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    assert!(
        matches!(&errs[0], CheckError::TautologicalComparison { expr, result, .. }
            if expr == "from.balance" && result == "true")
    );
}

#[test]
fn tautological_not_equal() {
    let src = r#"module TautNe

entity Account {
  id: UUID
  balance: Int
}

action Check {
  a: Account

  requires {
    a.balance != a.balance
  }
}
"#;
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    assert!(
        matches!(&errs[0], CheckError::TautologicalComparison { result, .. }
            if result == "false")
    );
}

#[test]
fn old_in_ensures_is_valid() {
    // old() in ensures is correct usage — should produce no errors.
    let src = r#"module OldEnsOk

entity Account {
  id: UUID
  balance: Int
}

action Withdraw {
  account: Account
  amount: Int

  ensures {
    account.balance == old(account.balance) - amount
  }
}
"#;
    assert!(check(src).is_empty());
}

// ── State machine validation ────────────────────────────────

#[test]
fn state_machine_registers_as_type() {
    let src = r#"module SM

state TaskStatus {
  Open -> InProgress -> Done
}

entity Task {
  id: UUID
  status: TaskStatus
}
"#;
    assert!(check(src).is_empty());
}

#[test]
fn state_machine_duplicate_name() {
    let src = r#"module SM

entity TaskStatus {
  id: UUID
}

state TaskStatus {
  Open -> Done
}
"#;
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    assert!(matches!(&errs[0], CheckError::DuplicateEntity { name, .. } if name == "TaskStatus"));
}

#[test]
fn state_machine_cross_module_import() {
    let types_src = r#"module Types

state OrderStatus {
  Pending -> Confirmed -> Shipped
}
"#;
    let main_src = r#"module Main

use Types

entity Order {
  id: UUID
  status: OrderStatus
}
"#;
    let types_file = parse_file(types_src).unwrap();
    let main_file = parse_file(main_src).unwrap();
    let errors = check_file_with_imports(&main_file, &[&types_file]);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

#[test]
fn valid_task_states_example() {
    let src = include_str!("../../../examples/task_states.intent");
    assert!(check(src).is_empty());
}

// ── Cross-module imports ────────────────────────────────────

#[test]
fn cross_module_entity_resolves() {
    let types_src = r#"module Types

entity Account {
  id: UUID
  balance: Int
}
"#;
    let main_src = r#"module Main

use Types

action Transfer {
  from: Account
  amount: Int

  requires {
    from.balance >= amount
  }
}
"#;
    let types_file = parse_file(types_src).unwrap();
    let main_file = parse_file(main_src).unwrap();
    let errors = check_file_with_imports(&main_file, &[&types_file]);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

#[test]
fn cross_module_selective_import() {
    let types_src = r#"module Types

entity Account {
  id: UUID
  balance: Int
}

entity User {
  name: String
}
"#;
    let main_src = r#"module Main

use Types.Account

action Transfer {
  from: Account
  amount: Int
}
"#;
    let types_file = parse_file(types_src).unwrap();
    let main_file = parse_file(main_src).unwrap();
    let errors = check_file_with_imports(&main_file, &[&types_file]);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

#[test]
fn cross_module_selective_import_wrong_name() {
    let types_src = r#"module Types

entity Account {
  id: UUID
}
"#;
    let main_src = r#"module Main

use Types.NonExistent
"#;
    let types_file = parse_file(types_src).unwrap();
    let main_file = parse_file(main_src).unwrap();
    let errors = check_file_with_imports(&main_file, &[&types_file]);
    assert_eq!(errors.len(), 1);
    assert!(matches!(
        &errors[0],
        CheckError::UnresolvedImport { name, module, .. }
            if name == "NonExistent" && module == "Types"
    ));
}

#[test]
fn cross_module_field_access_works() {
    let types_src = r#"module Types

entity Account {
  id: UUID
  balance: Int
  status: Active | Frozen
}
"#;
    let main_src = r#"module Main

use Types

action Withdraw {
  account: Account
  amount: Int

  requires {
    account.balance >= amount
    account.status == Active
  }

  ensures {
    account.balance == old(account.balance) - amount
  }
}
"#;
    let types_file = parse_file(types_src).unwrap();
    let main_file = parse_file(main_src).unwrap();
    let errors = check_file_with_imports(&main_file, &[&types_file]);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

#[test]
fn cross_module_unknown_field_detected() {
    let types_src = r#"module Types

entity Account {
  id: UUID
  balance: Int
}
"#;
    let main_src = r#"module Main

use Types

action Withdraw {
  account: Account
  amount: Int

  requires {
    account.credit_limit >= amount
  }
}
"#;
    let types_file = parse_file(types_src).unwrap();
    let main_file = parse_file(main_src).unwrap();
    let errors = check_file_with_imports(&main_file, &[&types_file]);
    assert_eq!(errors.len(), 1);
    assert!(matches!(
        &errors[0],
        CheckError::UnknownField { field, entity, .. }
            if field == "credit_limit" && entity == "Account"
    ));
}

// ── Enhanced diagnostics: fuzzy matching & suggestions ──────

#[test]
fn undefined_type_suggests_similar_name() {
    // "Cusotmer" is edit distance 2 from "Customer" — should suggest.
    let src = r#"module SuggestTest

entity Customer {
  id: UUID
  name: String
}

entity Order {
  id: UUID
  buyer: Cusotmer
}
"#;
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    match &errs[0] {
        CheckError::UndefinedType {
            name, help_text, ..
        } => {
            assert_eq!(name, "Cusotmer");
            assert!(
                help_text.contains("did you mean"),
                "expected 'did you mean' suggestion, got: {help_text}"
            );
            assert!(
                help_text.contains("Customer"),
                "expected 'Customer' in suggestion, got: {help_text}"
            );
        }
        other => panic!("expected UndefinedType, got: {:?}", other),
    }
}

#[test]
fn undefined_type_no_suggestion_when_distant() {
    // "Xyzabc" is far from any known type — should fall back to default help.
    let src = r#"module NoSuggestTest

entity Order {
  id: UUID
  buyer: Xyzabc
}
"#;
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    match &errs[0] {
        CheckError::UndefinedType {
            name, help_text, ..
        } => {
            assert_eq!(name, "Xyzabc");
            assert!(
                help_text.contains("define an entity"),
                "expected default help text, got: {help_text}"
            );
        }
        other => panic!("expected UndefinedType, got: {:?}", other),
    }
}

#[test]
fn unknown_field_suggests_similar_name() {
    // "balence" is edit distance 1 from "balance" — should suggest.
    let src = r#"module FieldSuggestTest

entity Account {
  id: UUID
  balance: Int
  status: Active | Frozen
}

action Transfer {
  from: Account
  amount: Int

  requires {
    from.balence >= amount
  }
}
"#;
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    match &errs[0] {
        CheckError::UnknownField {
            field,
            entity,
            help_text,
            ..
        } => {
            assert_eq!(field, "balence");
            assert_eq!(entity, "Account");
            assert!(
                help_text.contains("did you mean"),
                "expected 'did you mean' suggestion, got: {help_text}"
            );
            assert!(
                help_text.contains("balance"),
                "expected 'balance' in suggestion, got: {help_text}"
            );
        }
        other => panic!("expected UnknownField, got: {:?}", other),
    }
}

#[test]
fn unknown_field_no_suggestion_when_distant() {
    // "xyzabc" is far from any field — should fall back to default help.
    let src = r#"module FieldNoSuggestTest

entity Account {
  id: UUID
  balance: Int
}

action Transfer {
  from: Account
  amount: Int

  requires {
    from.xyzabc >= amount
  }
}
"#;
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    match &errs[0] {
        CheckError::UnknownField {
            field, help_text, ..
        } => {
            assert_eq!(field, "xyzabc");
            assert!(
                help_text.contains("has no field named"),
                "expected default help text, got: {help_text}"
            );
        }
        other => panic!("expected UnknownField, got: {:?}", other),
    }
}

#[test]
fn tautological_comparison_suggests_old() {
    // Tautological comparison on field access should suggest old() variant.
    let src = r#"module TautSuggestTest

entity Account {
  id: UUID
  balance: Int
}

action Transfer {
  from: Account

  requires {
    from.balance == from.balance
  }
}
"#;
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    match &errs[0] {
        CheckError::TautologicalComparison {
            expr, help_text, ..
        } => {
            assert_eq!(expr, "from.balance");
            assert!(
                help_text.contains("old(from.balance)"),
                "expected old() suggestion, got: {help_text}"
            );
        }
        other => panic!("expected TautologicalComparison, got: {:?}", other),
    }
}

#[test]
fn old_in_requires_has_enhanced_help() {
    // OldInRequires should have enhanced help explaining pre-state semantics.
    let src = r#"module OldHelpTest

entity Account {
  id: UUID
  balance: Int
}

action Transfer {
  from: Account

  requires {
    from.balance == old(from.balance)
  }
}
"#;
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    assert!(matches!(&errs[0], CheckError::OldInRequires { .. }));
    // Verify the error uses miette Diagnostic — the help text is part of the
    // Diagnostic derive, so we verify it through the Display trait.
    let msg = format!("{}", errs[0]);
    assert!(msg.contains("old()"));
}

#[test]
fn undefined_type_suggests_builtin() {
    // "Sting" is edit distance 1 from "String" — should suggest the built-in type.
    let src = r#"module BuiltinSuggestTest

entity User {
  name: Sting
}
"#;
    let errs = check(src);
    assert_eq!(errs.len(), 1);
    match &errs[0] {
        CheckError::UndefinedType {
            name, help_text, ..
        } => {
            assert_eq!(name, "Sting");
            assert!(
                help_text.contains("did you mean"),
                "expected 'did you mean' suggestion, got: {help_text}"
            );
            assert!(
                help_text.contains("String"),
                "expected 'String' in suggestion, got: {help_text}"
            );
        }
        other => panic!("expected UndefinedType, got: {:?}", other),
    }
}

#[test]
fn correct_code_produces_no_suggestions() {
    // Valid code should produce no errors at all.
    let src = r#"module ValidTest

entity Account {
  id: UUID
  balance: Int
}

action Deposit {
  account: Account
  amount: Int

  requires {
    amount > 0
  }

  ensures {
    account.balance == old(account.balance) + amount
  }
}
"#;
    let errs = check(src);
    assert!(
        errs.is_empty(),
        "expected no errors for valid code, got: {:?}",
        errs
    );
}
