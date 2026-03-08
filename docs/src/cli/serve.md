# Serving Specs (Runtime API)

```bash
intent serve <file>
```

Starts a stateless HTTP server that exposes your spec's actions as REST endpoints. The runtime enforces preconditions, computes postconditions, and validates invariants on every request.

## Options

| Flag | Description |
|------|-------------|
| `--addr <HOST:PORT>` | Bind address (default: `127.0.0.1:3000`) |

## Example

```bash
$ intent serve examples/transfer.intent
intent serve: listening on http://127.0.0.1:3000
  module: TransferFunds
  POST /actions/Transfer
  GET  /
```

## Endpoints

### `GET /` — Module info

Returns the module's entities, actions, and invariants:

```json
{
  "name": "TransferFunds",
  "entities": [
    {
      "name": "Account",
      "fields": [
        { "name": "id", "type": "Named(\"UUID\")" },
        { "name": "balance", "type": "Decimal(2)" },
        { "name": "status", "type": "Union([\"Active\", \"Suspended\"])" }
      ]
    }
  ],
  "actions": [
    {
      "name": "Transfer",
      "params": [
        { "name": "from", "type": "Struct(\"Account\")" },
        { "name": "to", "type": "Struct(\"Account\")" },
        { "name": "amount", "type": "Decimal(2)" }
      ],
      "precondition_count": 4,
      "postcondition_count": 2
    }
  ],
  "invariants": ["BalanceNonNegative"]
}
```

### `POST /actions/<name>` — Execute an action

Executes the named action against caller-provided state. The server is **stateless** — you provide all entity state in each request and receive the new state in the response.

#### Request format

```json
{
  "params": {
    "from": { "id": "1", "balance": 1000.0, "status": "Active" },
    "to": { "id": "2", "balance": 500.0, "status": "Active" },
    "amount": 200.0
  },
  "state": {
    "Account": [
      { "id": "1", "balance": 1000.0, "status": "Active" },
      { "id": "2", "balance": 500.0, "status": "Active" }
    ]
  }
}
```

| Field | Description |
|-------|-------------|
| `params` | Action parameters by name. Primitive values or objects for entity-typed params. |
| `state` | Entity instances by type name. Used for `forall`/`exists` evaluation and invariant checking. |

#### Success response (200 OK)

```json
{
  "ok": true,
  "new_params": {
    "from": { "id": "1", "balance": 800.0, "status": "Active" },
    "to": { "id": "2", "balance": 700.0, "status": "Active" },
    "amount": 200.0
  },
  "violations": []
}
```

The `new_params` object reflects state after postconditions have been applied. Postconditions with the pattern `param.field == old(param.field) +/- expr` are extracted as state assignments.

#### Violation response (422 Unprocessable Entity)

When a contract is violated, the server returns `422` with details:

```json
{
  "ok": false,
  "new_params": {
    "from": { "id": "1", "balance": 50.0, "status": "Active" },
    "to": { "id": "2", "balance": 500.0, "status": "Active" },
    "amount": 200.0
  },
  "violations": [
    {
      "kind": "precondition_failed",
      "message": "precondition failed: from.balance >= amount"
    }
  ]
}
```

Violation kinds:

| Kind | Meaning |
|------|---------|
| `precondition_failed` | A `requires` condition was false |
| `postcondition_failed` | An `ensures` condition was false after state transformation |
| `invariant_violated` | A module invariant was false after the action |
| `edge_guard_triggered` | An edge case guard matched |

## Execution pipeline

Each request goes through four stages in order:

1. **Preconditions** — Evaluate `requires` expressions. If any fail, return violations immediately (no state change).
2. **Edge guards** — Evaluate `edge_cases` guards. If any match, return violations immediately.
3. **State transformation** — Extract assignments from `ensures` clauses (patterns like `param.field == old(param.field) - amount`) and apply them to produce `new_params`.
4. **Postcondition + invariant validation** — Evaluate all `ensures` and `invariant` expressions against the new state. Collect any violations.

## How `old()` works at runtime

In postconditions, `old(expr)` evaluates `expr` against the pre-action parameter values. For example, given:

```intent
ensures {
  from.balance == old(from.balance) - amount
}
```

If `from.balance` was `1000.0` before the action and `amount` is `200.0`, then `old(from.balance)` evaluates to `1000.0`, and the runtime verifies that `from.balance` in the new state equals `800.0`.

## Status codes

| Code | Meaning |
|------|---------|
| `200` | Action executed, all contracts satisfied |
| `400` | Malformed request (invalid JSON, missing fields, runtime error) |
| `404` | Unknown endpoint or action name |
| `422` | Action executed but contracts were violated |

## cURL example

```bash
curl -X POST http://127.0.0.1:3000/actions/Transfer \
  -H "Content-Type: application/json" \
  -d '{
    "params": {
      "from": {"id": "1", "balance": 1000.0, "status": "Active"},
      "to": {"id": "2", "balance": 500.0, "status": "Active"},
      "amount": 200.0
    },
    "state": {
      "Account": [
        {"id": "1", "balance": 1000.0, "status": "Active"},
        {"id": "2", "balance": 500.0, "status": "Active"}
      ]
    }
  }'
```

## Notes

- The server is **single-threaded and blocking** (via `tiny_http`). It's designed for development, testing, and demos — not production traffic.
- **Stateless**: the server stores nothing between requests. The caller owns all state.
- Union types are represented as strings (e.g., `"Active"`, not `{"variant": "Active"}`).
- Multi-file specs with `use` imports are resolved at startup before serving.
