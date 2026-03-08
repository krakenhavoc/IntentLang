//! Beta milestone integration tests.
//!
//! These tests exercise the full IntentLang CLI pipeline against a realistic
//! multi-module task tracker system in `examples/beta/`. They validate that
//! every command works end-to-end: parse, check, render, compile, verify,
//! audit, coverage, query, codegen, openapi, test, test-harness, fmt, diff,
//! lock/unlock/status.

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
