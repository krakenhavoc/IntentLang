# IntentLang

IntentLang is a declarative specification language for human-AI collaboration.

Humans write **what** the system must do and **what constraints must hold**.
Agents handle **how** — generating verifiable implementations from specs.
The toolchain proves the implementation satisfies the contract.

## Why IntentLang?

As AI agents write more code, the bottleneck shifts from *writing* to *verifying*. IntentLang addresses this with four layers:

0. **Natural Language** — Describe what you want in plain English. An AI agent generates a formal spec from your description. The lowest-barrier entry point.
1. **Intent Layer** — Write or refine declarative specs directly: entities, actions, pre/postconditions, invariants. Readable by anyone on the team, formally parseable by machines.
2. **Agent IR** — Agents generate a dense, typed intermediate representation from specs. Optimized for machine generation, not human authoring.
3. **Audit Bridge** — Tooling maps every IR construct back to a spec requirement. Orphan code (implementation without spec justification) is a first-class error.

Layers 0 and 1 are both human-facing — the system meets you where you are.

## Quick Example

```intent
module TransferFunds

--- A fund transfer between two accounts within the same currency.

entity Account {
  id: UUID
  balance: Decimal(precision: 2)
  currency: CurrencyCode
  status: Active | Frozen | Closed
}

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
}

invariant NoNegativeBalances {
  forall a: Account => a.balance >= 0
}
```

```
$ intent check examples/transfer.intent
OK: TransferFunds — 7 top-level item(s), no issues found
```

## Prior Art

IntentLang draws on [Design by Contract](https://en.wikipedia.org/wiki/Design_by_contract) (requires/ensures), [Dafny](https://dafny.org/) (verification-aware programming), [TLA+](https://lamport.azurewebsites.net/tla/tla.html) (system-level invariants), and [Alloy](https://alloytools.org/) (lightweight formal modeling).
