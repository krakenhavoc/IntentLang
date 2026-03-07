# IntentLang — Design Specification

## Vision

A four-layer programming system designed for a world where humans and AI agents collaborate on software:

0. **Natural Language** (human-facing) — A natural language interface where humans describe what they want in plain English. An AI agent translates the description into a formal intent spec. This is the lowest-barrier entry point — the language meets humans where they are.
1. **Intent Layer** (human-facing) — A declarative specification language where humans express *what* they want and *what constraints must hold*, without prescribing implementation. Humans can author specs directly here, or refine specs generated from Layer 0.
2. **Agent IR** (agent-facing) — A dense, formally verifiable intermediate representation that agents generate, optimize, and maintain.
3. **Audit Bridge** — Tooling that maps between the two layers so humans can review, approve, and understand agent-produced code at the specification level.

Layers 0 and 1 are both human-facing — they meet users at their comfort level. A PM can describe an idea in plain English (Layer 0) and get a formal spec. An engineer can write or refine that spec directly (Layer 1). Either way, the human controls the *what*; agents handle the *how*.

The analogy: Humans describe the *idea*. The system formalizes it into a *contract*. Agents write the *implementation*. The toolchain *proves* the implementation satisfies the contract.

> **Traceability note:** The natural language prompt used to generate a spec should be stored alongside the `.intent` file in version control (e.g., as a `--- @prompt` annotation or companion `.prompt` file). This preserves the original human intent so that other team members can understand not just *what* was specified, but *why* — the ask behind the spec.

---

## Core Principles

### 1. Specification is the Source of Truth
The intent layer is the canonical artifact. Agent IR is derived from it and must provably conform to it. If the spec changes, the implementation must re-verify. If the implementation drifts, it's a bug — not a feature.

### 2. Correctness Over Readability (at the IR level)
Agent IR does not need to be human-scannable. It needs to be machine-verifiable, compact, and unambiguous. Think of it like a typed AST with embedded proofs, not like Python.

### 3. Readability Over Completeness (at the Intent level)
The intent layer should feel closer to a design doc than a program. A product manager should be able to read it. A senior engineer should be able to review it. It should be possible to onboard someone by reading the intent specs alone.

### 4. Auditable by Default
Every agent-produced construct must trace back to a spec requirement. Orphan code (implementation with no spec justification) is a first-class error.

### 5. Composable Contracts
Intent specs should compose — you can import, extend, and constrain other specs. This mirrors how real systems are built from smaller, well-defined components.

---

## Layer 0: Natural Language

### Purpose
A natural language interface that translates human descriptions into formal `.intent` specs. Layer 0 lowers the barrier to entry — anyone who can describe what they want in plain English can produce a verified specification.

### How It Works
1. Human provides a natural language description (e.g., *"I want a program that greets a user by name"*)
2. An AI agent translates the description into IntentLang syntax
3. The generated spec is automatically validated via `intent check`
4. If validation fails, the agent self-corrects and retries
5. The validated `.intent` file is output for human review

### CLI Interface
```
intent generate "I want a task tracker with priorities and assignments"
intent generate --interactive "build me a shopping cart"
intent generate --edit cart.intent "add a discount code feature"
intent generate --confidence 2 "healthcare patient records system"
intent generate --model claude-sonnet-4-6 --out hello.intent "greeting service"
```

### Confidence Levels
The `--confidence` flag (1–5) controls how much the agent asks vs. assumes:

