use crate::audit::{SpecItemKind, generate_audit};
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
fn basic_entity_trace() {
    let report = audit("module M entity X { v: Int w: Bool }");
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].kind, SpecItemKind::Entity);
    assert_eq!(report.entries[0].name, "X");
    assert_eq!(report.entries[0].parts.len(), 2);
    assert_eq!(report.entries[0].parts[0].label, "field:v");
    assert_eq!(report.entries[0].parts[1].label, "field:w");
}

#[test]
fn basic_action_trace() {
    let report = audit(
        "module M entity X { v: Int } action A { x: X requires { x.v > 0 } ensures { x.v == old(x.v) + 1 } }",
    );
    let action = report
        .entries
        .iter()
        .find(|e| e.kind == SpecItemKind::Action)
        .unwrap();
    assert_eq!(action.name, "A");
    assert_eq!(action.parts.len(), 3); // param + requires + ensures
    assert_eq!(action.parts[0].label, "param:x");
    assert_eq!(action.parts[1].label, "requires[0]");
    assert_eq!(action.parts[2].label, "ensures[0]");
}

#[test]
fn invariant_trace() {
    let report = audit("module M entity X { v: Int } invariant Pos { forall x: X => x.v >= 0 }");
    let inv = report
        .entries
        .iter()
        .find(|e| e.kind == SpecItemKind::Invariant)
        .unwrap();
    assert_eq!(inv.name, "Pos");
}

#[test]
fn edge_cases_trace() {
    let report = audit("module M edge_cases { when x == y => reject(\"no\") }");
    let ec = report
        .entries
        .iter()
        .find(|e| e.kind == SpecItemKind::EdgeCases)
        .unwrap();
    assert_eq!(ec.parts.len(), 1);
    assert_eq!(ec.parts[0].label, "guard[0]");
    assert!(ec.parts[0].ir_desc.contains("reject"));
}

#[test]
fn coverage_summary_counts() {
    let report = audit(
        "module M entity X { v: Int w: Bool } action A { x: X requires { x.v > 0 } ensures { x.v == old(x.v) + 1 } } invariant Pos { forall x: X => x.v >= 0 }",
    );
    assert_eq!(report.summary.entities, 1);
    assert_eq!(report.summary.fields, 2);
    assert_eq!(report.summary.actions, 1);
    assert_eq!(report.summary.params, 1);
    assert_eq!(report.summary.preconditions, 1);
    assert_eq!(report.summary.postconditions, 1);
    assert_eq!(report.summary.invariants, 1);
    assert_eq!(report.summary.total(), 8);
}

#[test]
fn obligations_appear_in_report() {
    let report = audit(
        "module M entity X { v: Int } action A { x: X ensures { x.v == old(x.v) + 1 } } invariant Pos { forall x: X => x.v >= 0 }",
    );
    assert_eq!(report.obligations.len(), 1);
    let action = report
        .entries
        .iter()
        .find(|e| e.kind == SpecItemKind::Action)
        .unwrap();
    assert!(!action.related_obligations.is_empty());
}

#[test]
fn transfer_example_audit() {
    let src = std::fs::read_to_string("../../examples/transfer.intent").unwrap();
    let report = audit(&src);
    assert_eq!(report.verify_error_count, 0);
    assert_eq!(report.obligations.len(), 2);
    assert!(report.summary.entities >= 1);
    assert!(report.summary.actions >= 1);
    assert!(report.summary.invariants >= 1);
    let trace = report.format_trace_map();
    assert!(trace.contains("TransferFunds"));
    assert!(trace.contains("Account"));
    assert!(trace.contains("Transfer"));
}

#[test]
fn coverage_format_contains_sections() {
    let report = audit("module M entity X { v: Int } action A { x: X requires { x.v > 0 } }");
    let cov = report.format_coverage();
    assert!(cov.contains("Entities:"));
    assert!(cov.contains("Actions:"));
    assert!(cov.contains("Total:"));
    assert!(cov.contains("clean"));
}

#[test]
fn line_numbers_are_valid() {
    let src = "module M\n\nentity X {\n  v: Int\n  w: Bool\n}\n";
    let report = audit(src);
    let entity = &report.entries[0];
    assert!(entity.line >= 3);
    assert!(entity.parts[0].line >= 4);
    assert!(entity.parts[1].line >= 5);
}
