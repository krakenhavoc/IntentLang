# AGENTS.md -- AI Agent Onboarding Guide

This file is for AI agents and coding assistants working on IntentLang.
For the full project spec, see `docs/SPEC.md`. For contributor conventions, see `CLAUDE.md`.

## What Is IntentLang?

IntentLang is a declarative specification language for human-AI collaboration. Humans write **what** and **what constraints** in `.intent` files; agents handle **how** via a compiled intermediate representation. Four layers:

0. **Natural Language** -- Describe what you want in plain English; an AI agent generates a formal `.intent` spec (Layer 0)
1. **Intent Layer** -- The spec language; humans write or refine specs directly (Layer 1)
2. **Agent IR** -- A verifiable intermediate representation agents generate
3. **Audit Bridge** -- Maps between layers so humans can review agent work

Layers 0 and 1 are both human-facing -- the system meets users at their comfort level.

## Quick Start

```bash
# Install Rust (if not present)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Run the test suite
cargo test --workspace

# Build the CLI
cargo build -p intent-cli

# Check a spec file
cargo run -p intent-cli -- check examples/transfer.intent

# Render a spec to Markdown
cargo run -p intent-cli -- render examples/transfer.intent

# Compile to IR (JSON output)
cargo run -p intent-cli -- compile examples/transfer.intent

# Verify structural + logical correctness
cargo run -p intent-cli -- verify examples/transfer.intent

# Show audit trace map (spec items -> IR constructs)
cargo run -p intent-cli -- audit examples/transfer.intent

# Show coverage summary
cargo run -p intent-cli -- coverage examples/transfer.intent

# Diff two versions of a spec
cargo run -p intent-cli -- diff old.intent new.intent

# Query specific items (for agent integration)
cargo run -p intent-cli -- query examples/transfer.intent entities
cargo run -p intent-cli -- query examples/transfer.intent Transfer

# Incremental verification (caches results, re-verifies only changes)
cargo run -p intent-cli -- verify --incremental examples/transfer.intent

# Multi-agent collaboration: lock/unlock spec items
cargo run -p intent-cli -- lock examples/transfer.intent Transfer --agent agent-1
cargo run -p intent-cli -- status examples/transfer.intent
cargo run -p intent-cli -- unlock examples/transfer.intent Transfer --agent agent-1

# Format a spec file
cargo run -p intent-cli -- fmt examples/transfer.intent
cargo run -p intent-cli -- fmt examples/transfer.intent --write

# Scaffold a new spec
cargo run -p intent-cli -- init --name MyModule

# Shell completions
cargo run -p intent-cli -- completions bash > intent.bash
cargo run -p intent-cli -- completions zsh > _intent

# Generate a spec from natural language (Layer 0)
AI_API_KEY=... cargo run -p intent-cli -- generate "a greeting service that stores greetings by name"
cargo run -p intent-cli -- generate --interactive "build a shopping cart"
cargo run -p intent-cli -- generate --edit examples/transfer.intent "add rate limiting"
cargo run -p intent-cli -- generate --confidence 2 "patient records system"

# Generate OpenAPI 3.0 spec
cargo run -p intent-cli -- openapi examples/transfer.intent
cargo run -p intent-cli -- openapi examples/transfer.intent -o transfer-api.json

# Generate skeleton code (Rust, TypeScript, Python, Go)
cargo run -p intent-cli -- codegen examples/transfer.intent --lang rust
cargo run -p intent-cli -- codegen examples/transfer.intent --lang go --out-dir ./generated

# JSON output (for agent consumption)
cargo run -p intent-cli -- --output json check examples/transfer.intent
```

## Project Structure

```
intentlang/
  grammar/intent.pest        -- PEG grammar (pest)
  crates/
    intent-parser/           -- Grammar -> typed AST + module resolver
    intent-check/            -- Semantic analysis & validation (incl. cross-module)
    intent-render/           -- AST -> Markdown/HTML/formatted source
    intent-ir/               -- AST -> Agent IR (lowering, verification, audit, diff, incremental, lock)
    intent-gen/              -- Natural language -> .intent spec (Layer 0, LLM-powered)
    intent-runtime/          -- Stateless runtime & HTTP server
    intent-lsp/              -- Language Server Protocol server (diagnostics, hover, go-to-def, completion)
    intent-codegen/          -- Skeleton code generator (Rust, TypeScript, Python, Go)
    intent-cli/              -- CLI binary: check, render, compile, verify, audit, coverage, diff, query, lock, unlock, status, fmt, init, completions, generate, serve, codegen, openapi
  editors/vscode/            -- VSCode extension (syntax highlighting, snippets, LSP client)
  examples/                  -- Example .intent files
  tests/valid/               -- Specs that must parse and pass checks
  tests/invalid/             -- Specs that must fail with known errors
  docs/SPEC.md               -- Full language specification
```

