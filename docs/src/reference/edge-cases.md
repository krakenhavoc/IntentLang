# Edge Cases

The `edge_cases` block defines boundary conditions as pattern-matched rules:

```intent
edge_cases {
  when amount > 10000.00 => require_approval(level: "manager")
  when from.owner == to.owner => allow(note: "Self-transfer between own accounts")
  when from.currency != to.currency => reject("Cross-currency transfers not supported.")
}
```

## Syntax

Each edge case rule follows the pattern:

```
when <condition> => <handler>
```

The condition is a boolean expression. The handler is a function call with optional named arguments.

## Common handlers

These are conventions — handlers are not validated by the checker, but are meaningful to downstream tooling:

| Handler | Purpose |
|---------|---------|
| `reject(message)` | Reject the operation with an error message |
| `require_approval(level: role)` | Require manual approval from a specific role |
| `allow(note: reason)` | Explicitly allow with documentation |

## Edge case validation

The checker validates that field references in edge case conditions refer to valid fields on known entity types. Undefined fields or entities produce errors with source spans.
