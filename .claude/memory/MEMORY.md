# IntentLang Project Memory

## Project State
- Phase 1: Intent Language MVP (parser, type checker, CLI)
- Rust workspace with 4 crates: intent-parser, intent-check, intent-render, intent-cli
- PEG grammar (pest) in grammar/intent.pest
- Git: main branch has initial files, feature branches for work
- User git config: krakenhavoc <krakenhavoc@protonmail.com> — co-author commits with Claude

## Architecture Notes
- See [architecture.md](architecture.md) for crate details and grammar design decisions
- pest grammar uses `or_expr` (not `expr`) for `when`/`edge_rule` conditions to avoid `=>` ambiguity
- `type_ident` (uppercase start) vs `ident` (any alpha start) separates types from values in grammar
- Quantifier body greedily consumes all operators — users need parens to scope quantifiers tightly

## Environment
- Devcontainer: Ubuntu base, no Rust pre-installed (install via rustup)
- Rust 1.94.0 installed at ~/.cargo
- No `gh` CLI — install if needed for PR workflows
- Source `.cargo/env` before any cargo/rustc commands

## Workflow Preferences
- User wants their GitHub identity as commit author, Claude as co-author
- Treat as git repo with proper branching (feature branches off main)
- Save memory in /workspaces/IntentLang/.claude/memory/ (project-local, survives container rebuilds)