### Crate Dependency Graph

```
intent-cli -> intent-parser, intent-check, intent-render, intent-ir, intent-gen, intent-runtime, intent-codegen
intent-lsp -> intent-parser, intent-check (for diagnostics, hover, navigation)
intent-gen -> intent-parser, intent-check (for validation loop)
intent-runtime -> intent-ir (for contract evaluation)
intent-ir -> intent-parser
intent-codegen -> intent-parser
intent-check -> intent-parser
intent-render -> intent-parser
intent-parser -> pest (grammar/intent.pest)
```

---

## IntentLang Complete Language Reference

This section is a complete reference for the `.intent` specification language. It covers every construct, every type, every operator, and every validation rule. If you are generating `.intent` files, this section tells you everything you need to produce valid output.

### File Structure

Every `.intent` file has this structure, in order:

```intent
module ModuleName           -- required, exactly one, must be first

--- Optional documentation block.
--- Multiple lines allowed. Each starts with triple-dash.

use OtherModule             -- zero or more import declarations
use OtherModule.SpecificItem

entity ... { }              -- zero or more entity declarations
action ... { }              -- zero or more action declarations
invariant ... { }           -- zero or more invariant declarations
edge_cases { }              -- zero or one edge_cases block
```

**Rules:**
- The `module` declaration must come first. The name must be PascalCase (start with uppercase letter).
- An optional doc block may follow the module declaration.
- Zero or more `use` declarations may follow (after the doc block, before items).
- Top-level items (entity, action, invariant, edge_cases) can appear in any order and any quantity.
- Single-line comments use `//`.

### Entity Declarations

Entities define domain objects with typed fields.

```intent
entity Account {
  id: UUID
  balance: Decimal(precision: 2)
  currency: CurrencyCode
  status: Active | Frozen | Closed
  created_at: DateTime
}
```

**Rules:**
- Entity names must be PascalCase (start with uppercase).
- Each field has the form `field_name: TypeExpr`.
- Field names are snake_case or camelCase (start with any letter or underscore).
- An optional doc block (`---` lines) may precede the `entity` keyword.
- No duplicate entity names within a module.
- No duplicate field names within an entity.

### Action Declarations

Actions define behavioral operations with parameters, preconditions, postconditions, and metadata.

```intent
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
```

**Structure (all parts optional except the name):**

| Part | Description |
|------|-------------|
| Doc block | `---` lines inside the action, before parameters |
| Parameters | `name: Type` declarations, like entity fields |
| `requires { }` | Preconditions -- boolean expressions that must be true **before** execution |
| `ensures { }` | Postconditions -- boolean expressions that must be true **after** execution |
| `properties { }` | Key-value metadata annotations |

**Rules:**
- Action names must be PascalCase.
- No duplicate action names within a module.
- The `requires` block may only appear once per action.
- The `ensures` block may only appear once per action.
- The `properties` block may only appear once per action.
- Blocks must appear in order: parameters, then requires, then ensures, then properties.

#### The `requires` Block

Contains boolean expressions representing preconditions. Each expression on its own line is implicitly AND-ed together.

```intent
requires {
  account.status == Active       -- each line is a separate condition
  amount > 0                     -- all must be true
  account.balance >= amount
}
```

**Semantic rules:**
- `old()` is **NOT allowed** in `requires` blocks. Pre-state references make no sense in preconditions.
- Self-comparisons like `x == x` are flagged as tautological.

#### The `ensures` Block

Contains postconditions. Supports plain expressions and conditional `when` clauses.

```intent
ensures {
  -- Plain postcondition
  account.balance == old(account.balance) - amount

  -- Conditional postcondition (when/then pattern)
  when password_verified(password, hash) =>
    exists s: Session => s.user == user && s.revoked == false

  when !password_verified(password, hash) =>
    user.failed_attempts == old(user.failed_attempts) + 1
}
```

