# State Machines

State machines define named types with explicit allowed transitions between states.

## Syntax

```intent
state StatusName {
  StateA -> StateB -> StateC
  StateA -> StateD
}
```

Each transition chain (`->`) declares that transitions between adjacent states are valid. Multiple chains can share states to express branching.

## Example

```intent
module TaskTracker

state TaskStatus {
  Open -> InProgress -> Done
  Open -> Cancelled
  InProgress -> Blocked -> InProgress
}

entity Task {
  id: UUID
  title: String
  status: TaskStatus
}

action StartTask {
  task: Task

  requires {
    task.status == Open
  }

  ensures {
    task.status == InProgress
  }
}
```

This defines `TaskStatus` as a type with five states (`Open`, `InProgress`, `Done`, `Cancelled`, `Blocked`) and these valid transitions:
- `Open` → `InProgress`
- `InProgress` → `Done`
- `Open` → `Cancelled`
- `InProgress` → `Blocked`
- `Blocked` → `InProgress`

## Type Registration

A `state` name registers as a type, just like `entity`. You can use it as a field type, action parameter type, or quantifier binding type.

## Documentation

State machines support doc blocks:

```intent
--- Tracks the lifecycle of a customer order.
state OrderStatus {
  Pending -> Confirmed -> Shipped -> Delivered
  Pending -> Cancelled
}
```

## Code Generation

`intent codegen` produces language-idiomatic enums with transition validation:

| Language | Enum type | Validation method |
|----------|-----------|-------------------|
| Rust | `pub enum` | `is_valid_transition(&self, to: &Self) -> bool` |
| TypeScript | Union type | `isValidXxxTransition(from, to): boolean` |
| Python | `StrEnum` | `is_valid_transition(from_state, to_state)` |
| Go | `type X string` + constants | `IsValidTransition(to X) bool` |
| Java | `enum` | `canTransitionTo(Target target)` |
| C# | `enum` + extensions | `CanTransitionTo(this X from, X to)` |
| Swift | `enum: String, Codable` | `canTransition(to:) -> Bool` |

## Cross-Module Imports

State machines are included when importing a module with `use`:

```intent
module Types

state OrderStatus {
  Pending -> Confirmed -> Shipped
}
```

```intent
module Main

use Types

entity Order {
  id: UUID
  status: OrderStatus
}
```
