use crate::lower::lower_file;
use crate::verify::{verify_module, VerifyErrorKind};

fn parse_and_lower(src: &str) -> crate::types::Module {
    let ast = intent_parser::parse_file(src).unwrap();
    lower_file(&ast)
}

fn verify(src: &str) -> Vec<VerifyErrorKind> {
    let ir = parse_and_lower(src);
    verify_module(&ir).into_iter().map(|e| e.kind).collect()
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