**Semantic rules:**
- `old(expr)` is allowed and encouraged -- it refers to the value of `expr` before the action executed.
- `when condition => consequence` creates conditional postconditions.

#### The `properties` Block

Key-value metadata. Values can be booleans, numbers, strings, identifiers, lists, or objects.

```intent
properties {
  idempotent: true                                    -- bool
  atomic: true                                        -- bool
  audit_logged: true                                  -- bool
  max_latency_ms: 500                                 -- number
  requires_role: "admin"                              -- string
  idempotency_key: request_id                         -- identifier
  sensitive_fields: [password]                        -- list
  rate_limited: { max: 10, window_seconds: 60 }       -- object
  requires_permission: { resource: "roles", action: "assign" }
}
```

### Invariant Declarations

Invariants are system-wide constraints that must **always** hold, across all actions.

```intent
invariant NoNegativeBalances {
  --- No account may ever have a negative balance.
  forall a: Account => a.balance >= 0
}

invariant NoDuplicateAssignments {
  --- A user cannot be assigned the same role twice.
  forall a1: RoleAssignment =>
    forall a2: RoleAssignment =>
      a1.user == a2.user && a1.role == a2.role => a1 == a2
}

invariant FailedRecordsTracked {
  --- Every failed record must have a dead letter entry.
  forall r: Record =>
    r.stage == Failed =>
      exists d: DeadLetterEntry => d.record == r
}
```

**Rules:**
- Invariant names must be PascalCase.
- The body is a single expression (typically a `forall` quantifier).
- An optional doc block can appear inside the invariant, before the expression.
- Invariants typically use `forall x: Entity => predicate` form.
- Nested quantifiers and implications (`=>`) are supported.

### Edge Cases Block

Explicit handling of boundary conditions. One block per module with multiple rules.

```intent
edge_cases {
  when from == to => reject("Cannot transfer to same account")
  when amount > 10000.00 => require_approval(level: "manager")
  when from.owner == to.owner => allow(note: "Self-transfer between own accounts")
  when route.status == Deprecated => allow(note: "Deprecated endpoint, migrate to v2")
}
```

**Structure:** `when condition => action_call(args)`

**Rules:**
- The condition is a boolean expression.
- The action is a function call -- typically `reject("reason")`, `allow(note: "...")`, or `require_approval(level: "...")`.
- Edge case action names that start with an uppercase letter must reference a defined action in the module.
- Arguments can be positional or named (`key: value`).

---

### Type System

#### Primitive Types

| Type | Description | Example |
|------|-------------|---------|
| `UUID` | Universally unique identifier | `id: UUID` |
| `String` | Text string | `name: String` |
| `Int` | Integer number | `count: Int` |
| `Decimal(precision: N)` | Fixed-precision decimal | `balance: Decimal(precision: 2)` |
| `Bool` | Boolean true/false | `active: Bool` |
| `DateTime` | Date and time | `created_at: DateTime` |

#### Domain Types

| Type | Description | Example |
|------|-------------|---------|
| `Email` | Email address | `email: Email` |
| `URL` | Web URL | `homepage: URL` |
| `CurrencyCode` | ISO currency code | `currency: CurrencyCode` |

#### Collection Types

| Type | Description | Example |
|------|-------------|---------|
| `List<T>` | Ordered collection | `items: List<CartItem>` |
| `Set<T>` | Unique collection | `permissions: Set<Permission>` |
| `Map<K, V>` | Key-value mapping | `metadata: Map<String, Int>` |

#### Type Modifiers

**Optional:** Append `?` to any type to make it nullable.
```intent
locked_until: DateTime?          -- may be null
error_message: String?
parent: Role?
```

**Union (enum-like):** Use `|` to define a set of named variants.
```intent
status: Active | Frozen | Closed
stage: Ingested | Validated | Transformed | Loaded | Failed
tier: Free | Pro | Enterprise
```

Union variants are **enum-like labels**, not references to other types. `Active`, `Frozen`, `Closed` are literal variant names -- they don't need to be defined as separate entities.

**Optional union:** The `?` applies to the entire union.
```intent
result: Open | Closed?           -- the whole union is optional (can be null)
```

**Parameterized types:** Some types take parameters.
```intent
balance: Decimal(precision: 2)   -- precision parameter
price: Decimal(precision: 4)
```

#### User-Defined Types

