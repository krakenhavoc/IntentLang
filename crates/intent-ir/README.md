# intent-ir

[![crates.io](https://img.shields.io/crates/v/intent-ir.svg)](https://crates.io/crates/intent-ir)
[![docs](https://img.shields.io/badge/docs-mdBook-blue)](https://krakenhavoc.github.io/IntentLang/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/krakenhavoc/IntentLang/blob/main/LICENSE)

Agent IR lowering, verification, audit bridge, and collaboration tooling for [IntentLang](https://github.com/krakenhavoc/IntentLang).

Part of the IntentLang toolchain — a declarative specification language for human-AI collaboration.

## What it does

Lowers IntentLang ASTs into a typed intermediate representation and provides verification, auditing, diffing, incremental caching, and multi-agent locking:

- **Lowering** — AST to IR (structs, functions, invariants, edge guards). Every IR node carries a `SourceTrace` linking back to the original spec.
- **Verification** — structural checks (type resolution, field access, `old()` placement) and coherence analysis (verification obligations).
- **Audit** — trace maps from spec items to IR constructs with source lines. Coverage summaries.
- **Diff** — spec-level diffs between two versions of a spec.
- **Incremental** — per-item verification caching. Re-verifies only items whose content changed.
- **Lock** — multi-agent spec-item claiming. Prevents conflicts when multiple agents work on the same spec.

## Usage

```rust
use intent_parser::parse_file;
use intent_ir::{lower_file, verify_module, generate_audit};

let source = std::fs::read_to_string("spec.intent").unwrap();
let ast = parse_file(&source).unwrap();

// Lower to IR
let ir = lower_file(&ast);

// Verify structural correctness + coherence
let errors = verify_module(&ir);
let obligations = intent_ir::analyze_obligations(&ir);

// Generate audit trace map
let report = generate_audit(&ir);
```

## Modules

| Module | Public API | Description |
|--------|-----------|-------------|
| `lower` | `lower_file()` | AST to IR lowering |
| `verify` | `verify_module()`, `analyze_obligations()` | Structural verification and coherence |
| `audit` | `generate_audit()` | Audit trace maps and coverage |
| `diff` | `diff_reports()` | Spec-level diffs between versions |
| `incremental` | `incremental_verify()`, `VerifyCache` | Cached per-item verification |
| `lock` | `lock_item()`, `unlock_item()`, `format_status()` | Multi-agent collaboration |

## Part of IntentLang

This crate is the IR and verification layer. Other crates in the workspace:

- **intent-parser** — PEG parser and typed AST
- **intent-check** — Semantic analysis and type checking
- **intent-render** — Markdown and HTML rendering
- **intent-cli** — CLI binary (`intent check`, `intent verify`, etc.)
