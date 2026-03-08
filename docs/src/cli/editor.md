# Editor Integration

IntentLang ships with a VSCode extension and Language Server Protocol (LSP) server for a rich editing experience.

## Features

| Feature | Description |
|---------|-------------|
| Syntax highlighting | Full TextMate grammar for `.intent` files |
| Code snippets | 15 snippets for entity, action, invariant, imports, quantifiers, etc. |
| Diagnostics | Real-time parse and semantic errors on open/change/save |
| Go-to-definition | Jump from type references to entity/action declarations (F12) |
| Hover | Keyword help, entity/action docs with field listings, built-in type descriptions |
| Completion | Context-aware: top-level keywords, types after `:`, action params in requires/ensures |

## Setup

### 1. Install the LSP server

```bash
# From source
cargo install --path crates/intent-lsp

# Or, if published to crates.io
cargo install intent-lsp
```

This installs the `intent-lsp` binary to your Cargo bin directory (typically `~/.cargo/bin/`).

### 2. Install the VSCode extension

```bash
cd editors/vscode
npm install
npm run compile
```

Then in VSCode: Command Palette (`Ctrl+Shift+P`) > "Developer: Install Extension from Location..." > select the `editors/vscode/` directory.

### 3. Open a `.intent` file

Open any `.intent` file and the extension activates automatically. If the `intent-lsp` binary is on your PATH, you'll get full LSP features.

## Configuration

The extension provides two settings:

| Setting | Default | Description |
|---------|---------|-------------|
| `intentlang.server.path` | `""` (uses PATH) | Path to the `intent-lsp` binary |
| `intentlang.server.enabled` | `true` | Enable/disable the LSP server |

## How It Works

The LSP server (`intent-lsp`) runs as a stdio process. When you open a `.intent` file:

1. The extension spawns the `intent-lsp` binary
2. On every file change, the server re-parses and re-checks the file
3. Parse errors and semantic diagnostics are pushed back as editor squiggles
4. Hover, completion, and go-to-definition requests are handled against the cached AST

For files with `use` imports, the server resolves imported modules from disk and runs cross-module type checking.

## Cursor

[Cursor](https://cursor.com/) is a VS Code fork with built-in AI features. The IntentLang extension works identically in Cursor since it is fully compatible with VS Code extensions.

### Setup

The setup process is the same as VS Code:

1. **Install the LSP server** (see [above](#1-install-the-lsp-server))
2. **Build the extension**:
   ```bash
   cd editors/vscode
   npm install
   npm run compile
   ```
3. **Install the extension**: In Cursor, open the Command Palette (`Ctrl+Shift+P` / `Cmd+Shift+P`) > "Developer: Install Extension from Location..." > select the `editors/vscode/` directory.
4. **Open a `.intent` file** and the extension activates automatically.

### Configuration

All VS Code settings work the same in Cursor:

| Setting | Default | Description |
|---------|---------|-------------|
| `intentlang.server.path` | `""` (uses PATH) | Path to the `intent-lsp` binary |
| `intentlang.server.enabled` | `true` | Enable/disable the LSP server |

### Cursor Rules for AI-Assisted Development

The IntentLang repo includes `.cursor/rules/` files that teach Cursor's AI about the project:

- **`intentlang-syntax.mdc`** -- Activates for `.intent` files. Teaches Cursor's AI the IntentLang syntax, type system, expression grammar, and common mistakes to avoid. Enables AI-assisted writing and editing of `.intent` specs.
- **`project-conventions.mdc`** -- Activates for Rust source files and `.pest` grammar files. Teaches Cursor's AI the codebase structure, crate responsibilities, dependency graph, and development conventions.

These rules are checked into the repository so all contributors get them automatically.

## Without the LSP

If the `intent-lsp` binary is not installed, you still get:

- Syntax highlighting (TextMate grammar)
- Code snippets
- Bracket matching, folding, and auto-indentation
- Comment toggling (`Ctrl+/`)

A warning message will suggest installing the LSP server for full functionality.
