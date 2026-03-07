# Contributing to IntentLang

Thanks for your interest in IntentLang! This document covers how to get started.

## Getting Started

```bash
# Clone and build
git clone https://github.com/krakenhavoc/IntentLang.git
cd IntentLang
cargo build --workspace

# Run the test suite
cargo test --workspace

# Try the CLI
cargo run -p intent-cli -- check examples/transfer.intent
```

## Development Workflow

1. Create a feature branch off `main`
2. Write a failing test before implementing
3. Make your changes
4. Run the full test suite: `cargo test --workspace`
5. Run `cargo fmt --all` and `cargo clippy --workspace`
6. Open a PR against `main`

## Project Structure

The project is a Cargo workspace with five crates:

| Crate | Purpose |
|-------|---------|
| `intent-parser` | PEG grammar (pest) and typed AST |
| `intent-check` | Semantic analysis and type checking |
| `intent-render` | Markdown and HTML rendering |
| `intent-ir` | IR lowering, verification, and audit bridge |
| `intent-cli` | CLI entry point |

Each crate has a focused responsibility. The parser doesn't validate semantics; the checker doesn't render output.

## Conventions

- **Tests first** — Every feature, validation rule, and error case gets a test.
- **Error messages matter** — Include source spans and actionable suggestions.
- **Document grammar rules** — Every rule in `grammar/intent.pest` should have a comment.
- **Keep it simple** — Don't add abstractions for one-time operations.

## Adding a Language Feature

1. Add example usage to an `.intent` file in `examples/`
2. Add the grammar rule to `grammar/intent.pest`
3. Add AST nodes to `intent-parser/src/ast.rs`
4. Add parser logic and tests
5. Add semantic checks and tests in `intent-check`
6. Update the renderer in `intent-render`
7. Run `cargo test --workspace`

## Key Files

- [`CLAUDE.md`](CLAUDE.md) — Full project conventions and design decisions
- [`AGENTS.md`](AGENTS.md) — Quick-start guide for AI agents
- [`docs/SPEC.md`](docs/SPEC.md) — Language design specification
- [`grammar/intent.pest`](grammar/intent.pest) — PEG grammar definition

## Reporting Issues

Open an issue on [GitHub Issues](https://github.com/krakenhavoc/IntentLang/issues). Include:

- What you expected
- What happened
- A minimal `.intent` file that reproduces the problem
- The output of `intent check` on that file

## Discussions

Have a question or idea? Start a thread in [GitHub Discussions](https://github.com/krakenhavoc/IntentLang/discussions).