Any entity name can be used as a type in field declarations and parameters:
```intent
entity Cart {
  items: List<CartItem>          -- CartItem is a user-defined entity type
  owner: UUID
}

action AddItem {
  cart: Cart                     -- Cart is a user-defined entity type
  product: Product               -- Product is a user-defined entity type
}
```

---

### Expression Grammar

Expressions are used in `requires`, `ensures`, `invariant` bodies, and `edge_cases` conditions. Here is the complete expression grammar with operator precedence from **lowest to highest**:

| Precedence | Operator | Syntax | Associativity | Description |
|------------|----------|--------|---------------|-------------|
| 1 (lowest) | Implies | `a => b` | Left-to-right | Logical implication |
| 2 | Or | `a \|\| b` | Left-to-right | Logical OR |
| 3 | And | `a && b` | Left-to-right | Logical AND |
| 4 | Not | `!a` | Prefix (right) | Logical NOT |
| 5 | Comparison | `a == b`, `a != b`, `a > b`, `a < b`, `a >= b`, `a <= b` | Non-associative | Comparison (at most one operator) |
| 6 (highest) | Additive | `a + b`, `a - b` | Left-to-right | Addition, subtraction |

**Primary expressions** (highest precedence, form the atoms of all expressions):

| Form | Example | Description |
|------|---------|-------------|
| Identifier | `amount`, `status` | Variable or field reference |
| Field access | `from.balance`, `item.product.stock` | Dot-separated path (can chain) |
| Function call | `now()`, `lookup(User, email)`, `reject("msg")` | Call with positional or named args |
| `old(expr)` | `old(from.balance)` | Pre-execution state reference |
| `forall x: T => body` | `forall a: Account => a.balance >= 0` | Universal quantifier |
| `exists x: T => body` | `exists r: Record => r.id == id` | Existential quantifier |
| Number literal | `0`, `42`, `10000.00`, `-1` | Integer or decimal |
| String literal | `"hello"`, `"Cannot transfer"` | Quoted string (supports `\"` escapes) |
| Boolean literal | `true`, `false` | Boolean value |
| `null` | `null` | Null/absent value |
| List literal | `[a, b, c]` | List of expressions |
| Parenthesized | `(a + b)` | Explicit grouping |

#### Union Variant References in Expressions

Union variants like `Active`, `Frozen`, `Completed` appear as bare uppercase identifiers in expressions. They are **not** function calls or type references -- they're enum-like labels:

```intent
from.status == Active            -- comparing field to variant
r.stage == Failed                -- variant used as a value
record.stage != Loaded           -- variant in inequality
```

#### Quantifier Expressions

Quantifiers bind a variable to a type and assert something about all or some instances:

```intent
-- Universal: "for every Account a, a.balance >= 0"
forall a: Account => a.balance >= 0

-- Existential: "there exists a Session s where s.user == user"
exists s: Session => s.user == user && s.revoked == false

-- Nested quantifiers
forall a1: RoleAssignment =>
  forall a2: RoleAssignment =>
    a1.user == a2.user && a1.role == a2.role => a1 == a2

-- Quantifier with implication in body
forall r: Record =>
  r.stage == Failed =>
    exists d: DeadLetterEntry => d.record == r
```

**Rules:**
- The type after `:` must be an uppercase name (PascalCase) -- either a defined entity or a defined action.
- The `=>` after the type binding is the quantifier body separator, not the implies operator. The body itself is a full expression and can contain its own `=>` (implies).
- Quantifiers can reference action names (e.g., `forall t: Transfer => ...`) for temporal invariants.

#### The `old()` Expression

References the value of an expression **before** the action executed. Only valid in `ensures` blocks and temporal invariants.

```intent
ensures {
  from.balance == old(from.balance) - amount     -- "new balance = old balance minus amount"
  pipeline.records_processed == old(pipeline.records_processed) + 1
  user.failed_attempts == old(user.failed_attempts) + 1
}
```

#### Function Calls

Function calls use parentheses with optional positional or named arguments:

```intent
now()                                 -- zero-arg call
lookup(User, email)                   -- positional args
reject("Account is frozen")          -- string arg
require_approval(level: "manager")   -- named arg
allow(note: "Self-transfer")         -- named arg
password_verified(password, hash)    -- positional args
```

#### Field Access Chains

Dot notation accesses fields, and can be chained:

```intent
from.balance                         -- one level
item.product.stock                   -- two levels
lookup(User, email).status           -- call result then field
r.client.request_count               -- chained through entity references
```