| Level | Behavior |
|-------|----------|
| 1 | Always start interactive — ask clarifying questions before generating |
| 2 | Generate a draft, then ask "does this look right?" before finalizing |
| 3 (default) | Generate and auto-validate. Switch to interactive only if `check` finds errors after retry |
| 4 | Generate, auto-validate, auto-retry errors. Only prompt if completely stuck |
| 5 | Single-shot. Output whatever the LLM returns (still runs `check`, but doesn't retry or prompt) |

### Edit Mode
`intent generate --edit <file> "description of changes"` reads an existing spec, applies the requested modifications, and outputs the updated file. Use `--diff` to see a diff instead of the full file.

### Prompt Preservation
The natural language prompt used to generate a spec should be committed to version control alongside the `.intent` file. This preserves the original ask so that team members can understand the intent behind the spec. Storage format TBD (companion `.prompt` file or `--- @prompt` annotation in the spec itself).

### Configuration
- `AI_API_KEY` — API key (env var)
- `AI_API_BASE` — API base URL (env var, defaults to OpenAI-compatible endpoint)
- `AI_MODEL` — Default model (env var, overridden by `--model`)
- `--max-retries N` — Max validation retry attempts (default: 2, so 3 total attempts)

The API client uses the OpenAI-compatible chat completions format, supporting any provider (OpenAI, Anthropic, local models via Ollama/vLLM, Azure, etc.) through configurable base URL and model.

---

## Layer 1: Intent Language

### Purpose
A declarative specification language where humans describe system behavior, constraints, and invariants.

### Design Goals
- Readable by non-engineers (PMs, designers, stakeholders)
- Formally parseable and machine-interpretable
- Supports behavioral specs, not just type signatures
- Versioned and diffable

### Preliminary Syntax Sketch

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
  when from == to => reject("Cannot transfer to same account")
  when amount > system.max_transfer => require_approval(level: "manager")
}
```

### Key Features
- **`requires` / `ensures`** — Pre/postcondition blocks (Design by Contract)
- **`properties`** — Declarative behavioral annotations (idempotency, atomicity, etc.)
- **`invariant`** — System-wide constraints that must always hold
- **`edge_cases`** — Explicit handling of boundary conditions
- **`old()` references** — Refer to pre-execution state in postconditions
- **Natural language descriptions** via `---` doc blocks

---

## Layer 2: Agent IR (Intermediate Representation)

### Purpose
A dense, typed, formally verifiable representation that agents generate from intent specs.

### Design Goals
- Optimized for machine generation, not human authoring
- Embeds proof obligations alongside logic
- Supports incremental verification (change one function, re-verify only what's affected)
- Serializable (agents can pass IR between contexts)

### Characteristics (not final syntax)
- Typed lambda calculus foundation with dependent types
- Effects system (IO, State, Network, Time are tracked in types)
- Proof annotations inline with executable code
- Deterministic evaluation order (no undefined behavior)
- All functions total or explicitly marked partial with termination arguments

### Example Direction

```ir
// This is illustrative, not final syntax
fn transfer(
  from: Account {status: Active},
  to: Account {status: Active, currency: from.currency},
  amount: Decimal {value: > 0, value: <= from.balance}
) -> Result<(Account, Account), TransferError>
  @proves NoNegativeBalances
  @traces TransferFunds.Transfer
  @property idempotent(key: (from.id, to.id, amount, request_id))
  @property atomic
{
  let from' = from with { balance: from.balance - amount }
  let to' = to with { balance: to.balance + amount }
  // proof: from'.balance >= 0 by (from.balance >= amount) ∧ (amount > 0)
  // proof: to'.balance >= 0 by (to.balance >= 0) ∧ (amount > 0)
  Ok((from', to'))
}
```

---

## Layer 3: Audit Bridge

### Purpose
Tooling and formats that let humans verify agent work at the intent level.

### Components

#### Trace Map
Every IR construct links back to a spec requirement. The audit bridge can generate a coverage report:
```
TransferFunds.Transfer
  ├── requires.from_active    → transfer:line3 (type constraint)
  ├── requires.balance_check  → transfer:line5 (refinement type)
  ├── ensures.from_debited    → transfer:line8 (proof annotation)
  ├── ensures.to_credited     → transfer:line9 (proof annotation)
  ├── property.idempotent     → transfer:@property (annotation)
  └── invariant.NoNegativeBalances → transfer:@proves (proof)
