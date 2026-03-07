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

### Phase 1: Intent Language MVP
- Parser and AST for the intent language
- Validation (type checking, constraint consistency)
- Markdown/HTML rendering of specs for human review
- CLI tool: `intent check`, `intent render`

### Phase 2: Agent IR Foundation
- Define the core IR type system
- Build IR generator from intent specs (scaffold generation)
- Basic verification: do postconditions follow from preconditions?
- CLI tool: `intent compile`, `intent verify`

### Phase 3: Audit Bridge
- Trace map generation
- Diff reporting
- Coverage analysis
- CLI tool: `intent audit`, `intent coverage`

### Phase 4: Agent Integration
- Agent-friendly API for reading specs and producing IR
- Incremental verification (re-verify only changed code)
- Multi-agent collaboration support (lock/claim spec sections)

---

## Open Questions

1. **How much of the IR should be human-inspectable?** Even if agents maintain it, debugging will sometimes require humans to look at IR. What's the right level of readability?

2. **How do we handle specs that are inherently ambiguous?** Natural language leaks in at the edges. Should the intent language support probabilistic or fuzzy constraints?

3. **What's the compilation target?** Does Agent IR compile to LLVM, WASM, or something else? Or is it interpreted?

4. **How do we bootstrap?** The first version needs to be written in an existing language. What's the best host language for the compiler/toolchain?

5. **Formal verification scope** — Full theorem proving (like Lean/Coq) is powerful but heavy. SMT-based checking (like Dafny) is more practical. Where on this spectrum do we land?

---

## Influences and Prior Art

- **Dafny** — Verification-aware language with pre/postconditions
- **TLA+** — Formal specification of concurrent systems
- **Alloy** — Lightweight formal modeling
- **Design by Contract (Eiffel)** — requires/ensures pattern
- **Rust's type system** — Encoding invariants in types
- **Lean 4** — Theorem proving meets general-purpose programming
- **JSON Schema / OpenAPI** — Machine-readable specs for APIs
