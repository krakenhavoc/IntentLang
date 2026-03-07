# 08 - Real-World: E-Commerce

A multi-module e-commerce system with catalog management, order fulfillment, and payment processing.

## Modules

| File | Domain | Key Concepts |
|------|--------|-------------|
| `catalog.intent` | Product catalog | Categories, pricing tiers, reviews, volume discounts |
| `orders.intent` | Order lifecycle | Cart-to-order, shipping, delivery, returns |
| `payments.intent` | Payment processing | Authorization, capture, refunds, reconciliation |

## What Makes This Real-World

- **Multi-module design** — Each module owns its domain. In production, separate teams/agents would own each module.
- **Complex state machines** — Orders flow through `Pending -> Confirmed -> Shipped -> Delivered`, with branches for cancellation and returns.
- **Cross-domain invariants** — Refund amounts can't exceed payment amounts, only captured payments can be refunded.
- **Idempotency keys** — Payments use explicit idempotency keys to prevent double-charging.
- **Role-based access** — Certain actions require specific roles (`catalog_manager`, `customer_service`, `finance`).
- **Audit logging** — Every state-changing action is audit-logged for compliance.

## Try It

```bash
# Check all three modules
intent check catalog.intent
intent check orders.intent
intent check payments.intent

# Full verification pipeline
intent verify catalog.intent
intent verify orders.intent
intent verify payments.intent

# Audit and coverage
intent audit payments.intent
intent coverage orders.intent

# Query specific items
intent query catalog.intent TierBelowBase
intent query payments.intent obligations

# Multi-agent: different agents own different modules
intent lock catalog.intent ListProduct --agent catalog-agent
intent lock payments.intent AuthorizePayment --agent payment-agent
intent status catalog.intent
intent status payments.intent
```

## Design Patterns to Notice

1. **State machine encoding**: Union types model states (`Pending | Confirmed | Shipped | ...`), and `requires` blocks enforce valid transitions.
2. **Existence as creation**: `ensures { exists r: Refund => ... }` specifies that a new record must exist after the action — the spec doesn't say how to create it.
3. **Negative existence as uniqueness**: `!(exists r: Review => r.product == product && r.author_id == author_id)` enforces one review per customer per product.
4. **Layered invariants**: Each module has its own invariants, but they collectively ensure system correctness.

## Pre-compiled IR

Each module has a pre-compiled IR file (`catalog.ir.json`, `orders.ir.json`, `payments.ir.json`). Regenerate them with:

```bash
intent compile catalog.intent > catalog.ir.json
intent compile orders.intent > orders.ir.json
intent compile payments.intent > payments.ir.json
```
