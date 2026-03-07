# Your First Spec

Let's write a simple IntentLang spec from scratch and run it through the toolchain.

## Create a file

Create `hello.intent` with the following content:

```intent
module HelloWorld

--- A minimal task tracker to demonstrate IntentLang basics.

entity Task {
  id: UUID
  title: String
  done: Bool
}

action CompleteTask {
  task: Task

  requires {
    task.done == false
  }

  ensures {
    task.done == true
  }
}

invariant TasksHaveTitle {
  forall t: Task => t.title != ""
}
```

## Check it

Run the semantic checker:

```bash
intent check hello.intent
```

Output:

```
OK: HelloWorld — 3 top-level item(s), no issues found
```

The checker validates:
- All types are defined or built-in
- Field accesses are valid
- `old()` is only used in `ensures` blocks
- Quantifier bindings reference valid types
- No tautological comparisons

## Render it

Generate a Markdown summary:

```bash
intent render hello.intent
```

Or a self-contained HTML document:

```bash
intent render-html hello.intent > hello.html
```

## Compile to IR

Lower the spec to Agent IR:

```bash
intent compile hello.intent
```

This outputs a JSON representation of the typed intermediate representation that agents can consume.

## Verify

Run structural verification on the compiled IR:

```bash
intent verify hello.intent
```

This checks that the IR is well-formed, all references resolve, and coherence obligations are satisfied.

## What's next?

- Read the [Language Reference](../reference/modules.md) for the full syntax
- Browse the [Examples](../examples/transfer.md) to see real-world specs
- See the [CLI Reference](../cli/usage.md) for all available commands
