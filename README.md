# IntentLang

[![CI](https://github.com/krakenhavoc/IntentLang/actions/workflows/ci.yml/badge.svg)](https://github.com/krakenhavoc/IntentLang/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/intent-cli.svg)](https://crates.io/crates/intent-cli)
[![docs](https://img.shields.io/badge/docs-mdBook-blue)](https://krakenhavoc.github.io/IntentLang/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A declarative specification language for human-AI collaboration.

Humans write **what** the system must do and **what constraints must hold**.
Agents handle **how** — generating verifiable implementations from specs.
The toolchain proves the implementation satisfies the contract.

## Why IntentLang?

As AI agents write more code, the bottleneck shifts from *writing* to *verifying*. IntentLang addresses this with four layers:

0. **Natural Language** — Describe what you want in plain English. An AI agent generates a formal spec from your description. The lowest-barrier entry point.
1. **Intent Layer** — Write or refine declarative specs directly: entities, actions, pre/postconditions, invariants. Readable by anyone on the team, formally parseable by machines.
2. **Agent IR** — Agents generate a dense, typed intermediate representation from specs. Optimized for machine generation, not human authoring.
3. **Audit Bridge** — Tooling maps every IR construct back to a spec requirement. Orphan code (implementation without spec justification) is a first-class error.

Layers 0 and 1 are both human-facing — the system meets you where you are. A PM can describe an idea in English; an engineer can write the spec directly. Either way, humans own the *what*.

## Example

```intent
module TransferFunds

--- A fund transfer between two accounts within the same currency.

entity Account {
  id: UUID
  balance: Decimal(precision: 2)
  currency: CurrencyCode
  status: Active | Frozen | Closed
}

action Transfer {
  from: Account
  to: Account
  amount: Decimal(precision: 2)

  requires {
    from.status == Active
    to.status == Active
    from.currency == to.currency
    amount > 0
    from.balance >= amount
  }

  ensures {
    from.balance == old(from.balance) - amount
    to.balance == old(to.balance) + amount
  }

  properties {
    idempotent: true
    atomic: true
    audit_logged: true
  }
}

invariant NoNegativeBalances {
  forall a: Account => a.balance >= 0
}

edge_cases {
  when amount > 10000.00 => require_approval(level: "manager")
  when from.currency != to.currency => reject("Cross-currency transfers not supported.")
}
```

```
$ intent check examples/transfer.intent
OK: TransferFunds — 7 top-level item(s), no issues found
```

See [`examples/`](examples/) for more: authentication, shopping cart, RBAC, API gateway, data pipeline. Full language documentation is at [krakenhavoc.github.io/IntentLang](https://krakenhavoc.github.io/IntentLang/).

## Getting Started

### Pre-built binary (Linux x86_64)

Download from the [latest release](https://github.com/krakenhavoc/IntentLang/releases):

```bash
chmod +x intent-linux-x86_64
./intent-linux-x86_64 check examples/transfer.intent
```

### Build from source

Requires [Rust](https://rustup.rs/) 1.70+.

```bash
git clone https://github.com/krakenhavoc/IntentLang.git
cd IntentLang
cargo build --release -p intent-cli
# Binary at target/release/intent
```

### Docker

```bash
docker build -t intent .
docker run -v $(pwd)/examples:/work intent check /work/transfer.intent
```

## CLI

```
intent check <file>                    Parse, type-check, and validate constraints
intent render <file>                   Render spec to Markdown
intent render-html <file>              Render spec to self-contained styled HTML
intent compile <file>                  Compile spec to IR (JSON output)
intent verify <file>                   Verify structural + logical correctness
intent verify --incremental <file>     Incremental verify (cache, re-verify only changes)
intent audit <file>                    Show audit trace map (spec → IR)
intent coverage <file>                 Show coverage summary
intent diff <old> <new>                Spec-level diff between two versions
intent query <file> <target>           Query items (for agent integration)
intent lock <file> <item> --agent X    Claim a spec item for an agent
intent unlock <file> <item> --agent X  Release a claimed spec item
intent status <file>                   Show lock status for all spec items
intent fmt <file>                      Format a spec file (--write to overwrite, --check to verify)
intent init                            Scaffold a new .intent file (--name, -o)
intent completions <shell>             Generate shell completions (bash, zsh, fish, etc.)
intent generate "description"          Generate a spec from natural language (Layer 0)
intent generate --interactive "desc"   Interactive mode with clarifying questions
intent generate --edit <file> "desc"   Modify an existing spec from natural language
intent serve <file>                    Serve spec as REST API (stateless runtime)
```

### Natural Language Generation (Layer 0)

`intent generate` translates plain English into validated `.intent` specs:

```bash
intent generate "I want a task tracker with priorities and assignments"
intent generate --confidence 2 "patient records system"
intent generate --edit cart.intent "add discount codes"
intent generate --model gpt-4o --out hello.intent "greeting service"
```

Configure via environment variables: `AI_API_KEY`, `AI_API_BASE` (endpoint URL), `AI_MODEL` (default model). Works with any OpenAI-compatible API provider.

### Semantic Analysis

`intent check` runs six passes:

1. Collect definitions, detect duplicates
2. Resolve type references (builtins + defined entities)
3. Validate quantifier binding types (`forall`/`exists`)
4. Validate edge case action references
5. Validate field access on entity-typed parameters
6. Constraint validation (`old()` placement, tautological comparisons)

Errors include source spans, labels, and actionable help via [miette](https://crates.io/crates/miette):

```
intent::check::undefined_type

  × undefined type `Customer`
   ╭─[5:13]
 4 │       id: UUID
 5 │ ╭─▶   customer: Customer
 6 │ ├─▶   items: List<LineItem>
   · ╰──── used here
 7 │     }
   ╰────
  help: define an entity named `Customer`, or use a built-in type
```

```
intent::check::old_in_requires

  × `old()` cannot be used in a `requires` block
    ╭─[13:21]
 12 │       requires {
 13 │ ╭─▶     from.balance == old(from.balance)
 14 │ ├─▶   }
    · ╰──── used here
    ╰────
  help: `old()` references pre-state values and is only meaningful in `ensures` blocks
```

### Rendering

`intent render` produces Markdown with entity field tables, action signatures, pre/postconditions, and edge case rules — suitable for sharing with non-technical stakeholders.

`intent render-html` produces a self-contained HTML document with color-coded sections. Redirect to a file and open in a browser:

```bash
intent render-html examples/transfer.intent > transfer.html
```

## Language Reference

### Constructs

| Construct | Purpose |
|-----------|---------|
| `module Name` | Required file header |
| `--- text` | Documentation block (multi-line) |
| `use Module` | Import all definitions from another module |
| `use Module.Item` | Import a specific item from another module |
| `entity Name { ... }` | Data structure with typed fields |
| `action Name { ... }` | Operation with params, pre/postconditions, properties |
| `invariant Name { ... }` | System-wide constraint (`forall`/`exists`) |
| `edge_cases { ... }` | Boundary conditions: `when cond => handler` |

### Type System

| Category | Examples |
|----------|---------|
| Primitives | `UUID`, `String`, `Int`, `Bool`, `DateTime` |
| Numeric | `Decimal(precision: N)` |
| Domain | `CurrencyCode`, `Email`, `URL` |
| Collections | `List<T>`, `Set<T>`, `Map<K, V>` |
| Optional | `T?` |
| Union | `Active \| Frozen \| Closed` |

### Expressions

| Category | Syntax |
|----------|--------|
| Comparison | `==` `!=` `>` `<` `>=` `<=` |
| Logical | `&&` `\|\|` `!` `=>` (implies) |
| Quantifiers | `forall x: Type => pred`, `exists x: Type => pred` |
| Pre-state | `old(expr)` — value before action execution |
| Field access | `entity.field`, `f(x).field` |

## Project Status

**Current release: [v0.5.0-alpha.1](https://github.com/krakenhavoc/IntentLang/releases/tag/v0.5.0-alpha.1)** — all phases complete, approaching beta.

| Phase | Status | What shipped |
|-------|--------|-------------|
| Phase 1 | Complete | PEG grammar, typed AST, six-pass semantic checker, Markdown/HTML renderers |
| Phase 2 | Complete | AST → Agent IR lowering, structural verification, coherence analysis |
| Phase 3 | Complete | Audit trace maps, coverage summaries, spec-level diffs |
| Phase 4 | Complete | Agent API, incremental verification, multi-agent collaboration |
| Phase 5 | Complete | Language polish (`fmt`, `init`, `completions`), NL generation (`intent generate`) |
| Phase 6 | Complete | Stateless runtime — `intent serve`, expression evaluator, REST API from specs |
| Phase 7 | Complete | Module imports (`use`), multi-file composition, cross-module type checking |

### Roadmap to v1.0

- **Alpha** (current) — all phases complete, full toolchain working
- **Beta** — a small real-world system runs end-to-end
- **Preview** — post-feedback hardening (if needed)
- **Stable (v1.0)** — production-ready runtime, stable API
- **Long-term** — self-hosting: IntentLang compiles itself (compiler spec in `.intent`, agents generate implementation)

188 tests across parser, checker, IR, runtime, and gen modules.

Long-term: IntentLang compiles itself. The compiler's spec is written in `.intent` files, agents generate the implementation, and the audit bridge verifies conformance. See the [self-hosting roadmap](CLAUDE.md) for details.

## Architecture

```
intent-cli ──→ intent-parser ←── grammar/intent.pest
    │              ↑
    ├──→ intent-check
    ├──→ intent-render
    ├──→ intent-ir (lowering, verification, audit)
    ├──→ intent-gen (NL → .intent, Layer 0)
    └──→ intent-runtime (stateless execution, HTTP server)
```

Seven crates in a Cargo workspace. The parser produces a typed AST and resolves module imports; the checker validates semantics (including cross-module type resolution); the renderer formats output; the IR crate lowers to a typed intermediate representation with verification, coherence analysis, and audit bridge; the gen crate translates natural language to `.intent` specs via LLM; the runtime crate provides a stateless HTTP server that executes specs natively. The CLI wires them together. See [`AGENTS.md`](AGENTS.md) for architecture details and [`docs/SPEC.md`](docs/SPEC.md) for the full language design.

## Examples

The [`examples/`](examples/) directory contains full working specs across different domains:

| Example | Domain |
|---------|--------|
| [`transfer.intent`](examples/transfer.intent) | Fund transfers with balance invariants |
| [`auth.intent`](examples/auth.intent) | Authentication with brute-force protection |
| [`shopping_cart.intent`](examples/shopping_cart.intent) | Shopping cart with inventory rules |
| [`rbac.intent`](examples/rbac.intent) | Role-based access control with hierarchical permissions |
| [`api_gateway.intent`](examples/api_gateway.intent) | API gateway with rate limiting and client tiers |
| [`data_pipeline.intent`](examples/data_pipeline.intent) | Staged data pipeline with retries and dead-letter queue |
| [`multi_module/`](examples/multi_module/) | Multi-file project with shared types and imports |

## Prior Art

IntentLang draws on [Design by Contract](https://en.wikipedia.org/wiki/Design_by_contract) (requires/ensures), [Dafny](https://dafny.org/) (verification-aware programming), [TLA+](https://lamport.azurewebsites.net/tla/tla.html) (system-level invariants), and [Alloy](https://alloytools.org/) (lightweight formal modeling).

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for development setup, workflow, and conventions. Questions and ideas welcome in [GitHub Discussions](https://github.com/krakenhavoc/IntentLang/discussions).

## License

[MIT](LICENSE)
