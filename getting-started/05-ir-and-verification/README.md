# 05 - IR and Verification

Compiling intent specs to the Agent IR layer and running verification.

## Concepts Introduced

- **Agent IR** — The typed intermediate representation generated from specs
- **Source traces** — Every IR node links back to the spec element that produced it
- **Verification** — Structural checks on the compiled IR
- **Obligations** — Proof obligations the verifier identifies (invariant preservation, etc.)

## Try It

```bash
# Compile the spec to IR (outputs JSON)
intent compile task_tracker.intent

# Pipe to jq to explore the structure
intent compile task_tracker.intent | jq '.structs[].name'
intent compile task_tracker.intent | jq '.functions[0].preconditions'

# Verify the IR for structural correctness
intent verify task_tracker.intent

# Incremental verification (caches results for faster re-runs)
intent verify --incremental task_tracker.intent
# Run again — notice cached items aren't re-verified
intent verify --incremental task_tracker.intent
```

## What the IR Looks Like

When you compile a spec, each construct maps to IR:

| Intent Layer | IR Layer | Purpose |
|-------------|----------|---------|
| `entity` | `struct` | Data structure with typed fields |
| `action` | `function` | Operation with pre/postconditions |
| `requires` | `preconditions` | Input constraints on function |
| `ensures` | `postconditions` | Output guarantees from function |
| `invariant` | `invariant` | Module-wide assertion |
| `edge_cases` | `edge_guards` | Boundary condition checks |

Every IR node carries a `SourceTrace` with the module name, item name, part label, and byte span — this is the foundation for the Audit Bridge (next example).

## Verification Obligations

The verifier analyzes the IR and produces obligations like:

- **InvariantPreservation** — "Action X modifies field Y, which is constrained by invariant Z. Does X preserve Z?"
- **TemporalProperty** — "Invariant Z quantifies over action X. Is Z maintained across all executions of X?"

These obligations tell agents (or humans) what needs to be proven for the system to be correct.

## Pre-compiled IR

The file `task_tracker.ir.json` contains the pre-compiled IR for this spec. You can also explore it directly with `jq`, or regenerate it:

```bash
intent compile task_tracker.intent > task_tracker.ir.json
```