---

### Semantic Validation Rules

The checker performs six passes of semantic analysis. Understanding these rules is critical for generating valid specs.

#### Pass 1: Duplicate Detection

- **No duplicate entity names** in a module.
- **No duplicate action names** in a module.
- **No duplicate invariant names** in a module.
- **No duplicate field names** within a single entity.

```intent
-- INVALID: duplicate entity
entity Account { id: UUID }
entity Account { name: String }     -- ERROR: duplicate entity "Account"

-- INVALID: duplicate field
entity Account {
  id: UUID
  id: String                        -- ERROR: duplicate field "id"
}
```

#### Pass 2: Type Resolution

Every type reference must resolve to either a built-in type or a defined entity.

**Built-in types:** `UUID`, `String`, `Int`, `Decimal`, `Bool`, `DateTime`, `CurrencyCode`, `Email`, `URL`

**Collection wrappers:** `List`, `Set`, `Map` (their type parameters are also resolved)

**Union variants are NOT type-checked.** Variants like `Active | Frozen` are treated as enum labels, not type references, so they don't need to correspond to defined entities.

```intent
entity Order {
  cart: Cart                         -- OK if Cart is defined as an entity
  owner: Customer                    -- ERROR if Customer is not defined
  status: Pending | Shipped          -- OK: union variants are labels, not types
  items: List<LineItem>              -- LineItem must be a defined entity
}
```

#### Pass 3: Quantifier Binding Validation

The type in `forall x: Type` and `exists x: Type` must be a defined entity or action name.

```intent
-- OK: Account is a defined entity
forall a: Account => a.balance >= 0

-- OK: Transfer is a defined action (for temporal invariants)
forall t: Transfer => old(t.from.balance) >= 0

-- ERROR: "String" is a built-in, not an entity/action
forall s: String => s != ""
```

#### Pass 4: Edge Case Action Validation

In edge_cases, action names that start with an uppercase letter must reference a defined action.

```intent
edge_cases {
  when x => reject("ok")            -- OK: "reject" is lowercase, treated as built-in
  when x => Transfer(from: a)       -- OK only if Transfer is a defined action
  when x => DoSomething()           -- ERROR if DoSomething is not defined
}
```

#### Pass 5: Field Access Validation

When an action parameter is typed as an entity, field accesses on that parameter are validated against the entity's field list.

```intent
entity Account {
  id: UUID
  balance: Decimal(precision: 2)
}

action Withdraw {
  account: Account

  requires {
    account.balance >= 0             -- OK: "balance" exists on Account
    account.email == "x"             -- ERROR: "email" not a field of Account
  }
}
```

#### Pass 6: Constraint Validation

- **`old()` in `requires`:** Using `old()` in a precondition is an error. There is no "old state" before execution begins.
- **Tautological comparisons:** Self-comparisons like `x == x` or `a.b == a.b` are flagged as always-true tautologies.

```intent
-- ERROR: old() in requires
requires {
  from.balance == old(from.balance)  -- ERROR: old() not valid in requires
}

-- ERROR: tautological
requires {
  from.balance == from.balance       -- ERROR: always true, likely a bug
}
```

---

### Documentation Blocks

Triple-dash (`---`) lines are documentation blocks. They can appear:

1. Before a top-level item (entity, action, invariant)
2. After the module declaration
3. Inside an action (before parameters)
4. Inside an invariant (before the expression)

```intent
module MyModule

--- Module-level documentation.
--- Can span multiple lines.

--- Entity documentation.
entity Account {
  id: UUID
}

action Transfer {
  --- Action-level documentation (inside the action).
  from: Account
}

invariant Rule {
  --- Invariant documentation (inside the invariant).
  forall a: Account => a.balance >= 0
}
```

Each `---` line is a separate doc line. The content after `---` is free-form natural language.

---

### Complete Annotated Example

Here is a fully annotated example demonstrating every language construct:

