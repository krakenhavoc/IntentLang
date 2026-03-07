# Invariants

Invariants define system-wide constraints that must always hold, regardless of which action executes:

```intent
invariant NoNegativeBalances {
  --- No account may ever have a negative balance.
  forall a: Account => a.balance >= 0
}
```

## Quantifiers

Invariants typically use quantifiers to express universal or existential properties:

```intent
// Universal: must hold for all instances
invariant StockNonNegative {
  forall p: Product => p.stock >= 0
}

// Temporal: constrains action effects
invariant TransferConservation {
  forall t: Transfer =>
    old(t.from.balance) + old(t.to.balance) ==
    t.from.balance + t.to.balance
}
```

When an invariant quantifies over an action type (like `forall t: Transfer`), it becomes a *temporal invariant* — a constraint on the state transition that the action produces. `old()` is valid in these contexts.

## Verification obligations

The IR verifier automatically detects *coherence obligations* — situations where an action modifies fields that an invariant constrains. For example, if `Transfer` modifies `balance` and `NoNegativeBalances` constrains `balance`, the verifier flags this as an obligation that the implementation must satisfy.

```bash
intent verify examples/transfer.intent
```

The output includes a section listing these obligations, connecting each invariant to the actions that might affect it.
