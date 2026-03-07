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
| `verify --incremental <file>` | Incremental verification (cache, re-verify only changes) |
| `audit <file>` | Show audit trace map (spec to IR) |
| `coverage <file>` | Show coverage summary |
| `diff <old> <new>` | Spec-level diff between two versions |
| `query <file> <target>` | Query specific items (for agent integration) |
| `lock <file> <item> --agent X` | Claim a spec item for an agent |
| `unlock <file> <item> --agent X` | Release a claimed spec item |
| `status <file>` | Show lock status for all spec items |
| `fmt <file>` | Format a spec file (`--write` to overwrite, `--check` to verify) |
| `init` | Scaffold a new `.intent` file (`--name`, `-o`) |
| `completions <shell>` | Generate shell completions (bash, zsh, fish, etc.) |

## Global options

```
intent --output json    JSON output (for agent consumption)
intent --help           Show help
intent --version        Show version
```

See the subpages for detailed usage of each command.
