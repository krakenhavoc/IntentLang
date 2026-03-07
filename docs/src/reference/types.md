# Type System

IntentLang has a rich type system designed for specification clarity.

## Primitive types

| Type | Description |
|------|-------------|
| `UUID` | Universally unique identifier |
| `String` | Text value |
| `Int` | Integer |
| `Bool` | Boolean (`true` / `false`) |
| `DateTime` | Date and time value |

## Numeric types

```intent
balance: Decimal(precision: 2)
```

`Decimal(precision: N)` specifies a fixed-precision decimal number. The `precision` parameter indicates the number of decimal places.

## Domain types

| Type | Description |
|------|-------------|
| `CurrencyCode` | ISO 4217 currency code (e.g., USD, EUR) |
| `Email` | Email address |
| `URL` | URL string |

Domain types are extensible — they serve as semantic markers for downstream tooling.

## Collection types

```intent
items: List<CartItem>       // ordered list
tags: Set<String>           // unique set
metadata: Map<String, Int>  // key-value map
```

| Type | Description |
|------|-------------|
| `List<T>` | Ordered collection of `T` |
| `Set<T>` | Unordered unique collection of `T` |
| `Map<K, V>` | Key-value mapping |

## Optional types

Append `?` to make a type optional:

```intent
locked_until: DateTime?     // may be null
last_login: DateTime?
```

## Union types

Union types define a closed set of possible values:

```intent
status: Active | Frozen | Closed
```

Union variants are enum-like labels (PascalCase). They are not references to other types — `Active` is a value, not a type.
