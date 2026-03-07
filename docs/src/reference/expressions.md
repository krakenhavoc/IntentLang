# Expressions

Expressions are used in `requires`, `ensures`, `invariant`, and `edge_cases` blocks.

## Comparison operators

| Operator | Meaning |
|----------|---------|
| `==` | Equal |
| `!=` | Not equal |
| `>` | Greater than |
| `<` | Less than |
| `>=` | Greater or equal |
| `<=` | Less or equal |

## Logical operators

| Operator | Meaning |
|----------|---------|
| `&&` | Logical AND |
| `\|\|` | Logical OR |
| `!` | Logical NOT |
| `=>` | Implies |

The implication operator `=>` is right-associative: `a => b` means "if a, then b".

## Quantifiers

```intent
// Universal quantifier
forall a: Account => a.balance >= 0

// Existential quantifier
exists s: Session => s.user.email == email && s.revoked == false
```

Quantifiers bind a variable to a type and apply a predicate. The binding type must be a defined entity or action.

## Field access

```intent
from.balance
from.status
item.product.name
```

Dotted field access navigates entity fields. The checker validates that each field exists on the referenced type.

## Function calls

```intent
old(from.balance)           // pre-state reference
now()                       // current time
lookup(User, email)         // entity lookup
password_verified(a, b)     // domain function
require_approval(level: "manager")  // with named args
```

### `old(expr)`

References the value of an expression *before* the current action executed. Only valid in `ensures` blocks and temporal invariants (invariants that quantify over actions).

```intent
ensures {
  from.balance == old(from.balance) - amount
}
```

Using `old()` in a `requires` block is an error:

```
intent::check::old_in_requires

  × `old()` cannot be used in a `requires` block
  help: `old()` references pre-state values and is only meaningful in `ensures` blocks
```

## Literals

```intent
42              // integer
10000.00        // decimal
"hello"         // string
true / false    // boolean
null            // null (for optional comparisons)
```
