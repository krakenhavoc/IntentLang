# Four-Layer Design

IntentLang is built around four distinct layers. Layers 0 and 1 are both human-facing — the system meets users at their comfort level.

## Layer 0: Natural Language (human-facing)

A natural language interface where humans describe what they want in plain English. An AI agent translates the description into a formal `.intent` spec.

**Design goals:**
- Lowest-barrier entry point — anyone can describe an idea
- Automated validation: generated specs are checked before output
- Confidence levels control how much the agent asks vs. assumes
- Edit mode for modifying existing specs from natural language
- Prompt preservation: original ask stored alongside the spec in version control

## Layer 1: Intent Layer (human-facing)

A declarative specification language where humans express *what* they want and *what constraints must hold*, without prescribing implementation. Humans can author specs directly here, or refine specs generated from Layer 0.

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

Humans describe the **idea**. The system formalizes it into a **contract**. Agents write the **implementation**. The toolchain **proves** the implementation satisfies the contract.
