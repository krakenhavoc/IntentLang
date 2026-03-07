# Agent Tooling

Commands designed for agent integration: structured queries, spec diffing, JSON output, and multi-agent collaboration.

## JSON Output

Any command supports `--output json` for structured, machine-readable output:

```bash
intent --output json check examples/transfer.intent
intent --output json verify examples/transfer.intent
```

## Diff

```bash
intent diff <old-file> <new-file>
```

Shows a spec-level diff between two versions of a spec file. Reports added, removed, and modified spec items — useful for reviewing changes before committing or verifying that a refactor preserved all constraints.

## Query

```bash
intent query <file> <target>
```

Query specific items from a spec. Available targets:

| Target | Returns |
|--------|---------|
| `entities` | All entity definitions |
| `actions` | All action definitions |
| `invariants` | All invariants |
| `edge-cases` | All edge case rules |
| `obligations` | Verification obligations |
| `summary` | High-level spec summary |
| `<Name>` | A specific item by name (e.g., `Transfer`) |

Combine with `--output json` for structured output that agents can parse directly.

## Multi-Agent Collaboration

When multiple agents work on the same spec, the locking system prevents conflicts.

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
