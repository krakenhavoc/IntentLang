//! Beta milestone integration tests.
//!
//! These tests exercise the full IntentLang CLI pipeline against a realistic
//! multi-module task tracker system in `examples/beta/`. They validate that
//! every command works end-to-end: parse, check, render, compile, verify,
//! audit, coverage, query, codegen, openapi, test, test-harness, fmt, diff,
//! lock/unlock/status, add-invariant.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn beta_file(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/beta")
        .join(name)
}

fn intent_cmd() -> Command {
    Command::cargo_bin("intent").unwrap()
}

// ── check ──────────────────────────────────────────────────────────────

#[test]
fn check_single_module() {
    intent_cmd()
        .args(["check", beta_file("Core.intent").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("OK: Core"));
}

#[test]
fn check_with_imports() {
    intent_cmd()
        .args(["check", beta_file("Projects.intent").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("OK: Projects"));
}

#[test]
fn check_transitive_imports() {
    intent_cmd()
        .args(["check", beta_file("Tracking.intent").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("OK: Tracking"));
}

#[test]
fn check_json_output() {
    intent_cmd()
        .args([
            "--output",
            "json",
            "check",
            beta_file("Projects.intent").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\": true"));
}

// ── render ─────────────────────────────────────────────────────────────

#[test]
fn render_markdown() {
    intent_cmd()
        .args(["render", beta_file("Core.intent").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Core"));
}

#[test]
fn render_html() {
    intent_cmd()
        .args(["render-html", beta_file("Core.intent").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("<html"));
}

// ── compile ────────────────────────────────────────────────────────────

#[test]
fn compile_produces_ir_json() {
    intent_cmd()
        .args(["compile", beta_file("Projects.intent").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"Projects\""));
}

// ── verify ─────────────────────────────────────────────────────────────

#[test]
fn verify_succeeds() {
    intent_cmd()
        .args(["verify", beta_file("Projects.intent").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("VERIFIED"));
}

#[test]
fn verify_incremental() {
    intent_cmd()
        .args([
            "verify",
            "--incremental",
            beta_file("Tracking.intent").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("VERIFIED"));
}

// ── audit & coverage ──────────────────────────────────────────────────

#[test]
fn audit_shows_trace_map() {
    intent_cmd()
        .args(["audit", beta_file("Tracking.intent").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Audit Trace Map"));
}

#[test]
fn coverage_shows_summary() {
    intent_cmd()
        .args(["coverage", beta_file("Tracking.intent").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Coverage Summary"));
}

// ── query ──────────────────────────────────────────────────────────────

#[test]
fn query_entities() {
    intent_cmd()
        .args([
            "query",
            beta_file("Projects.intent").to_str().unwrap(),
            "entities",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Project").and(predicate::str::contains("Task")));
}

#[test]
fn query_actions() {
    intent_cmd()
        .args([
            "query",
            beta_file("Projects.intent").to_str().unwrap(),
            "actions",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("CreateTask"));
}

#[test]
fn query_by_name() {
    intent_cmd()
        .args([
            "query",
            beta_file("Projects.intent").to_str().unwrap(),
            "CreateTask",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("CreateTask"));
}

// ── codegen ────────────────────────────────────────────────────────────

#[test]
fn codegen_all_languages() {
    let languages = [
        "rust",
        "typescript",
        "python",
        "go",
        "java",
        "csharp",
        "swift",
    ];
    for lang in languages {
        intent_cmd()
            .args([
                "codegen",
                beta_file("Projects.intent").to_str().unwrap(),
                "--lang",
                lang,
            ])
            .assert()
            .success()
            .stdout(predicate::str::is_empty().not());
    }
}

// ── openapi ────────────────────────────────────────────────────────────

#[test]
fn openapi_produces_valid_json() {
    intent_cmd()
        .args(["openapi", beta_file("Projects.intent").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"openapi\""));
}

// ── test ───────────────────────────────────────────────────────────────

#[test]
fn spec_tests_pass_projects() {
    intent_cmd()
        .args(["test", beta_file("Projects.intent").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("3 passed"));
}

#[test]
fn spec_tests_pass_tracking() {
    intent_cmd()
        .args(["test", beta_file("Tracking.intent").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("3 passed"));
}

// ── test-harness ───────────────────────────────────────────────────────

#[test]
fn test_harness_produces_rust_tests() {
    intent_cmd()
        .args([
            "test-harness",
            beta_file("Projects.intent").to_str().unwrap(),
            "--lang",
            "rust",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("#[cfg(test)]"));
}

// ── fmt ────────────────────────────────────────────────────────────────

#[test]
fn fmt_check_passes() {
    intent_cmd()
        .args(["fmt", "--check", beta_file("Core.intent").to_str().unwrap()])
        .assert()
        .success();
}

// ── diff ───────────────────────────────────────────────────────────────

#[test]
fn diff_same_file() {
    let path = beta_file("Core.intent");
    let p = path.to_str().unwrap();
    intent_cmd()
        .args(["diff", p, p])
        .assert()
        .success()
        .stdout(predicate::str::contains("No spec-level changes"));
}

// ── lock/unlock/status ─────────────────────────────────────────────────

#[test]
fn lock_status_unlock_cycle() {
    let file = beta_file("Projects.intent");
    let p = file.to_str().unwrap();

    // Lock
    intent_cmd()
        .args(["lock", p, "CreateTask", "--agent", "integration-test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Locked"));

    // Status shows claim
    intent_cmd()
        .args(["status", p])
        .assert()
        .success()
        .stdout(predicate::str::contains("integration-test"));

    // Unlock
    intent_cmd()
        .args(["unlock", p, "CreateTask", "--agent", "integration-test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Unlocked"));
}

// ── State machine sugar ─────────────────────────────────────────

fn example_file(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

#[test]
fn state_machine_check() {
    intent_cmd()
        .args([
            "check",
            example_file("task_states.intent").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("OK: TaskTracker"));
}

#[test]
fn state_machine_codegen_rust() {
    intent_cmd()
        .args([
            "codegen",
            example_file("task_states.intent").to_str().unwrap(),
            "--lang",
            "rust",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("pub enum TaskStatus"))
        .stdout(predicate::str::contains("is_valid_transition"));
}

#[test]
fn state_machine_codegen_typescript() {
    intent_cmd()
        .args([
            "codegen",
            example_file("task_states.intent").to_str().unwrap(),
            "--lang",
            "typescript",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("export type TaskStatus"))
        .stdout(predicate::str::contains("isValidTaskStatusTransition"));
}

#[test]
fn state_machine_render() {
    intent_cmd()
        .args([
            "render",
            example_file("task_states.intent").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("State Machine: TaskStatus"))
        .stdout(predicate::str::contains("`Open` → `InProgress`"));
}

#[test]
fn state_machine_fmt() {
    intent_cmd()
        .args(["fmt", example_file("task_states.intent").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("state TaskStatus {"))
        .stdout(predicate::str::contains("Open -> InProgress -> Done"));
}

// ── add-invariant ────────────────────────────────────────────

/// Helper to create a temporary .intent file with the given content.
fn write_temp_intent(name: &str, content: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("intent-test-add-invariant");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

const SAMPLE_SPEC: &str = r#"module TestSpec

entity Account {
  id: UUID
  balance: Decimal(precision: 2)
  owner_id: UUID
  status: Active | Frozen
}

entity User {
  id: UUID
  name: String
}

action FreezeAccount {
  account: Account
  reason: String

  requires {
    account.status == Active
  }

  ensures {
    account.status == Frozen
  }
}

action Transfer {
  from: Account
  to: Account
  amount: Decimal(precision: 2)

  requires {
    amount > 0
  }

  ensures {
    from.balance == old(from.balance) - amount
  }

  properties {
    atomic: true
  }
}
"#;

#[test]
fn add_invariant_unique_dry_run() {
    let path = write_temp_intent("unique_dry.intent", SAMPLE_SPEC);
    intent_cmd()
        .args([
            "add-invariant",
            path.to_str().unwrap(),
            "--pattern",
            "unique",
            "--entity",
            "Account",
            "id",
        ])
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("invariant UniqueAccountId"))
        .stdout(predicate::str::contains(
            "forall a: Account => forall b: Account",
        ))
        .stdout(predicate::str::contains("a != b => a.id != b.id"));
    // Verify file was NOT modified
    let content = std::fs::read_to_string(&path).unwrap();
    assert_eq!(content, SAMPLE_SPEC);
}

#[test]
fn add_invariant_non_negative_dry_run() {
    let path = write_temp_intent("nonneg_dry.intent", SAMPLE_SPEC);
    intent_cmd()
        .args([
            "add-invariant",
            path.to_str().unwrap(),
            "--pattern",
            "non-negative",
            "--entity",
            "Account",
            "balance",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "invariant NonNegativeAccountBalance",
        ))
        .stdout(predicate::str::contains(
            "forall a: Account => a.balance >= 0",
        ));
}

#[test]
fn add_invariant_no_dangling_ref_dry_run() {
    let path = write_temp_intent("noref_dry.intent", SAMPLE_SPEC);
    intent_cmd()
        .args([
            "add-invariant",
            path.to_str().unwrap(),
            "--pattern",
            "no-dangling-ref",
            "--entity",
            "Account",
            "--dry-run",
            "owner_id",
            "User",
            "id",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "invariant NoDanglingAccountOwnerId",
        ))
        .stdout(predicate::str::contains(
            "forall a: Account => exists b: User => a.owner_id == b.id",
        ));
}

#[test]
fn add_invariant_idempotent_dry_run() {
    let path = write_temp_intent("idemp_dry.intent", SAMPLE_SPEC);
    intent_cmd()
        .args([
            "add-invariant",
            path.to_str().unwrap(),
            "--pattern",
            "idempotent",
            "--action",
            "FreezeAccount",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("idempotent: true"));
}

#[test]
fn add_invariant_unique_modifies_file() {
    let path = write_temp_intent("unique_write.intent", SAMPLE_SPEC);
    intent_cmd()
        .args([
            "add-invariant",
            path.to_str().unwrap(),
            "--pattern",
            "unique",
            "--entity",
            "Account",
            "id",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Added invariant 'UniqueAccountId'",
        ));
    // Verify file was modified and the new invariant is present
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("invariant UniqueAccountId"));
    // Verify the modified file still parses and checks OK
    intent_cmd()
        .args(["check", path.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn add_invariant_entity_not_found() {
    let path = write_temp_intent("no_entity.intent", SAMPLE_SPEC);
    intent_cmd()
        .args([
            "add-invariant",
            path.to_str().unwrap(),
            "--pattern",
            "unique",
            "--entity",
            "NonExistent",
            "id",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("entity 'NonExistent' not found"));
}

#[test]
fn add_invariant_field_not_found() {
    let path = write_temp_intent("no_field.intent", SAMPLE_SPEC);
    intent_cmd()
        .args([
            "add-invariant",
            path.to_str().unwrap(),
            "--pattern",
            "unique",
            "--entity",
            "Account",
            "nonexistent_field",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("has no field 'nonexistent_field'"));
}

#[test]
fn add_invariant_wrong_field_count() {
    let path = write_temp_intent("wrong_count.intent", SAMPLE_SPEC);
    // unique requires 1 field, give 2
    intent_cmd()
        .args([
            "add-invariant",
            path.to_str().unwrap(),
            "--pattern",
            "unique",
            "--entity",
            "Account",
            "id",
            "balance",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "requires exactly 1 field argument",
        ));
}

#[test]
fn add_invariant_missing_entity_flag() {
    let path = write_temp_intent("no_flag.intent", SAMPLE_SPEC);
    intent_cmd()
        .args([
            "add-invariant",
            path.to_str().unwrap(),
            "--pattern",
            "unique",
            "id",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--entity is required"));
}

#[test]
fn add_invariant_action_not_found() {
    let path = write_temp_intent("no_action.intent", SAMPLE_SPEC);
    intent_cmd()
        .args([
            "add-invariant",
            path.to_str().unwrap(),
            "--pattern",
            "idempotent",
            "--action",
            "NonExistent",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("action 'NonExistent' not found"));
}

#[test]
fn add_invariant_idempotent_writes_property() {
    let path = write_temp_intent("idemp_write.intent", SAMPLE_SPEC);
    intent_cmd()
        .args([
            "add-invariant",
            path.to_str().unwrap(),
            "--pattern",
            "idempotent",
            "--action",
            "FreezeAccount",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added idempotent property"));
    // Verify file was modified
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("idempotent: true"));
}

#[test]
fn add_invariant_idempotent_existing_properties() {
    // Transfer already has properties { atomic: true }
    let path = write_temp_intent("idemp_existing.intent", SAMPLE_SPEC);
    intent_cmd()
        .args([
            "add-invariant",
            path.to_str().unwrap(),
            "--pattern",
            "idempotent",
            "--action",
            "Transfer",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added idempotent property"));
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("idempotent: true"));
    assert!(content.contains("atomic: true"));
}

// ── suggest ────────────────────────────────────────────────────

#[test]
fn suggest_produces_suggestions_for_transfer() {
    intent_cmd()
        .args(["suggest", example_file("transfer.intent").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Analyzing"))
        .stdout(predicate::str::contains("suggestion(s) for TransferFunds"));
}

#[test]
fn suggest_json_output() {
    intent_cmd()
        .args([
            "--output",
            "json",
            "suggest",
            example_file("transfer.intent").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"suggestions\""))
        .stdout(predicate::str::contains("\"count\""));
}

#[test]
fn suggest_shopping_cart() {
    intent_cmd()
        .args([
            "suggest",
            example_file("shopping_cart.intent").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("suggestion(s) for ShoppingCart"));
}

#[test]
fn suggest_auth_spec() {
    intent_cmd()
        .args(["suggest", example_file("auth.intent").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("suggestion(s)"));
}

#[test]
fn suggest_well_specified_has_fewer_warnings() {
    // The transfer.intent example is well-specified; it should still produce some
    // suggestions (e.g., uniqueness for TransferRecord) but not flag Transfer as
    // missing atomic/audit_logged since those properties exist.
    let output = intent_cmd()
        .args([
            "--output",
            "json",
            "suggest",
            example_file("transfer.intent").to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    // Transfer already has atomic: true and audit_logged: true
    assert!(
        !text.contains("Transfer.atomic"),
        "should not suggest atomic for Transfer (already has it)"
    );
    assert!(
        !text.contains("Transfer.audit_logged"),
        "should not suggest audit_logged for Transfer (already has it)"
    );
}
