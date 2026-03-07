# CLI Usage

The `intent` CLI provides commands for checking, rendering, compiling, and verifying IntentLang specs.

```
intent <command> <file>
```

## Commands

| Command | Description |
|---------|-------------|
| `check <file>` | Parse, type-check, and validate constraints |
| `render <file>` | Render spec to Markdown |
| `render-html <file>` | Render spec to self-contained styled HTML |
| `compile <file>` | Compile spec to Agent IR (JSON) |
| `verify <file>` | Verify structural + logical correctness |
| `audit <file>` | Show audit trace map (spec to IR) |
| `coverage <file>` | Show coverage summary |

## Global options

```
intent --help       Show help
intent --version    Show version
```

See the subpages for detailed usage of each command.