```intent
module ShoppingCart                   -- Module name (PascalCase, required, first line)

--- A shopping cart system.           -- Module doc block (optional, after module)
--- Supports item management and checkout.

entity Product {                      -- Entity declaration
  id: UUID                           -- Primitive type
  name: String
  price: Decimal(precision: 2)       -- Parameterized type
  stock: Int
  status: Available | Discontinued   -- Union type (enum-like variants)
}

entity CartItem {
  product: Product                   -- Entity reference (user-defined type)
  quantity: Int
}

entity Cart {
  id: UUID
  owner: UUID
  items: List<CartItem>              -- Collection type with entity parameter
  created_at: DateTime
  checked_out: Bool
}

entity Order {
  id: UUID
  cart: Cart
  total: Decimal(precision: 2)
  status: Pending | Confirmed | Shipped | Delivered | Cancelled
  created_at: DateTime
}

action AddItem {                     -- Action declaration
  --- Add a product to the cart.     -- Doc block inside action
  cart: Cart                         -- Parameters (like entity fields)
  product: Product
  quantity: Int

  requires {                         -- Preconditions (all implicitly AND-ed)
    cart.checked_out == false         -- Field access + comparison to bool
    product.status == Available      -- Field access + comparison to variant
    quantity > 0                     -- Comparison to number literal
    product.stock >= quantity        -- Comparison between fields
  }

  ensures {                          -- Postconditions
    exists item: CartItem =>         -- Existential quantifier
      item.product == product &&     -- Logical AND spanning lines
      item.quantity == quantity
  }

  properties {                       -- Metadata key-value pairs
    idempotent: false                -- Boolean value
  }
}

action Checkout {
  cart: Cart

  requires {
    cart.checked_out == false
    forall item: CartItem =>         -- Universal quantifier in requires
      item.product.stock >= item.quantity   -- Chained field access
  }

  ensures {
    cart.checked_out == true
    exists o: Order =>               -- Existential in ensures
      o.cart == cart &&
      o.status == Confirmed
  }

  properties {
    atomic: true
    audit_logged: true
  }
}

invariant StockNonNegative {         -- Invariant declaration
  --- Product stock can never go below zero.
  forall p: Product => p.stock >= 0  -- Universal constraint
}

invariant CartItemsPositive {
  forall item: CartItem => item.quantity > 0
}

edge_cases {                         -- Edge cases block (one per module)
  when product.status == Discontinued => reject("Product is no longer available")
  when product.stock < quantity => reject("Insufficient stock")
  when cart.checked_out == true => reject("Cart has already been checked out")
}
```

---

### Common Patterns

#### Pattern: State Machine via Union Types

Use union types to model states, preconditions to enforce valid transitions:

```intent
entity Record {
  stage: Ingested | Validated | Transformed | Loaded | Failed
}

action ValidateRecord {
  record: Record
  requires { record.stage == Ingested }
  ensures  { record.stage == Validated }
}

action TransformRecord {
  record: Record
  requires { record.stage == Validated }
  ensures  { record.stage == Transformed }
}
```

#### Pattern: Conservation / Balance Invariants

Use `old()` to assert that quantities are conserved:

```intent
invariant TransferConservation {
  forall t: Transfer =>
    old(t.from.balance) + old(t.to.balance) ==
    t.from.balance + t.to.balance
}
```

#### Pattern: Counter Increments

```intent
ensures {
  pipeline.records_processed == old(pipeline.records_processed) + 1
  user.failed_attempts == old(user.failed_attempts) + 1
}
```

#### Pattern: Conditional Postconditions

Use `when` clauses in `ensures` to express different outcomes:

```intent
ensures {
  when password_verified(password, hash) =>
    exists s: Session => s.user == user && s.revoked == false

  when !password_verified(password, hash) =>
    user.failed_attempts == old(user.failed_attempts) + 1
}
```

#### Pattern: Existence Guarantees

Use `exists` to assert that a record was created:

```intent
ensures {
  exists r: TransferRecord =>
    r.request_id == request_id &&
    r.status == Completed
}
```

#### Pattern: Uniqueness via Double Quantification

```intent
invariant NoDuplicateAssignments {
  forall a1: RoleAssignment =>
    forall a2: RoleAssignment =>
      a1.user == a2.user && a1.role == a2.role => a1 == a2
}
```

#### Pattern: Implication Chains in Invariants

```intent
invariant FailedRecordsTracked {
  forall r: Record =>
    r.stage == Failed =>                   -- "if stage is Failed..."
      exists d: DeadLetterEntry => d.record == r   -- "...then a DLE exists"
}
```

---

### Common Mistakes to Avoid

