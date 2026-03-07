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

# Compile to IR (JSON output)
cargo run -p intent-cli -- compile examples/transfer.intent

# Verify structural + logical correctness
cargo run -p intent-cli -- verify examples/transfer.intent

# Show audit trace map (spec items → IR constructs)
cargo run -p intent-cli -- audit examples/transfer.intent

# Show coverage summary
cargo run -p intent-cli -- coverage examples/transfer.intent
```

## Project Structure

```
intentlang/
  grammar/intent.pest        -- PEG grammar (pest)
  crates/
    intent-parser/           -- Grammar -> typed AST
    intent-check/            -- Semantic analysis & validation
    intent-render/           -- AST -> Markdown/HTML
    intent-ir/               -- AST -> Agent IR (lowering, verification)
    intent-cli/              -- CLI binary: check, render, compile, verify, audit, coverage
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

**IR (intent-ir)**: Lowers AST to a typed intermediate representation (structs, functions, invariants, edge guards). Every IR node carries a `SourceTrace { module, item, part, span }` for audit tracing. The verification pass checks structural correctness (bound variables, `old()` placement, quantifier types, postcondition connectivity) and logical coherence (invariant-action field coverage, verification obligations). The audit module generates trace maps (spec→IR mapping with source lines) and coverage summaries from the IR and verification results.

**CLI (intent-cli)**: `clap` derive-based. Subcommands: `check`, `render`, `render-html`, `compile`, `verify`, `audit`, `coverage`.

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

## Current Test Coverage (77 total)

- 26 semantic checker tests (duplicates, type resolution, quantifiers, edge actions, field access, constraints, valid files)
- 14 parser tests (7 unit + 7 insta snapshot tests for all fixtures and examples)
- 28 IR tests (11 lowering + 11 verification + 6 coherence, including integration tests for all 3 examples)
- 9 audit tests (trace map entries, coverage counts, line numbers, obligation display, integration)
- Fixtures: 4 valid, 9 invalid + 3 example files

## Current Phase & Status

Phase 3: Audit Bridge. Building on Phase 2 (IR + verification).

Phase 1 (complete):
- PEG grammar, typed AST with spans, snapshot tests (insta)
- Six-pass semantic analysis with miette diagnostics
- Markdown and HTML renderers
- CLI: `check`, `render`, `render-html`

Phase 2 (complete):
- AST → IR lowering (entities→structs, actions→functions, invariants, edge cases→guards)
- IR structural verification (bound variables, `old()` placement, quantifier types, postcondition connectivity)
- IR coherence analysis (invariant-action field coverage, verification obligations)
- CLI: `compile` (IR JSON output), `verify` (semantic + structural + coherence checks)

Phase 3 (in progress):
- Audit trace maps (spec items → IR constructs with source line numbers)
- Coverage summaries (entity/action/invariant/edge guard counts, verification status, obligations)
- CLI: `audit` (trace map view), `coverage` (summary view)
