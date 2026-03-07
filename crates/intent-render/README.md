# intent-render

[![crates.io](https://img.shields.io/crates/v/intent-render.svg)](https://crates.io/crates/intent-render)
[![docs](https://img.shields.io/badge/docs-mdBook-blue)](https://krakenhavoc.github.io/IntentLang/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/krakenhavoc/IntentLang/blob/main/LICENSE)

Markdown and HTML renderer for [IntentLang](https://github.com/krakenhavoc/IntentLang) specifications.

Part of the IntentLang toolchain — a declarative specification language for human-AI collaboration.

## What it does

Converts a parsed IntentLang AST into readable documentation:

- **Markdown** — entity field tables, action signatures with requires/ensures, invariant expressions, edge case rules
- **HTML** — self-contained styled HTML with the same content (no external dependencies)

Both renderers use a shared `format_type` helper for consistent type display (unions, generics, optionals).

## Usage

```rust
use intent_parser::parse_file;
use intent_render::{markdown, html};

let source = r#"
module TransferFunds

entity Account {
  id: UUID
  balance: Decimal(precision: 2)
}
"#;

let module = parse_file(source).unwrap();

let md = markdown::render(&module);   // Markdown string
let page = html::render(&module);     // Self-contained HTML string
```

## Public API

| Export | Description |
|--------|-------------|
| `markdown::render(module) -> String` | Render AST to Markdown |
| `html::render(module) -> String` | Render AST to self-contained HTML |
| `format_type(type_expr) -> String` | Format a type expression as a string |

## Part of IntentLang

This crate is the rendering layer. Other crates in the workspace:

- **intent-parser** — PEG parser and typed AST
- **intent-check** — Semantic analysis and type checking
- **intent-ir** — Agent IR lowering, verification, and audit
- **intent-cli** — CLI binary (`intent check`, `intent verify`, etc.)