| Mistake | Why It Fails | Fix |
|---------|-------------|-----|
| `old()` in `requires` | No pre-state exists before execution | Move to `ensures` block |
| `x == x` comparison | Tautological, always true | Compare to a different value |
| Duplicate entity names | Checker rejects duplicates | Use unique names |
| Duplicate field names in entity | Checker rejects duplicates | Use unique field names |
| Undefined type reference | `cart: ShoppingCart` when only `Cart` is defined | Define the entity or fix the name |
| `forall x: String => ...` | Quantifier type must be entity/action, not primitive | Use a defined entity name |
| Uppercase edge action without definition | `when x => DoThing()` where `DoThing` isn't an action | Define the action or use lowercase |
| Accessing undefined field | `account.email` when Account has no `email` field | Check entity field list |
| Missing `module` declaration | Every file must start with `module Name` | Add `module MyModule` as first line |
| Module name in lowercase | `module myModule` | Use PascalCase: `module MyModule` |

---

## Architecture Notes

**Parser (intent-parser)**: PEG grammar via `pest`. Every AST node carries a `Span { start, end }` for source locations. The grammar uses `or_expr` (not `expr`) for `when`/`edge_rule` conditions to avoid ambiguity with the `=>` operator. Union variants like `Active | Frozen` are treated as enum-like labels, not type references. Includes a module resolver (`resolve.rs`) that loads and parses imported modules via DFS with cycle detection and topological ordering.

**Checker (intent-check)**: Six-pass semantic analysis with cross-module support:
1. Collect definitions + detect duplicates (entities, actions, invariants, fields)
2. Resolve type references (verify all types exist as builtins or defined entities)
3. Validate quantifier bindings (forall/exists variable types must be entities or actions)
4. Validate edge case action references (uppercase names must be defined actions)
5. Validate field access on entity-typed parameters (e.g., `from.balance` checks `balance` exists on the entity)
6. Constraint validation (`old()` not in requires, tautological self-comparisons)

`check_file_with_imports()` pre-populates the type environment with definitions from imported modules, enabling cross-module type resolution and field access validation.

Both parse and check errors use `miette` diagnostics with source spans, labels, and help text.

**Renderer (intent-render)**: Converts AST to Markdown, self-contained HTML, and canonical `.intent` source (formatter). Shared `format_type` and `format_literal` helpers in lib root.

**IR (intent-ir)**: Lowers AST to a typed intermediate representation (structs, functions, invariants, edge guards). Every IR node carries a `SourceTrace { module, item, part, span }` for audit tracing. Modules: `lower` (AST->IR), `verify` (structural + coherence), `audit` (trace maps + coverage), `diff` (spec-level diffs), `incremental` (cached per-item verification), `lock` (multi-agent spec-item claiming).

**Generator (intent-gen)**: Translates natural language to `.intent` specs via LLM. Uses OpenAI-compatible chat completions API via `ureq` (configurable base URL + model). Includes a generate-check-retry loop: generates spec, validates via parser/checker, feeds errors back to LLM for correction (max 2 retries). Supports `--confidence 1-5` levels, `--edit` for modifying existing specs, and `--diff` for patch output. Config: `AI_API_KEY`, `AI_API_BASE`, `AI_MODEL` env vars.

**Runtime (intent-runtime)**: Stateless execution engine. Evaluates expressions against concrete JSON values, enforces preconditions/postconditions/invariants, and auto-generates REST endpoints from actions. `old()` semantics via snapshot-and-compare.

**LSP (intent-lsp)**: Language Server Protocol server using `tower-lsp` + `tokio`. Provides real-time diagnostics (parse + semantic errors), go-to-definition (entity/action type references), hover (keyword help, entity docs, built-in types), and context-aware completion (keywords, types, entity names, action params). Uses `DashMap<Url, Document>` for concurrent per-file state. Full text sync (re-parse on every change). Cross-module diagnostics via the existing module resolver.

**VSCode Extension (editors/vscode/)**: TextMate grammar for syntax highlighting, language configuration (brackets, folding, comments), 15 code snippets, TypeScript LSP client. Install the `intent-lsp` binary (`cargo install --path crates/intent-lsp`), then build the extension (`npm install && npm run compile` in `editors/vscode/`).

