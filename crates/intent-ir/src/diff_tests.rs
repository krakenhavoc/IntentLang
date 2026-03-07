use crate::audit::{SpecItemKind, generate_audit};
use crate::diff::{ChangeKind, diff_reports};
use crate::lower::lower_file;
use crate::verify::{analyze_obligations, verify_module};

fn audit(src: &str) -> crate::audit::AuditReport {
    let ast = intent_parser::parse_file(src).unwrap();
    let ir = lower_file(&ast);
    let errors = verify_module(&ir);
    let obligations = analyze_obligations(&ir);
    generate_audit(src, &ir, &errors, &obligations)
}

#[test]
fn identical_specs_no_changes() {
    let src = "module M entity X { v: Int }";
    let old = audit(src);
    let new = audit(src);
    let diff = diff_reports(&old, &new);
    assert!(diff.changes.is_empty());
    assert_eq!(diff.summary.added, 0);
    assert_eq!(diff.summary.removed, 0);
    assert_eq!(diff.summary.modified, 0);
}

#[test]
fn added_entity() {
    let old = audit("module M entity X { v: Int }");
    let new = audit("module M entity X { v: Int } entity Y { w: Bool }");
    let diff = diff_reports(&old, &new);
    assert_eq!(diff.summary.added, 1);
    let added = diff
        .changes
        .iter()
        .find(|c| c.change == ChangeKind::Added)
        .unwrap();
    assert_eq!(added.kind, SpecItemKind::Entity);
    assert_eq!(added.name, "Y");
}

#[test]
fn removed_entity() {
    let old = audit("module M entity X { v: Int } entity Y { w: Bool }");
    let new = audit("module M entity X { v: Int }");
    let diff = diff_reports(&old, &new);
    assert_eq!(diff.summary.removed, 1);
    let removed = diff
        .changes
        .iter()
        .find(|c| c.change == ChangeKind::Removed)
        .unwrap();
    assert_eq!(removed.kind, SpecItemKind::Entity);
    assert_eq!(removed.name, "Y");
}

#[test]
fn modified_entity_field_added() {
    let old = audit("module M entity X { v: Int }");
    let new = audit("module M entity X { v: Int w: Bool }");
    let diff = diff_reports(&old, &new);
    assert_eq!(diff.summary.modified, 1);
    let modified = &diff.changes[0];
    assert_eq!(modified.name, "X");
    assert_eq!(modified.change, ChangeKind::Modified);
    assert_eq!(modified.details.len(), 1);
    assert_eq!(modified.details[0].label, "field:w");
    assert_eq!(modified.details[0].change, ChangeKind::Added);
}

#[test]
fn modified_entity_field_removed() {
    let old = audit("module M entity X { v: Int w: Bool }");
    let new = audit("module M entity X { v: Int }");
    let diff = diff_reports(&old, &new);
    assert_eq!(diff.summary.modified, 1);
    let modified = &diff.changes[0];
    assert_eq!(modified.details.len(), 1);
    assert_eq!(modified.details[0].label, "field:w");
    assert_eq!(modified.details[0].change, ChangeKind::Removed);
}

#[test]
fn modified_entity_field_type_changed() {
    let old = audit("module M entity X { v: Int }");
    let new = audit("module M entity X { v: Bool }");
    let diff = diff_reports(&old, &new);
    assert_eq!(diff.summary.modified, 1);
    let detail = &diff.changes[0].details[0];
    assert_eq!(detail.label, "field:v");
    assert_eq!(detail.change, ChangeKind::Modified);
    assert_eq!(detail.old_desc.as_deref(), Some("v: Int"));
    assert_eq!(detail.new_desc.as_deref(), Some("v: Bool"));
}

#[test]
fn added_action() {
    let old = audit("module M entity X { v: Int }");
    let new = audit("module M entity X { v: Int } action A { x: X requires { x.v > 0 } }");
    let diff = diff_reports(&old, &new);
    assert_eq!(diff.summary.added, 1);
    let added = diff
        .changes
        .iter()
        .find(|c| c.change == ChangeKind::Added)
        .unwrap();
    assert_eq!(added.kind, SpecItemKind::Action);
    assert_eq!(added.name, "A");
}

#[test]
fn added_invariant() {
    let old = audit("module M entity X { v: Int }");
    let new = audit("module M entity X { v: Int } invariant Pos { forall x: X => x.v >= 0 }");
    let diff = diff_reports(&old, &new);
    assert_eq!(diff.summary.added, 1);
    let added = diff
        .changes
        .iter()
        .find(|c| c.change == ChangeKind::Added)
        .unwrap();
    assert_eq!(added.kind, SpecItemKind::Invariant);
    assert_eq!(added.name, "Pos");
}

#[test]
fn action_precondition_added() {
    let old = audit("module M entity X { v: Int } action A { x: X }");
    let new = audit("module M entity X { v: Int } action A { x: X requires { x.v > 0 } }");
    let diff = diff_reports(&old, &new);
    assert_eq!(diff.summary.modified, 1);
    let modified = &diff.changes[0];
    assert_eq!(modified.name, "A");
    let added_detail = modified
        .details
        .iter()
        .find(|d| d.label == "requires[0]")
        .unwrap();
    assert_eq!(added_detail.change, ChangeKind::Added);
}

#[test]
fn coverage_delta_in_format() {
    let old = audit("module M entity X { v: Int }");
    let new = audit("module M entity X { v: Int w: Bool }");
    let diff = diff_reports(&old, &new);
    let formatted = diff.format();
    assert!(formatted.contains("Spec Diff Report"));
    assert!(formatted.contains("[~] Entity X"));
    assert!(formatted.contains("[+] field:w"));
    assert!(formatted.contains("Coverage:"));
    assert!(formatted.contains("+1"));
}

#[test]
fn no_changes_format() {
    let src = "module M entity X { v: Int }";
    let old = audit(src);
    let new = audit(src);
    let diff = diff_reports(&old, &new);
    let formatted = diff.format();
    assert!(formatted.contains("No spec-level changes detected"));
}

#[test]
fn verification_status_in_format() {
    let old = audit("module M entity X { v: Int }");
    let new = audit("module M entity X { v: Int w: Bool }");
    let diff = diff_reports(&old, &new);
    let formatted = diff.format();
    assert!(formatted.contains("clean -> clean"));
}

#[test]
fn obligations_delta() {
    let old =
        audit("module M entity X { v: Int } action A { x: X ensures { x.v == old(x.v) + 1 } }");
    let new = audit(
        "module M entity X { v: Int } action A { x: X ensures { x.v == old(x.v) + 1 } } invariant Pos { forall x: X => x.v >= 0 }",
    );
    let diff = diff_reports(&old, &new);
    assert_eq!(diff.summary.old_obligations, 0);
    assert_eq!(diff.summary.new_obligations, 1);
    let formatted = diff.format();
    assert!(formatted.contains("Obligations: 0 -> 1"));
}
