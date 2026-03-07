# intent-cli

[![crates.io](https://img.shields.io/crates/v/intent-cli.svg)](https://crates.io/crates/intent-cli)
[![docs](https://img.shields.io/badge/docs-mdBook-blue)](https://krakenhavoc.github.io/IntentLang/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/krakenhavoc/IntentLang/blob/main/LICENSE)

CLI toolchain for [IntentLang](https://github.com/krakenhavoc/IntentLang) — a declarative specification language for human-AI collaboration.

## Install

```bash
cargo install intent-cli
```

## Commands

```
intent check <file>                    Parse, type-check, and validate constraints
intent render <file>                   Render spec to Markdown
intent render-html <file>              Render spec to self-contained styled HTML
intent compile <file>                  Compile spec to IR (JSON output)
intent verify <file>                   Verify structural + logical correctness
intent verify --incremental <file>     Incremental verify (cache, re-verify only changes)
intent audit <file>                    Show audit trace map (spec -> IR)
intent coverage <file>                 Show coverage summary
intent diff <old> <new>                Spec-level diff between two versions
intent query <file> <target>           Query items (entities, actions, invariants, etc.)
intent lock <file> <item> --agent X    Claim a spec item for an agent
intent unlock <file> <item> --agent X  Release a claimed spec item
intent status <file>                   Show lock status for all spec items
intent fmt <file>                      Format a spec file (--write, --check)
intent init                            Scaffold a new .intent file (--name, --out)
intent completions <shell>             Generate shell completions (bash, zsh, fish, etc.)
```

All commands support `--output json` for machine-readable output.

## Example

```bash
$ intent check examples/transfer.intent
OK: TransferFunds — 7 top-level item(s), no issues found

$ intent query examples/transfer.intent entities
Entities: Account, Transfer

$ intent verify examples/transfer.intent
Verification passed: 0 errors
Coherence obligations: 2
  - InvariantPreservation: ExecuteTransfer must preserve NonNegativeBalance
  - TemporalProperty: AuditTrail quantifies over ExecuteTransfer
```

## Part of IntentLang

This crate is the CLI entry point. It wires together the other crates:

- **intent-parser** — PEG parser and typed AST
- **intent-check** — Semantic analysis and type checking
- **intent-render** — Markdown and HTML rendering
- **intent-ir** — Agent IR lowering, verification, and audit

Full documentation: [krakenhavoc.github.io/IntentLang](https://krakenhavoc.github.io/IntentLang/)
