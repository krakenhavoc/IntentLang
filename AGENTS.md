# AGENTS.md -- AI Agent Onboarding Guide

This file is for AI agents and coding assistants working on IntentLang.
For the full project spec, see `docs/SPEC.md`. For contributor conventions, see `CLAUDE.md`.

## What Is IntentLang?

IntentLang is a declarative specification language for human-AI collaboration. Humans write **what** and **what constraints** in `.intent` files; agents handle **how** via a compiled intermediate representation. Three layers:

1. **Intent Layer** -- The spec language (this is what we're building now)
2. **Agent IR** -- A verifiable intermediate representation agents generate (future)
3. **Audit Bridge** -- Maps between layers so humans can review agent work (future)

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
```

## Project Structure

```
intentlang/
  grammar/intent.pest        -- PEG grammar (pest)
  crates/
    intent-parser/           -- Grammar -> typed AST
    intent-check/            -- Semantic analysis & validation
    intent-render/           -- AST -> Markdown (HTML stub)
    intent-cli/              -- CLI binary: `intent check`, `intent render`
  examples/                  -- Example .intent files
  tests/valid/               -- Specs that must parse and pass checks
  tests/invalid/             -- Specs that must fail with known errors
  docs/SPEC.md               -- Full language specification
```

### Crate Dependency Graph

```
intent-cli -> intent-parser, intent-check, intent-render
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

**Checker (intent-check)**: Three-pass semantic analysis:
1. Collect definitions + detect duplicates (entities, actions, invariants, fields)
2. Resolve type references (verify all types exist as builtins or defined entities)
3. Validate quantifier bindings (forall/exists variable types must be entities or actions)

Errors use `miette` diagnostics with source spans, labels, and help text.

**Renderer (intent-render)**: Converts AST to Markdown. HTML renderer is a stub.

**CLI (intent-cli)**: `clap` derive-based. Two subcommands: `check` and `render`.

## Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| pest / pest_derive | 2.x | PEG parsing |
| miette | 7.x | Diagnostic error reporting |
| thiserror | 2.x | Error type derivation |
| clap | 4.x | CLI argument parsing |
| serde | 1.x | AST serialization |

## Conventions

- **Tests first**: Write a failing test before implementing a feature.
- **Error messages matter**: Include source spans and actionable suggestions.
- **Crates stay focused**: Parser doesn't validate semantics. Checker doesn't render.
- **Grammar rules get comments**: Link to the relevant SPEC.md section.
- Run `cargo test --workspace` before committing. All tests must pass.

## Current Test Coverage

- 17 semantic checker tests (duplicates, type resolution, quantifiers, valid files)
- 7 parser unit tests (all language constructs)
- 7 test fixtures (4 valid, 3 invalid) + 3 example files

## Current Phase & Status

Phase 1 MVP. The parser, semantic checker, Markdown renderer, and CLI are functional.

Remaining work:
- Snapshot tests (insta) for AST regression
- HTML renderer
- Richer parse error messages with miette source spans
- Constraint satisfiability checking (constraints.rs is a stub)
