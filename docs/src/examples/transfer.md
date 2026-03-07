# Fund Transfer

A fund transfer system between accounts within the same currency. Demonstrates entities with union-typed status fields, actions with pre/postconditions, conservation invariants, and edge cases.

**File:** [`examples/transfer.intent`](https://github.com/krakenhavoc/IntentLang/blob/main/examples/transfer.intent)

```intent
module TransferFunds

--- A fund transfer system between accounts within the same currency.
--- Supports basic account-to-account transfers with balance validation,
--- idempotency guarantees, and manager approval for large amounts.

entity Account {
  id: UUID
  owner: String
  balance: Decimal(precision: 2)
  currency: CurrencyCode
  status: Active | Frozen | Closed
  created_at: DateTime
}

entity TransferRecord {
  id: UUID
  request_id: UUID
  from_account: Account
  to_account: Account
  amount: Decimal(precision: 2)
  status: Pending | Completed | Failed | Reversed
  created_at: DateTime
  completed_at: DateTime?
}

action Transfer {
  --- Move funds from one account to another.
  from: Account
  to: Account
  amount: Decimal(precision: 2)
  request_id: UUID

  requires {
    from.status == Active
    to.status == Active
    from.currency == to.currency
    amount > 0
    from.balance >= amount
    from.id != to.id
  }

  ensures {
    from.balance == old(from.balance) - amount
    to.balance == old(to.balance) + amount
    exists r: TransferRecord =>
      r.request_id == request_id &&
      r.status == Completed
  }

  properties {
    idempotent: true
    idempotency_key: request_id
    atomic: true
    audit_logged: true
    max_latency_ms: 500
  }
}

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

  properties {
    audit_logged: true
    requires_role: "compliance_officer"
  }
}

invariant NoNegativeBalances {
  --- No account may ever have a negative balance.
  forall a: Account => a.balance >= 0
}

invariant TransferConservation {
  --- Transfers must not create or destroy money.
  forall t: Transfer =>
    old(t.from.balance) + old(t.to.balance) ==
    t.from.balance + t.to.balance
}

edge_cases {
  when amount > 10000.00 => require_approval(level: "manager")
  when from.owner == to.owner => allow(note: "Self-transfer between own accounts")
  when from.currency != to.currency => reject("Cross-currency transfers not supported.")
}
```

## Key concepts demonstrated

- **Union types** for entity status: `Active | Frozen | Closed`
- **`old()` in ensures** for state transition assertions
- **Conservation invariant** ensuring transfers don't create or destroy money
- **Properties** for runtime metadata (idempotency, atomicity, latency SLAs)
- **Edge cases** for approval workflows and error conditions
