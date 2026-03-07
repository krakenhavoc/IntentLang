# Actions

Actions define operations with parameters, preconditions, postconditions, and properties:

```intent
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
```

## Parameters

Action parameters are listed at the top of the block, before any `requires`, `ensures`, or `properties` sections. Each parameter has a name and type.

## Requires (preconditions)

The `requires` block lists conditions that must be true *before* the action executes. Each line is a boolean expression. If any precondition is false, the action must not execute.

`old()` **cannot** be used in `requires` blocks — preconditions describe the current state, not a state transition.

## Ensures (postconditions)

The `ensures` block lists conditions that must be true *after* the action executes. Use `old(expr)` to reference the value of an expression before the action ran:

```intent
ensures {
  from.balance == old(from.balance) - amount
  to.balance == old(to.balance) + amount
}
```

## Properties

The `properties` block contains key-value metadata about the action. Properties are not validated semantically — they're metadata for downstream tooling:

```intent
properties {
  idempotent: true
  idempotency_key: request_id
  atomic: true
  audit_logged: true
  max_latency_ms: 500
  requires_role: "compliance_officer"
}
```

## Documentation

Actions support inline documentation:

```intent
action FreezeAccount {
  --- Freeze an account, preventing all transfers.
  account: Account
  reason: String

  requires {
    account.status == Active
  }

  ensures {
    account.status == Frozen
  }
}
```
