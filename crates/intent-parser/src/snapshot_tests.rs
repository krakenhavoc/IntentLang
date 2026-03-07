//! Insta snapshot tests for parser AST output.
//!
//! Each test parses a fixture or example file and snapshots the full AST as
//! JSON. If the parser or AST structure changes, `cargo insta review` will
//! show the diff and let you accept or reject.

use crate::parse_file;

fn snap(name: &str, src: &str) {
    let ast = parse_file(src).expect("parse should succeed");
    insta::assert_json_snapshot!(name, ast);
}

// ── Valid fixtures ──────────────────────────────────────────

#[test]
fn snapshot_minimal() {
    snap("minimal", include_str!("../../../tests/valid/minimal.intent"));
}

#[test]
fn snapshot_entity_only() {
    snap(
        "entity_only",
        include_str!("../../../tests/valid/entity_only.intent"),
    );
}

#[test]
fn snapshot_all_types() {
    snap(
        "all_types",
        include_str!("../../../tests/valid/all_types.intent"),
    );
}

#[test]
fn snapshot_full_action() {
    snap(
        "full_action",
        include_str!("../../../tests/valid/full_action.intent"),
    );
}

// ── Examples ────────────────────────────────────────────────

#[test]
fn snapshot_transfer() {
    snap(
        "transfer",
        include_str!("../../../examples/transfer.intent"),
    );
}

#[test]
fn snapshot_auth() {
    snap("auth", include_str!("../../../examples/auth.intent"));
}

#[test]
fn snapshot_shopping_cart() {
    snap(
        "shopping_cart",
        include_str!("../../../examples/shopping_cart.intent"),
    );
}
