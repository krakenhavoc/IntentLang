# Verification & Audit

## Verify

```bash
intent verify <file>
```

Runs structural verification and coherence analysis on the compiled IR:

- **Structural checks** — all type references resolve, field accesses are valid, `old()` appears only in valid contexts
- **Coherence analysis** — detects verification obligations: invariants that constrain fields modified by actions

The output lists any verification errors and the set of coherence obligations that an implementation must satisfy.

### Incremental Verification

```bash
intent verify --incremental <file>
```

Caches per-item verification results in `.intent-cache/`. On subsequent runs, only re-verifies items whose content has changed. Reports how many items were re-verified vs cached.

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

## Diff

```bash
intent diff <old-file> <new-file>
```

Shows a spec-level diff between two versions of a spec file. Reports added, removed, and modified spec items.

## Query

```bash
intent query <file> <target>
```

Query specific items from a spec. Targets: `entities`, `actions`, `invariants`, `edge-cases`, `obligations`, `summary`, or a specific item name (e.g., `Transfer`). Designed for agent integration — combine with `--output json` for structured output.

## Multi-Agent Collaboration

### Lock

```bash
intent lock <file> <item> --agent <agent-id>
```

Claims a spec item for an agent. Other agents cannot claim the same item until it is unlocked. Lock state is stored in `.intent-lock/`.

### Unlock

```bash
intent unlock <file> <item> --agent <agent-id>
```

Releases a claimed spec item. Only the agent that locked it can unlock it.

### Status

```bash
intent status <file>
```

Shows lock status for all spec items — which items are claimed, by whom, and which are available.
