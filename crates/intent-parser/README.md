# intent-parser

[![crates.io](https://img.shields.io/crates/v/intent-parser.svg)](https://crates.io/crates/intent-parser)
[![docs](https://img.shields.io/badge/docs-mdBook-blue)](https://krakenhavoc.github.io/IntentLang/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/krakenhavoc/IntentLang/blob/main/LICENSE)

PEG parser and typed AST for the [IntentLang](https://github.com/krakenhavoc/IntentLang) specification language.

Part of the IntentLang toolchain — a declarative specification language for human-AI collaboration.

## What it does

Parses `.intent` files into a typed AST with source spans on every node. The grammar is defined as a PEG using [pest](https://pest.rs/).

Supports the full IntentLang syntax: modules, entities (with union and collection types), actions (requires/ensures/properties), invariants (with quantifiers), and edge cases.

## Usage

```rust
use intent_parser::{parse_file, Module};

let source = r#"
module Example

entity User {
  name: String
  email: Email
}
"#;

let module: Module = parse_file(source)?;
assert_eq!(module.name, "Example");
assert_eq!(module.entities[0].name, "User");
```

## Public API

| Export | Description |
|--------|-------------|
| `parse_file(src) -> Result<Module>` | Parse `.intent` source into a typed AST |
| `Module`, `Entity`, `Action`, `Invariant`, ... | AST node types with source spans |
| `ParseError` | Error type with source location info |

## Part of IntentLang

This crate is the parser layer. Other crates in the workspace:

- **intent-check** — Semantic analysis and type checking
- **intent-render** — Markdown and HTML rendering
- **intent-ir** — Agent IR lowering, verification, and audit
- **intent-cli** — CLI binary (`intent check`, `intent verify`, etc.)
