# Crate Structure

IntentLang is a Rust workspace with eight crates:

```
intent-cli ──→ intent-parser ←── intent.pest (PEG grammar)
    │              ↑
    ├──→ intent-check
    ├──→ intent-render
    ├──→ intent-ir (lowering, verification, audit, diff, incremental, lock)
    ├──→ intent-gen (NL → .intent, Layer 0)
    └──→ intent-runtime (stateless execution, HTTP server)

intent-lsp ──→ intent-parser, intent-check (LSP server)
```

## intent-parser

PEG grammar (via [pest](https://pest.rs/)) and AST definitions. Parses `.intent` files into a typed AST with source spans on every node. Includes a module resolver for `use` imports (DFS with cycle detection).

**Key modules:** `ast.rs` (AST node types), `parser.rs` (pest parser wrapper), `resolve.rs` (module resolver)

## intent-check

Six-pass semantic analyzer. Validates type references, field access, quantifier bindings, `old()` placement, and more. Supports cross-module type checking via `check_file_with_imports()`. Produces diagnostic errors via [miette](https://crates.io/crates/miette).

**Key modules:** `types.rs` (type checking), `constraints.rs` (constraint validation)

## intent-render

Renders parsed specs to Markdown, self-contained HTML, or canonical `.intent` source (formatter). Produces entity field tables, action signatures, and formatted invariants.

**Key modules:** `markdown.rs`, `html.rs`, `format.rs`

## intent-ir

AST-to-IR lowering, structural verification, coherence analysis, audit bridge, spec diffing, incremental verification, and multi-agent locking. Every IR node carries a `SourceTrace` linking back to the original spec.

**Key modules:** `lower.rs` (AST → IR), `verify.rs` (verification), `audit.rs` (audit bridge), `diff.rs` (spec diffs), `incremental.rs` (cached verification), `lock.rs` (multi-agent claims)

## intent-gen

Translates natural language descriptions into validated `.intent` specs via any OpenAI-compatible LLM API. Includes a generate-check-retry loop that feeds parse/check errors back to the LLM for correction.

**Key modules:** `prompt.rs` (system prompt), `client.rs` (LLM API client), `validate.rs` (generation loop)

## intent-runtime

Stateless execution engine. Evaluates expressions against concrete JSON values, enforces pre/postconditions and invariants, and auto-generates REST endpoints from actions via HTTP server.

**Key modules:** `eval.rs` (expression evaluator), `contract.rs` (contract evaluation), `serve.rs` (HTTP server)

## intent-lsp

Language Server Protocol server using [tower-lsp](https://crates.io/crates/tower-lsp) and [tokio](https://tokio.rs/). Provides real-time diagnostics, go-to-definition, hover, and context-aware completion for `.intent` files. Used by the VSCode extension.

**Key modules:** `server.rs` (LSP backend), `document.rs` (per-file state + line index), `diagnostics.rs`, `hover.rs`, `navigation.rs`, `completion.rs`

## intent-cli

CLI entry point using [clap](https://crates.io/crates/clap) (derive). Wires together all other crates and exposes subcommands: `check`, `render`, `render-html`, `compile`, `verify` (`--incremental`), `audit`, `coverage`, `diff`, `query`, `lock`, `unlock`, `status`, `fmt`, `init`, `completions`, `generate`, `serve`. Supports `--output json` for agent consumption.

## Published crates

Seven crates are published to [crates.io](https://crates.io/):

- [intent-parser](https://crates.io/crates/intent-parser)
- [intent-check](https://crates.io/crates/intent-check)
- [intent-render](https://crates.io/crates/intent-render)
- [intent-ir](https://crates.io/crates/intent-ir)
- [intent-gen](https://crates.io/crates/intent-gen)
- [intent-runtime](https://crates.io/crates/intent-runtime)
- [intent-cli](https://crates.io/crates/intent-cli)
