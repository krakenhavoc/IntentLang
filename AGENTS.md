# AGENTS.md -- AI Agent Onboarding Guide

This file is for AI agents and coding assistants working on IntentLang.
For the full project spec, see `docs/SPEC.md`. For contributor conventions, see `CLAUDE.md`.

## What Is IntentLang?

IntentLang is a declarative specification language for human-AI collaboration. Humans write **what** and **what constraints** in `.intent` files; agents handle **how** via a compiled intermediate representation. Three layers:

1. **Intent Layer** -- The spec language
2. **Agent IR** -- A verifiable intermediate representation agents generate
3. **Audit Bridge** -- Maps between layers so humans can review agent work

## Quick Start

```bash
# Install Rust (if not present)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Run the test suite
cargo test --workspace

# Build the CLI
cargo build -p intent-cli

# Check a spec file
cargo run -p intent-cli -- check examples/transfer.intent

# Render a spec to Markdown
cargo run -p intent-cli -- render examples/transfer.intent

# Compile to IR (JSON output)
cargo run -p intent-cli -- compile examples/transfer.intent

# Verify structural + logical correctness
cargo run -p intent-cli -- verify examples/transfer.intent

# Show audit trace map (spec items → IR constructs)
cargo run -p intent-cli -- audit examples/transfer.intent

# Show coverage summary
cargo run -p intent-cli -- coverage examples/transfer.intent

# Diff two versions of a spec
cargo run -p intent-cli -- diff old.intent new.intent

# Query specific items (for agent integration)
cargo run -p intent-cli -- query examples/transfer.intent entities
cargo run -p intent-cli -- query examples/transfer.intent Transfer

# Incremental verification (caches results, re-verifies only changes)
cargo run -p intent-cli -- verify --incremental examples/transfer.intent

# Multi-agent collaboration: lock/unlock spec items
cargo run -p intent-cli -- lock examples/transfer.intent Transfer --agent agent-1
cargo run -p intent-cli -- status examples/transfer.intent
cargo run -p intent-cli -- unlock examples/transfer.intent Transfer --agent agent-1

# JSON output (for agent consumption)
cargo run -p intent-cli -- --output json check examples/transfer.intent
```

## Project Structure

```
intentlang/
  grammar/intent.pest        -- PEG grammar (pest)
  crates/
    intent-parser/           -- Grammar -> typed AST
    intent-check/            -- Semantic analysis & validation
    intent-render/           -- AST -> Markdown/HTML
    intent-ir/               -- AST -> Agent IR (lowering, verification, audit, diff, incremental, lock)
    intent-cli/              -- CLI binary: check, render, compile, verify, audit, coverage, diff, query, lock, unlock, status
  examples/                  -- Example .intent files
  tests/valid/               -- Specs that must parse and pass checks
  tests/invalid/             -- Specs that must fail with known errors
  docs/SPEC.md               -- Full language specification
```

### Crate Dependency Graph

```
intent-cli -> intent-parser, intent-check, intent-render, intent-ir
intent-ir -> intent-parser
intent-check -> intent-parser
intent-render -> intent-parser
intent-parser -> pest (grammar/intent.pest)
```

## The Intent Language (Syntax Reference)

```intent
module ModuleName

--- Documentation block (triple-dash, natural language)

entity EntityName {
  field_name: Type
  status: Active | Frozen         -- union (enum-like labels)
  email: Email?                   -- optional
  items: List<Item>               -- collection
}

action ActionName {
  param: Type

  requires {                      -- preconditions (boolean expressions)
    param.field > 0
  }
  ensures {                       -- postconditions, old() for pre-state
    param.field == old(param.field) - 1
  }
  properties {                    -- key-value metadata
    atomic: true
  }
}

invariant Name {
  forall x: Entity => predicate   -- universal constraint
}

edge_cases {
  when condition => action        -- edge case handling
}
```

### Built-in Types
`UUID`, `String`, `Int`, `Decimal(precision: N)`, `Bool`, `DateTime`, `CurrencyCode`, `Email`, `URL`

### Collections
`List<T>`, `Set<T>`, `Map<K, V>`

### Modifiers
`T?` (optional), `A | B | C` (union/enum)

## Architecture Notes

**Parser (intent-parser)**: PEG grammar via `pest`. Every AST node carries a `Span { start, end }` for source locations. The grammar uses `or_expr` (not `expr`) for `when`/`edge_rule` conditions to avoid ambiguity with the `=>` operator. Union variants like `Active | Frozen` are treated as enum-like labels, not type references.

**Checker (intent-check)**: Six-pass semantic analysis:
1. Collect definitions + detect duplicates (entities, actions, invariants, fields)
2. Resolve type references (verify all types exist as builtins or defined entities)
3. Validate quantifier bindings (forall/exists variable types must be entities or actions)
4. Validate edge case action references (uppercase names must be defined actions)
5. Validate field access on entity-typed parameters (e.g., `from.balance` checks `balance` exists on the entity)
6. Constraint validation (`old()` not in requires, tautological self-comparisons)

Both parse and check errors use `miette` diagnostics with source spans, labels, and help text.

**Renderer (intent-render)**: Converts AST to Markdown and self-contained HTML. Shared `format_type` helper in lib root.

**IR (intent-ir)**: Lowers AST to a typed intermediate representation (structs, functions, invariants, edge guards). Every IR node carries a `SourceTrace { module, item, part, span }` for audit tracing. Modules: `lower` (AST→IR), `verify` (structural + coherence), `audit` (trace maps + coverage), `diff` (spec-level diffs), `incremental` (cached per-item verification), `lock` (multi-agent spec-item claiming).

**CLI (intent-cli)**: `clap` derive-based. Subcommands: `check`, `render`, `render-html`, `compile`, `verify` (`--incremental`), `audit`, `coverage`, `diff`, `query`, `lock`, `unlock`, `status`. Global `--output json` flag for agent consumption.

## Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| pest / pest_derive | 2.x | PEG parsing |
| miette | 7.x | Diagnostic error reporting |
| thiserror | 2.x | Error type derivation |
| clap | 4.x | CLI argument parsing |
| serde | 1.x | AST/IR serialization |
| serde_json | 1.x | IR JSON output |

## Conventions

- **Tests first**: Write a failing test before implementing a feature.
- **Error messages matter**: Include source spans and actionable suggestions.
- **Crates stay focused**: Parser doesn't validate semantics. Checker doesn't render.
- **Grammar rules get comments**: Link to the relevant SPEC.md section.
- Run `cargo test --workspace` before committing. All tests must pass.

## Current Test Coverage (112 total)

- 26 semantic checker tests (duplicates, type resolution, quantifiers, edge actions, field access, constraints, valid files)
- 14 parser tests (7 unit + 7 insta snapshot tests for all fixtures and examples)
- 72 IR tests: 11 lowering + 11 verification + 6 coherence + 9 audit + 13 diff + 11 incremental + 11 lock
- Fixtures: 4 valid, 9 invalid + 3 example files

## Current Phase & Status

All four phases complete. Current release: v0.4.0-alpha.1.

Phase 1 (complete): PEG grammar, typed AST with spans, six-pass semantic analysis, Markdown/HTML renderers. CLI: `check`, `render`, `render-html`.

Phase 2 (complete): AST → IR lowering, structural verification, coherence analysis. CLI: `compile`, `verify`.

Phase 3 (complete): Audit trace maps, coverage summaries, spec-level diffs. CLI: `audit`, `coverage`, `diff`.

Phase 4 (complete): Agent API (`--output json`, `query`), incremental verification (`verify --incremental`), multi-agent collaboration (`lock`, `unlock`, `status`).
