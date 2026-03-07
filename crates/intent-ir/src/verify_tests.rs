use crate::lower::lower_file;
use crate::verify::{analyze_obligations, verify_module, ObligationKind, VerifyErrorKind};

fn parse_and_lower(src: &str) -> crate::types::Module {
    let ast = intent_parser::parse_file(src).unwrap();
    lower_file(&ast)
}

fn verify(src: &str) -> Vec<VerifyErrorKind> {
    let ir = parse_and_lower(src);
    verify_module(&ir).into_iter().map(|e| e.kind).collect()
}

fn obligations(src: &str) -> Vec<crate::verify::Obligation> {
    let ir = parse_and_lower(src);
    analyze_obligations(&ir)
}

#[test]
fn valid_action_passes() {
    let errors = verify(
        "module M entity X { v: Int } action A { x: X requires { x.v > 0 } ensures { x.v == old(x.v) + 1 } }",
    );
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

#[test]
fn old_in_precondition_rejected() {
    // Note: old() in requires is already caught by intent-check at the AST level,
    // but the IR verifier should also catch it.
    let errors = verify(
        "module M entity X { v: Int } action A { x: X requires { x.v > old(x.v) } }",
    );
    assert!(errors.contains(&VerifyErrorKind::OldOutsidePoscondition));
}

#[test]
fn postcondition_without_params() {
    let errors = verify(
        "module M entity X { v: Int } action A { ensures { forall x: X => x.v > 0 } }",
    );
    assert!(errors.contains(&VerifyErrorKind::PostconditionWithoutParams {
        function: "A".into(),
    }));
}

#[test]
fn unknown_quantifier_type() {
    let errors = verify(
        "module M action A { x: Int requires { forall y: Unknown => y > 0 } }",
    );
    assert!(errors.contains(&VerifyErrorKind::UnknownQuantifierType {
        ty: "Unknown".into(),
    }));
}

#[test]
fn quantifier_over_action_accepted() {
    let errors = verify(
        "module M entity X { v: Int } action Transfer { x: X } invariant Cons { forall t: Transfer => t.x.v >= 0 }",
    );
    // Transfer is a known function name, should be accepted as quantifier type
    let has_unknown = errors.iter().any(|e| matches!(e, VerifyErrorKind::UnknownQuantifierType { .. }));
    assert!(!has_unknown, "Transfer should be accepted as quantifier type, got: {:?}", errors);
}

#[test]
fn union_variant_not_flagged_as_unbound() {
    let errors = verify(
        "module M entity X { status: Active | Frozen } action A { x: X requires { x.status == Active } ensures { x.status == Frozen } }",
    );
    let has_unbound = errors.iter().any(|e| matches!(e, VerifyErrorKind::UnboundVariable { .. }));
    assert!(!has_unbound, "union variants should not be flagged, got: {:?}", errors);
}

#[test]
fn old_in_temporal_invariant_accepted() {
    let errors = verify(
        "module M entity X { v: Int } action A { x: X } invariant Cons { forall t: A => old(t.x.v) == t.x.v }",
    );
    let has_old_err = errors.iter().any(|e| matches!(e, VerifyErrorKind::OldOutsidePoscondition));
    assert!(!has_old_err, "old() should be valid in temporal invariants, got: {:?}", errors);
}

#[test]
fn zero_arg_call_not_flagged() {
    let errors = verify(
        "module M action A { x: Int requires { x > now() } ensures { x == now() } }",
    );
    let has_unbound = errors.iter().any(|e| matches!(e, VerifyErrorKind::UnboundVariable { name } if name == "now"));
    assert!(!has_unbound, "now() should not be flagged as unbound, got: {:?}", errors);
}

#[test]
fn transfer_example_verifies() {
    let src = std::fs::read_to_string("../../examples/transfer.intent").unwrap();
    let ast = intent_parser::parse_file(&src).unwrap();
    let ir = lower_file(&ast);
    let errors = verify_module(&ir);
    assert!(errors.is_empty(), "transfer.intent should verify cleanly, got: {:?}", errors);
}

#[test]
fn auth_example_verifies() {
    let src = std::fs::read_to_string("../../examples/auth.intent").unwrap();
    let ast = intent_parser::parse_file(&src).unwrap();
    let ir = lower_file(&ast);
    let errors = verify_module(&ir);
    assert!(errors.is_empty(), "auth.intent should verify cleanly, got: {:?}", errors);
}

#[test]
fn shopping_cart_example_verifies() {
    let src = std::fs::read_to_string("../../examples/shopping_cart.intent").unwrap();
    let ast = intent_parser::parse_file(&src).unwrap();
    let ir = lower_file(&ast);
    let errors = verify_module(&ir);
    assert!(errors.is_empty(), "shopping_cart.intent should verify cleanly, got: {:?}", errors);
}

// ── Coherence analysis tests ───────────────────────────────

#[test]
fn obligation_invariant_preservation() {
    let obs = obligations(
        "module M entity X { v: Int } action A { x: X requires { x.v > 0 } ensures { x.v == old(x.v) + 1 } } invariant Pos { forall x: X => x.v >= 0 }",
    );
    assert_eq!(obs.len(), 1);
    assert_eq!(obs[0].action, "A");
    assert_eq!(obs[0].invariant, "Pos");
    assert_eq!(obs[0].entity, "X");
    assert_eq!(obs[0].fields, vec!["v"]);
    assert_eq!(obs[0].kind, ObligationKind::InvariantPreservation);
}

#[test]
fn obligation_temporal_property() {
    let obs = obligations(
        "module M entity X { v: Int } action A { x: X } invariant Cons { forall t: A => old(t.x.v) == t.x.v }",
    );
    assert_eq!(obs.len(), 1);
    assert_eq!(obs[0].action, "A");
    assert_eq!(obs[0].invariant, "Cons");
    assert_eq!(obs[0].kind, ObligationKind::TemporalProperty);
}

#[test]
fn no_obligations_when_no_invariants() {
    let obs = obligations(
        "module M entity X { v: Int } action A { x: X ensures { x.v == old(x.v) + 1 } }",
    );
    assert!(obs.is_empty());
}

#[test]
fn no_obligations_when_no_old() {
    // Action doesn't use old() → doesn't modify entity fields → no obligation
    let obs = obligations(
        "module M entity X { v: Int } action A { x: X ensures { x.v == 0 } } invariant Pos { forall x: X => x.v >= 0 }",
    );
    assert!(obs.is_empty());
}

#[test]
fn obligation_only_for_constrained_fields() {
    // Invariant constrains `v`, but action modifies `w` → no obligation
    let obs = obligations(
        "module M entity X { v: Int w: Int } action A { x: X ensures { x.w == old(x.w) + 1 } } invariant Pos { forall x: X => x.v >= 0 }",
    );
    assert!(obs.is_empty());
}

#[test]
fn transfer_example_obligations() {
    let src = std::fs::read_to_string("../../examples/transfer.intent").unwrap();
    let ir = parse_and_lower(&src);
    let obs = analyze_obligations(&ir);
    // Transfer modifies Account.balance (constrained by NoNegativeBalances)
    // TransferConservation is a temporal invariant on Transfer
    assert_eq!(obs.len(), 2);
    let preservation = obs.iter().find(|o| o.kind == ObligationKind::InvariantPreservation);
    let temporal = obs.iter().find(|o| o.kind == ObligationKind::TemporalProperty);
    assert!(preservation.is_some(), "expected InvariantPreservation obligation");
    assert!(temporal.is_some(), "expected TemporalProperty obligation");
    let p = preservation.unwrap();
    assert_eq!(p.action, "Transfer");
    assert_eq!(p.invariant, "NoNegativeBalances");
    assert_eq!(p.fields, vec!["balance"]);
}
