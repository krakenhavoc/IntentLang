# 03 - Types and Collections

IntentLang's full type system: collections, optionals, unions, and domain types.

## Concepts Introduced

### Primitive and Domain Types
- `UUID`, `String`, `Int`, `Bool`, `DateTime` — built-in primitives
- `Decimal(precision: N)` — fixed-precision decimals
- `URL`, `Email`, `CurrencyCode` — domain-specific types

### Collections
- `List<T>` — ordered collection (e.g., `List<String>`)
- `Set<T>` — unique elements (e.g., `Set<String>`)
- `Map<K, V>` — key-value pairs

### Optionals
- `T?` — a value that may or may not be present (e.g., `DateTime?`)

### Union Types
- `A | B | C` — enum-like variants (e.g., `Open | Closed | UnderMaintenance`)

## Try It

```bash
intent check inventory.intent
intent compile inventory.intent
intent verify inventory.intent
```

## Key Insight

Union types in IntentLang are labels, not type references. `Open | Closed` defines two symbolic variants — they aren't separate types you define elsewhere. This keeps specs clean and readable while still being machine-verifiable.

Optional types (`T?`) explicitly mark which fields may be absent, making the spec a complete description of the data model with no hidden nullability surprises.

## Compiled IR

The file `inventory.ir.json` contains the pre-compiled Agent IR. Regenerate it with:

```bash
intent compile inventory.intent > inventory.ir.json
```

Explore the IR to see how collections, optionals, and unions are represented in the typed IR structure.
