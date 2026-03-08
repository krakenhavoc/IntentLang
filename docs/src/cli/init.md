# Scaffolding a New Spec

```bash
intent init
```

Creates a new `.intent` file with a starter template including an example entity, action, and invariant.

## Options

| Flag | Description |
|------|-------------|
| `--name <NAME>` | Module name (defaults to current directory name, or `MyModule`) |
| `-o, --out <PATH>` | Output file path (defaults to `<name>.intent`) |

## Examples

```bash
# Use directory name as module name
intent init

# Specify a module name
intent init --name Payments

# Specify both name and output path
intent init --name Payments -o specs/payments.intent
```

## Generated template

Running `intent init --name Payments` creates `payments.intent`:

```intent
module Payments

--- TODO: Describe what this module specifies.

entity Example {
  id: UUID
  name: String
  status: Active | Inactive
}

action CreateExample {
  name: String

  requires {
    name != ""
  }

  ensures {
    exists e: Example => e.name == name
  }
}

invariant UniqueNames {
  forall a: Example => forall b: Example =>
    a.id != b.id => a.name != b.name
}
```

The template gives you working examples of each major construct. Replace them with your own entities, actions, and invariants.

## Behavior

- The module name's first letter is automatically capitalized
- If the output file already exists, the command exits with an error (no overwriting)
- Without `--name`, the module name is derived from the current directory name
