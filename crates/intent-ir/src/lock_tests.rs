use std::collections::HashMap;

use crate::audit::SpecItemKind;
use crate::lock::{LockError, LockFile, extract_spec_items, format_status, lock_item, unlock_item};

fn test_items() -> HashMap<String, SpecItemKind> {
    let mut items = HashMap::new();
    items.insert("Account".to_string(), SpecItemKind::Entity);
    items.insert("Transfer".to_string(), SpecItemKind::Action);
    items.insert("NoNegativeBalances".to_string(), SpecItemKind::Invariant);
    items
}

fn new_lockfile() -> LockFile {
    LockFile {
        module: "TestModule".to_string(),
        claims: HashMap::new(),
    }
}

#[test]
fn lock_and_unlock() {
    let items = test_items();
    let mut lf = new_lockfile();

    lock_item(
        &mut lf,
        &items,
        "Account",
        "agent-1",
        "2026-03-07T12:00:00Z",
    )
    .unwrap();
    assert!(lf.claims.contains_key("Account"));
    assert_eq!(lf.claims["Account"].agent, "agent-1");

    unlock_item(&mut lf, "Account", "agent-1").unwrap();
    assert!(!lf.claims.contains_key("Account"));
}

#[test]
fn lock_conflict() {
    let items = test_items();
    let mut lf = new_lockfile();

    lock_item(
        &mut lf,
        &items,
        "Account",
        "agent-1",
        "2026-03-07T12:00:00Z",
    )
    .unwrap();
    let err = lock_item(
        &mut lf,
        &items,
        "Account",
        "agent-2",
        "2026-03-07T12:01:00Z",
    )
    .unwrap_err();
    assert_eq!(
        err,
        LockError::AlreadyClaimed {
            item: "Account".to_string(),
            owner: "agent-1".to_string(),
            requester: "agent-2".to_string(),
        }
    );
}

#[test]
fn same_agent_reclaim() {
    let items = test_items();
    let mut lf = new_lockfile();

    lock_item(
        &mut lf,
        &items,
        "Account",
        "agent-1",
        "2026-03-07T12:00:00Z",
    )
    .unwrap();
    // Same agent can reclaim (updates timestamp).
    lock_item(
        &mut lf,
        &items,
        "Account",
        "agent-1",
        "2026-03-07T12:05:00Z",
    )
    .unwrap();
    assert_eq!(lf.claims["Account"].claimed_at, "2026-03-07T12:05:00Z");
}

#[test]
fn unlock_not_claimed() {
    let mut lf = new_lockfile();
    let err = unlock_item(&mut lf, "Account", "agent-1").unwrap_err();
    assert_eq!(
        err,
        LockError::NotClaimed {
            item: "Account".to_string(),
        }
    );
}

#[test]
fn unlock_wrong_agent() {
    let items = test_items();
    let mut lf = new_lockfile();

    lock_item(
        &mut lf,
        &items,
        "Account",
        "agent-1",
        "2026-03-07T12:00:00Z",
    )
    .unwrap();
    let err = unlock_item(&mut lf, "Account", "agent-2").unwrap_err();
    assert_eq!(
        err,
        LockError::NotOwner {
            item: "Account".to_string(),
            owner: "agent-1".to_string(),
            requester: "agent-2".to_string(),
        }
    );
}

#[test]
fn lock_unknown_item() {
    let items = test_items();
    let mut lf = new_lockfile();

    let err = lock_item(
        &mut lf,
        &items,
        "Nonexistent",
        "agent-1",
        "2026-03-07T12:00:00Z",
    )
    .unwrap_err();
    assert_eq!(
        err,
        LockError::UnknownItem {
            item: "Nonexistent".to_string(),
        }
    );
}

#[test]
fn multiple_agents_different_items() {
    let items = test_items();
    let mut lf = new_lockfile();

    lock_item(
        &mut lf,
        &items,
        "Account",
        "agent-1",
        "2026-03-07T12:00:00Z",
    )
    .unwrap();
    lock_item(
        &mut lf,
        &items,
        "Transfer",
        "agent-2",
        "2026-03-07T12:00:00Z",
    )
    .unwrap();

    assert_eq!(lf.claims.len(), 2);
    assert_eq!(lf.claims["Account"].agent, "agent-1");
    assert_eq!(lf.claims["Transfer"].agent, "agent-2");
}

#[test]
fn format_status_empty() {
    let items = test_items();
    let lf = new_lockfile();
    let status = format_status(&lf, &items);
    assert!(status.contains("No items are currently claimed"));
}

#[test]
fn format_status_with_claims() {
    let items = test_items();
    let mut lf = new_lockfile();
    lock_item(
        &mut lf,
        &items,
        "Account",
        "agent-1",
        "2026-03-07T12:00:00Z",
    )
    .unwrap();

    let status = format_status(&lf, &items);
    assert!(status.contains("Account"));
    assert!(status.contains("agent-1"));
    assert!(status.contains("Unclaimed"));
    assert!(status.contains("Transfer"));
}

#[test]
fn lockfile_serialization_roundtrip() {
    let items = test_items();
    let mut lf = new_lockfile();
    lock_item(
        &mut lf,
        &items,
        "Account",
        "agent-1",
        "2026-03-07T12:00:00Z",
    )
    .unwrap();

    let json = serde_json::to_string(&lf).unwrap();
    let restored: LockFile = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.claims["Account"].agent, "agent-1");
}

#[test]
fn extract_items_from_audit() {
    let src =
        "module M entity X { v: Int } action A { x: X } invariant P { forall x: X => x.v >= 0 }";
    let ast = intent_parser::parse_file(src).unwrap();
    let ir = crate::lower::lower_file(&ast);
    let errors = crate::verify::verify_module(&ir);
    let obligations = crate::verify::analyze_obligations(&ir);
    let report = crate::audit::generate_audit(src, &ir, &errors, &obligations);

    let items = extract_spec_items(&report);
    assert!(items.contains_key("X"));
    assert!(items.contains_key("A"));
    assert!(items.contains_key("P"));
    assert_eq!(items["X"], SpecItemKind::Entity);
    assert_eq!(items["A"], SpecItemKind::Action);
    assert_eq!(items["P"], SpecItemKind::Invariant);
}
