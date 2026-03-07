# Compiling to IR

```bash
intent compile <file>
```

Lowers an IntentLang spec to the Agent IR — a typed intermediate representation designed for machine consumption.

The output is JSON, suitable for piping to other tools:

```bash
intent compile examples/transfer.intent > transfer.ir.json
intent compile examples/transfer.intent | jq '.entities'
```

## What the IR contains

The Agent IR includes:
- **Entities** with typed fields and source traces
- **Actions** with lowered preconditions, postconditions, and properties
- **Invariants** with fully resolved expressions
- **Edge case rules** with condition/handler pairs

Every IR node carries a `SourceTrace` linking it back to the original spec location (file, line, column). This trace is the foundation of the audit bridge.
