# intent-check

[![crates.io](https://img.shields.io/crates/v/intent-check.svg)](https://crates.io/crates/intent-check)
[![docs](https://img.shields.io/badge/docs-mdBook-blue)](https://krakenhavoc.github.io/IntentLang/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/krakenhavoc/IntentLang/blob/main/LICENSE)

Semantic analyzer and type checker for [IntentLang](https://github.com/krakenhavoc/IntentLang) specifications.

Part of the IntentLang toolchain — a declarative specification language for human-AI collaboration.

## What it does

Runs six validation passes over a parsed IntentLang AST:

1. **Collect definitions** — detect duplicate entity/action/invariant names
2. **Resolve type references** — validate against builtins and defined entities
3. **Bind quantifier variables** — ensure `forall`/`exists` variables have valid types
4. **Validate edge case actions** — check that referenced actions exist
5. **Validate field access** — verify `entity.field` paths against entity definitions
6. **Constraint validation** — reject `old()` in preconditions, detect tautological comparisons

All errors use [miette](https://crates.io/crates/miette) diagnostics with source spans, labels, and help text.

## Usage

```rust
use intent_parser::parse_file;
use intent_check::check_file;

let source = r#"
module Example

entity User { name: String }
action Greet { user: UnknownType }
"#;

let module = parse_file(source).unwrap();
let errors = check_file(&module, source);

assert!(!errors.is_empty()); // UnknownType is not defined
```

## Public API

| Export | Description |
|--------|-------------|
| `check_file(module, src) -> Vec<CheckError>` | Run all six validation passes |
| `CheckError` | Diagnostic error type (implements `miette::Diagnostic`) |

## Part of IntentLang

This crate is the semantic analysis layer. Other crates in the workspace:

- **intent-parser** — PEG parser and typed AST
- **intent-render** — Markdown and HTML rendering
- **intent-ir** — Agent IR lowering, verification, and audit
- **intent-cli** — CLI binary (`intent check`, `intent verify`, etc.)
