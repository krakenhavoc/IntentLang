# Three-Layer Design

IntentLang is built around three distinct layers, each serving a different audience:

## Layer 1: Intent Layer (human-facing)

A declarative specification language where humans express *what* they want and *what constraints must hold*, without prescribing implementation.

**Design goals:**
- Readable by non-engineers (PMs, designers, stakeholders)
- Formally parseable and machine-interpretable
- Supports behavioral specs, not just type signatures
- Versioned and diffable

## Layer 2: Agent IR (agent-facing)

A dense, formally verifiable intermediate representation that agents generate, optimize, and maintain.

**Design goals:**
- Machine-verifiable, compact, and unambiguous
- Not designed for human authoring — think typed AST with embedded proofs
- Every node carries source traces back to the spec layer
- Supports structural verification and coherence analysis

## Layer 3: Audit Bridge

Tooling that maps between the two layers so humans can review, approve, and understand agent-produced code at the specification level.

**Key properties:**
- Every IR construct traces to a spec requirement
- Orphan code (implementation without spec justification) is a first-class error
- Coverage analysis shows which spec items have IR backing

## The analogy

Humans write the **contract**. Agents write the **implementation**. The system **proves** the implementation satisfies the contract.