```

#### Diff Reports
When agents modify IR, the bridge generates human-readable diffs at the intent level:
```
Change: Added rate limiting to Transfer action
Spec impact: None (implementation-only optimization)
Invariants: All still hold ✓
New behavior: Transfers exceeding 100/min per account are queued
```

#### Verification Dashboard
- Which specs have verified implementations
- Which invariants are proven vs. assumed
- Coverage gaps (specs without implementation)
- Orphan code (implementation without spec justification)

---

## Development Roadmap

### Phase 1: Intent Language MVP (complete)
- Parser and AST for the intent language
- Validation (type checking, constraint consistency)
- Markdown/HTML rendering of specs for human review
- CLI tool: `intent check`, `intent render`

### Phase 2: Agent IR Foundation (complete)
- Define the core IR type system
- Build IR generator from intent specs (scaffold generation)
- Basic verification: do postconditions follow from preconditions?
- CLI tool: `intent compile`, `intent verify`

### Phase 3: Audit Bridge (complete)
- Trace map generation
- Diff reporting
- Coverage analysis
- CLI tool: `intent audit`, `intent coverage`

### Phase 4: Agent Integration (complete)
- Agent-friendly API for reading specs and producing IR
- Incremental verification (re-verify only changed code)
- Multi-agent collaboration support (lock/claim spec sections)

### Phase 5: Language Polish & Natural Language Generation (in progress)
- Auto-formatter (`intent fmt`), scaffolding (`intent init`), shell completions
- Natural language to intent translation (`intent generate`)
- Interactive and single-shot generation modes with confidence levels
- Edit existing specs from natural language descriptions
- Model-agnostic LLM integration (OpenAI-compatible API)

### Phase 6: Stateless Runtime
- **Expression evaluator** — evaluate `requires`/`ensures`/`invariant` expressions against concrete values
- **State transformer** — apply `ensures` postconditions to compute new state from old
- **HTTP server** — auto-generate REST endpoints from actions (`intent serve <file>`)
- **Stateless execution model** — caller provides entity state in the request, runtime evaluates contracts, returns new state or constraint violations
- New crate: `intent-runtime`
- CLI tool: `intent serve`

### Phase 7: Module Imports
- `use OtherModule.Entity` syntax for cross-file composition
- Module resolver — loads and parses imported modules
- Cross-module type checking
- `intent serve` loads multiple modules together

### Long-Term: Self-Hosting
IntentLang compiles itself. The compiler's spec is written in `.intent` files, agents generate the implementation, and the audit bridge verifies conformance. Not a near-term priority, but a planned goal. See the [self-hosting roadmap](../CLAUDE.md) for stages and invariants.

### Milestone Definitions

| Milestone | Meaning |
|-----------|---------|
| **Alpha** | Core features working, API unstable, may have missing pieces |
| **Beta** | A small real-world system runs end-to-end. Module imports working. API stabilizing |
| **Preview** | Post-feedback hardening. Between beta and production (if needed) |
| **Stable (v1.0)** | Production-ready runtime, stable API |

Version numbers are not hardlocked to phases — minor and patch versions (e.g. v0.5.1-alpha) may ship between milestones as incremental progress.

### Future Considerations

Features under consideration for post-beta phases, grouped by theme.

#### Language Expressiveness
- **Parameterized entities / generics** — `entity Queue<T> { ... }`
- **Computed fields** — `derived total: sum(items.price * items.quantity)`
- **Temporal operators** — `eventually`, `always`, `until` for richer invariants
- **Enum values with data** — `status: Active(since: DateTime) | Frozen(reason: String)`

#### Verification & Correctness
- **SMT solver integration** — move beyond structural checking to actual constraint solving (Z3/CVC5)
- **Counterexample generation** — when verification fails, produce a concrete violating state
- **Property-based test generation** — derive tests from specs (fuzzing action sequences against invariants)
- **Refinement types** — `amount: Decimal { > 0, <= 100000 }` with solver-backed checking

#### Developer Experience
- **LSP server** — syntax highlighting, diagnostics, go-to-definition, completions in editors
- **WASM build** — run `intent check` in the browser, enable a playground site
- **Watch mode** — `intent watch <file>` re-checks on save
- **Explain command** — `intent explain <file> <item>` gives natural language explanation of a spec item
- **Test command** — `intent test <file>` generates and runs property-based tests from specs

#### Ecosystem & Integration
- **Code generation** — `intent codegen <file> --lang rust/typescript/python` generates skeleton implementations
- **OpenAPI bridge** — `intent import openapi.yaml` generates specs from existing API definitions
- **GitHub Action** — `intent check` as a CI step for `.intent` files in any repo
- **VS Code extension** — syntax highlighting + integrated diagnostics (lighter than full LSP)

---

## Open Questions

1. **How much of the IR should be human-inspectable?** Even if agents maintain it, debugging will sometimes require humans to look at IR. What's the right level of readability?

2. **How do we handle specs that are inherently ambiguous?** Natural language leaks in at the edges. Should the intent language support probabilistic or fuzzy constraints?

3. **Formal verification scope** — Full theorem proving (like Lean/Coq) is powerful but heavy. SMT-based checking (like Dafny) is more practical. Where on this spectrum do we land?

4. **Runtime execution boundaries** — How much logic should the stateless runtime infer from `ensures` blocks? Pure contract enforcement (check but don't compute) vs. state derivation (compute new state from postconditions)?

5. **Module import resolution** — file-system based (relative paths) or registry-based (namespaced packages)? Or both?

---

## Influences and Prior Art

- **Dafny** — Verification-aware language with pre/postconditions
- **TLA+** — Formal specification of concurrent systems
- **Alloy** — Lightweight formal modeling
- **Design by Contract (Eiffel)** — requires/ensures pattern
- **Rust's type system** — Encoding invariants in types
- **Lean 4** — Theorem proving meets general-purpose programming
- **JSON Schema / OpenAPI** — Machine-readable specs for APIs
