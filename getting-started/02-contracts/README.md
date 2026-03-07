# 02 - Contracts

Pre/postconditions and the `old()` operator for tracking state changes.

## Concepts Introduced

- `old(expr)` — References the value of an expression *before* the action executed
- `properties` — Declarative behavioral annotations (e.g., `idempotent: true`)
- Multiple `requires` conditions — all must hold for the action to proceed
- Arithmetic in contracts — `counter.value + step <= counter.max_value`

## Try It

```bash
# Validate the spec
intent check counter.intent

# Compile to IR — notice how old() becomes temporal references
intent compile counter.intent

# Verify structural correctness
intent verify counter.intent
```

## Key Insight

The `old()` operator is what makes IntentLang contracts powerful. It lets you express *relationships between before and after states*:

```intent
ensures {
  counter.value == old(counter.value) + step
}
```

This says: "after Increment runs, the counter's value must be exactly `step` more than it was before." This is a precise, verifiable contract — not just a comment or a hope.

## Compiled IR

The file `counter.ir.json` contains the pre-compiled Agent IR. Regenerate it with:

```bash
intent compile counter.intent > counter.ir.json
```

Notice how `old()` expressions in the spec become temporal references in the IR postconditions — the IR preserves the contract semantics in a machine-verifiable form.
