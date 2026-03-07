# Getting Started with IntentLang

This directory contains progressive examples covering all three layers of IntentLang:

1. **Intent Layer** — Declarative specifications (what and what constraints)
2. **Agent IR** — Typed intermediate representation (compiled from specs)
3. **Audit Bridge** — Trace maps, coverage analysis, and verification

## Prerequisites

Install the IntentLang CLI:

```bash
cargo install intent-cli
```

Or build from source:

```bash
git clone https://github.com/krakenhavoc/IntentLang.git
cd IntentLang
cargo build --release
# Binary at target/release/intent
```

## Examples by Difficulty

### Beginner — Intent Layer Basics

| # | Directory | What You'll Learn |
|---|-----------|-------------------|
| 01 | [hello-intent](01-hello-intent/) | Your first spec: one entity, one action |
| 02 | [contracts](02-contracts/) | Pre/postconditions with `requires`, `ensures`, `old()` |
| 03 | [types-and-collections](03-types-and-collections/) | `List<T>`, `Set<T>`, `Map<K,V>`, optionals, unions |
| 04 | [invariants-and-edges](04-invariants-and-edges/) | System invariants, `forall`/`exists`, `edge_cases` |

### Intermediate — IR and Verification

| # | Directory | What You'll Learn |
|---|-----------|-------------------|
| 05 | [ir-and-verification](05-ir-and-verification/) | Compiling specs to IR, running `intent verify` |
| 06 | [audit-and-coverage](06-audit-and-coverage/) | Trace maps, coverage reports, querying specs |
| 07 | [multi-agent](07-multi-agent/) | Lock/unlock, agent collaboration workflows |

### Advanced — Real-World Systems

| # | Directory | What You'll Learn |
|---|-----------|-------------------|
| 08 | [real-world-ecommerce](08-real-world-ecommerce/) | Multi-module e-commerce: catalog, orders, payments |
| 09 | [real-world-healthcare](09-real-world-healthcare/) | Regulatory compliance: patient records, appointments, audit trails |

## Quick Start

```bash
# Check a spec for errors
intent check getting-started/01-hello-intent/hello.intent

# Render as Markdown
intent render getting-started/01-hello-intent/hello.intent

# Compile to IR
intent compile getting-started/01-hello-intent/hello.intent

# Full verification
intent verify getting-started/01-hello-intent/hello.intent

# Audit trace map
intent audit getting-started/04-invariants-and-edges/voting.intent

# Coverage report
intent coverage getting-started/04-invariants-and-edges/voting.intent
```

## How to Use These Examples

Each subdirectory has:
- `.intent` files you can run through the toolchain
- A `README.md` explaining the concepts and CLI commands to try

Start with `01-hello-intent` and work your way up. Each example builds on concepts from the previous ones.
