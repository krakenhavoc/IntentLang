# 01 - Hello Intent

The simplest possible IntentLang spec: one entity and one action.

## Concepts Introduced

- `module` — Every spec starts with a module declaration
- `---` — Documentation blocks (natural language descriptions)
- `entity` — Defines a data structure with typed fields
- `action` — Defines an operation with parameters
- `requires` — Preconditions that must be true before the action runs
- `ensures` — Postconditions guaranteed after the action completes

## Try It

```bash
# Parse and validate
intent check hello.intent

# Render as readable Markdown
intent render hello.intent

# Render as HTML
intent render-html hello.intent
```

## What's Happening

The spec declares a `Greeting` entity with three fields (`id`, `name`, `message`), and a `CreateGreeting` action that:
- **Requires** the name is non-empty
- **Ensures** a matching greeting exists after execution

This is pure specification — it says *what* should happen, not *how*. An agent would later generate the implementation.

## Compiled IR

The file `hello.ir.json` contains the compiled Agent IR for this spec. You can regenerate it:

```bash
intent compile hello.intent > hello.ir.json
```

The IR maps each spec construct to a typed representation with source traces:

| Intent | IR |
|--------|-----|
| `entity Greeting` | `struct Greeting` with typed fields |
| `action CreateGreeting` | `function CreateGreeting` with pre/postconditions |

Every IR node carries a `SourceTrace` linking back to the exact byte range in the `.intent` file — this is how the Audit Bridge (Layer 3) verifies nothing was invented or lost.
