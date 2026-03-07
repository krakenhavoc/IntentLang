# Crate Structure

IntentLang is a Rust workspace with five crates:

```
intent-cli ──→ intent-parser ←── intent.pest (PEG grammar)
    │              ↑
    ├──→ intent-check
    ├──→ intent-render
    └──→ intent-ir (lowering, verification, audit)
```

## intent-parser

PEG grammar (via [pest](https://pest.rs/)) and AST definitions. Parses `.intent` files into a typed AST with source spans on every node.

**Key modules:** `ast.rs` (AST node types), `parser.rs` (pest parser wrapper)

## intent-check

Six-pass semantic analyzer. Validates type references, field access, quantifier bindings, `old()` placement, and more. Produces diagnostic errors via [miette](https://crates.io/crates/miette).

**Key modules:** `types.rs` (type checking), `constraints.rs` (constraint validation)

## intent-render

Renders parsed specs to Markdown or self-contained HTML. Produces entity field tables, action signatures, and formatted invariants.

**Key modules:** `markdown.rs`, `html.rs`

## intent-ir

AST-to-IR lowering, structural verification, coherence analysis, and audit bridge. Every IR node carries a `SourceTrace` linking back to the original spec.

**Key modules:** `lower.rs` (AST → IR), `verify.rs` (verification), `audit.rs` (audit bridge)

## intent-cli

CLI entry point using [clap](https://crates.io/crates/clap) (derive). Wires together all other crates and exposes the `check`, `render`, `compile`, `verify`, `audit`, and `coverage` subcommands.

## All crates on crates.io

All five crates are published to [crates.io](https://crates.io/):

- [intent-parser](https://crates.io/crates/intent-parser)
- [intent-check](https://crates.io/crates/intent-check)
- [intent-render](https://crates.io/crates/intent-render)
- [intent-ir](https://crates.io/crates/intent-ir)
- [intent-cli](https://crates.io/crates/intent-cli)
