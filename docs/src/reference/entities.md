# Entities

Entities define data structures with typed fields:

```intent
entity Account {
  id: UUID
  owner: String
  balance: Decimal(precision: 2)
  currency: CurrencyCode
  status: Active | Frozen | Closed
  created_at: DateTime
}
```

## Fields

Each field has a name and a type, separated by `:`. Field names use `snake_case`.

Fields can use any of the available types:

```intent
entity CartItem {
  product: Product       // reference to another entity
  quantity: Int          // primitive type
  notes: String?         // optional type
  tags: List<String>     // collection type
}
```

## Documentation

Entities can have documentation blocks:

```intent
entity TransferRecord {
  --- Records a completed or pending transfer for audit purposes.
  id: UUID
  amount: Decimal(precision: 2)
  status: Pending | Completed | Failed | Reversed
}
```

## Entity references

Entities can reference other entities as field types. The checker validates that all referenced entity names are defined:

```intent
entity Order {
  id: UUID
  cart: Cart           // must be defined as an entity
  total: Decimal(precision: 2)
}
```

If `Cart` is not defined, the checker reports:

```
intent::check::undefined_type

  × undefined type `Cart`
  help: define an entity named `Cart`, or use a built-in type
```