**Codegen (intent-codegen)**: Generates typed skeleton code and OpenAPI 3.0 specs from AST (not IR, to preserve doc blocks and human-readable names). Per-language modules (`rust.rs`, `typescript.rs`, `python.rs`, `go.rs`) with shared type mapping (`types.rs`) and expression formatting (`lib.rs`). Reserved keyword escaping: Rust uses `r#keyword`, Python uses `keyword_` suffix, Go uses `keyword_` suffix. Smart imports, union type mapping (Rust enums, TS string literals, Python Literal types, Go string type aliases with const blocks and validation methods). Go output uses JSON struct tags and PascalCase exported fields. OpenAPI generator (`openapi.rs`) maps entities to JSON Schema components and actions to `POST /actions/{Name}` endpoints.

**CLI (intent-cli)**: `clap` derive-based. Subcommands: `check`, `render`, `render-html`, `compile`, `verify` (`--incremental`), `audit`, `coverage`, `diff`, `query`, `lock`, `unlock`, `status`, `fmt`, `init`, `completions`, `generate`, `serve`, `codegen`, `openapi`. Global `--output json` flag for agent consumption. Commands that operate on specs (`check`, `compile`, `verify`, `serve`) automatically resolve module imports when `use` declarations are present.

## Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| pest / pest_derive | 2.x | PEG parsing |
| miette | 7.x | Diagnostic error reporting |
| thiserror | 2.x | Error type derivation |
| clap | 4.x | CLI argument parsing |
| serde | 1.x | AST/IR serialization |
| serde_json | 1.x | IR JSON output |
| ureq | 2.x | HTTP client for LLM API calls (intent-gen) |
| similar | 2.x | Diff output for edit mode (intent-cli) |
| tower-lsp | 0.20.x | LSP server framework (intent-lsp) |
| tokio | 1.x | Async runtime for LSP server |
| dashmap | 6.x | Concurrent per-file document state (intent-lsp) |

## Conventions

- **Tests first**: Write a failing test before implementing a feature.
- **Error messages matter**: Include source spans and actionable suggestions.
- **Crates stay focused**: Parser doesn't validate semantics. Checker doesn't render.
- **Grammar rules get comments**: Link to the relevant SPEC.md section.
- Run `cargo test --workspace` before committing. All tests must pass.

## Current Test Coverage (285 total)

- 31 semantic checker tests (duplicates, type resolution, quantifiers, edge actions, field access, constraints, valid files, 5 cross-module import tests)
- 28 parser tests (12 unit + 7 insta snapshot + 7 module resolver + 2 example parsing)
- 74 IR tests: 13 lowering + 11 verification + 6 coherence + 9 audit + 13 diff + 11 incremental + 11 lock
- 49 runtime tests: 7 contract + 42 expression evaluator
- 74 codegen tests (43 skeleton + 31 OpenAPI): naming helpers + Rust/TypeScript/Python/Go entity/action/type mapping + OpenAPI spec generation + full example files
- 23 LSP tests: 8 document/line-index + 5 diagnostics + 4 hover + 3 navigation + 3 completion
- 6 intent-gen tests: 3 strip_fences + 3 validate_spec
- Fixtures: 4 valid, 9 invalid + 6 example files + 2 multi-module example files

## Current Phase & Status

Phases 1-7 complete. Current release: v0.6.0-beta.1. VSCode extension and LSP server shipped (PR #41).

Phase 1 (complete): PEG grammar, typed AST with spans, six-pass semantic analysis, Markdown/HTML renderers. CLI: `check`, `render`, `render-html`.

Phase 2 (complete): AST -> IR lowering, structural verification, coherence analysis. CLI: `compile`, `verify`.

Phase 3 (complete): Audit trace maps, coverage summaries, spec-level diffs. CLI: `audit`, `coverage`, `diff`.

Phase 4 (complete): Agent API (`--output json`, `query`), incremental verification (`verify --incremental`), multi-agent collaboration (`lock`, `unlock`, `status`).

Phase 5 (complete): Language polish -- `fmt` (auto-formatter), `init` (scaffolding), `completions` (shell completions), list literal expressions. Natural language generation -- `generate` (NL -> `.intent` via LLM, Layer 0).

Phase 6 (complete): Stateless runtime -- expression evaluator, contract evaluation, HTTP server. CLI: `serve`.

Phase 7 (complete): Module imports -- `use` syntax, module resolver, cross-module type checking. Multi-file composition for real-world projects.

Phase 8 (in progress): Code Generation -- skeleton codegen for Rust, TypeScript, Python, Go shipped. CLI: `codegen`. Planned: Java, C#, Swift targets. AI-powered `intent implement` (LLM generates full implementations from spec contracts).
