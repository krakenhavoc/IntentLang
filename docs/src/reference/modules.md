# Modules

Every `.intent` file begins with a module declaration:

```intent
module TransferFunds
```

The module name must start with an uppercase letter and use PascalCase. It serves as a namespace for all definitions in the file.

## Documentation blocks

Documentation blocks use `---` and can appear after the module declaration or before any entity, action, or invariant:

```intent
module TransferFunds

--- A fund transfer system between accounts within the same currency.
--- Supports basic account-to-account transfers with balance validation,
--- idempotency guarantees, and manager approval for large amounts.
```

Documentation blocks are preserved in rendered output and audit traces, making them useful for explaining design intent to both humans and tooling.

## Imports

Modules can import definitions from other modules using `use` declarations. Imports appear after the doc block and before any top-level items.

### Whole-module import

Import all entities, actions, and invariants from another module:

```intent
module Banking

use Types
```

This makes every definition from the `Types` module available in `Banking`. The resolver looks for `Types.intent` in the same directory as the importing file.

### Selective import

Import a single item from another module:

```intent
module Banking

use Types.Account
```

This imports only the `Account` entity from `Types`.

### Multiple imports

```intent
module App

use Types
use Auth.User
use Billing
```

### Resolution rules

- `use Foo` looks for `Foo.intent` in the same directory as the importing file.
- Circular imports are detected and produce an error.
- Diamond dependencies (A imports B and C, both import D) are handled correctly -- each module is parsed only once.
- Imported definitions do not shadow local definitions. If both the local module and an imported module define `Account`, the local definition takes precedence.

### Example: multi-module project

**Types.intent:**
```intent
module Types

entity Account {
  id: UUID
  balance: Decimal(precision: 2)
  status: Active | Frozen | Closed
}
```

**Banking.intent:**
```intent
module Banking

use Types

action Transfer {
  from: Account
  to: Account
  amount: Decimal(precision: 2)

  requires {
    from.status == Active
    to.status == Active
    amount > 0
    from.balance >= amount
  }

  ensures {
    from.balance == old(from.balance) - amount
    to.balance == old(to.balance) + amount
  }
}
```

```
$ intent check Banking.intent
OK: Banking — 1 top-level item(s), no issues found
```
