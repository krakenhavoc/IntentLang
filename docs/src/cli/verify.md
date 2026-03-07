# Verification & Audit

## Verify

```bash
intent verify <file>
```

Runs structural verification and coherence analysis on the compiled IR:

- **Structural checks** — all type references resolve, field accesses are valid, `old()` appears only in valid contexts
- **Coherence analysis** — detects verification obligations: invariants that constrain fields modified by actions

The output lists any verification errors and the set of coherence obligations that an implementation must satisfy.

## Audit

```bash
intent audit <file>
```

Displays the audit trace map — a mapping from every spec-level construct to its IR representation, with source line references. This lets you verify that every spec requirement has a corresponding IR construct.

## Coverage

```bash
intent coverage <file>
```

Shows a coverage summary: counts of entities, actions, invariants, and edge cases, along with their verification status. Useful for quick assessment of spec completeness.
