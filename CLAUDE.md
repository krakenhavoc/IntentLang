# CLAUDE.md — IntentLang Project

## Project Overview

IntentLang is a four-layer programming system for human-agent collaboration:

0. **Natural Language** — A natural language interface where humans describe what they want in plain English. An AI agent translates the description into a formal intent spec. Meets users at the lowest barrier to entry.
1. **Intent Layer** — A declarative spec language where humans define *what* and *what constraints*, not *how*. Humans can author directly or refine specs generated from Layer 0.
2. **Agent IR** — A dense, formally verifiable intermediate representation that agents generate and maintain.
3. **Audit Bridge** — Tooling that maps between layers so humans can review agent work at the spec level.

Layers 0 and 1 are both human-facing. Layer 0 is for anyone who can describe an idea in English. Layer 1 is for those who want to write or refine formal specs directly.

The full design spec is in `docs/SPEC.md`. Read it before making architectural decisions.

## Current Phase: Phase 1 — Intent Language MVP

We are building the intent language parser, type checker, and CLI toolchain.

### Immediate Goals
1. **Formal grammar** for the intent language (PEG or EBNF)
2. **Parser** that produces a typed AST
3. **Semantic analyzer** that validates:
   - Type consistency within entities and actions
   - Constraint satisfiability (basic: are requires/ensures not contradictory?)
   - Completeness (do all referenced entities exist? are all fields typed?)
4. **Renderer** that produces clean Markdown/HTML from intent specs
5. **CLI** with commands: `intent check <file>`, `intent render <file>`

### Technical Decisions

**Host language: Rust**
- Strong type system aligns with the project's values
- Excellent parser toolchain (pest, nom, or tree-sitter)
- Good WASM compilation story for future browser tooling
- Fast enough for incremental re-checking

**Parser approach: PEG grammar (pest)**
- Readable grammar definition
- Good error messages
- Straightforward AST generation

**Project structure:**
```
intentlang/
├── CLAUDE.md              ← You are here
├── docs/
│   └── SPEC.md            ← Full design specification
├── grammar/
│   └── intent.pest        ← PEG grammar definition
├── crates/
│   ├── intent-parser/     ← Grammar → AST
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── ast.rs     ← AST node types
│   │   │   └── parser.rs  ← pest parser wrapper
│   │   └── Cargo.toml
│   ├── intent-check/      ← Semantic analysis & validation
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── types.rs   ← Type checking
│   │   │   └── constraints.rs ← Constraint validation
│   │   └── Cargo.toml
│   ├── intent-render/     ← Spec → Markdown/HTML
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── markdown.rs
│   │   │   └── html.rs
│   │   └── Cargo.toml
│   ├── intent-gen/        ← Natural language → .intent (Layer 0)
│   │   ├── src/
│   │   │   ├── lib.rs     ← Public API: generate(prompt) -> Result<String>
│   │   │   ├── prompt.rs  ← System prompt construction (syntax ref, examples)
│   │   │   ├── client.rs  ← LLM API client (OpenAI-compatible, model-agnostic)
│   │   │   └── validate.rs ← Generate-check-retry loop
│   │   └── Cargo.toml
│   ├── intent-runtime/    ← Stateless runtime & HTTP server (Phase 6, planned)
│   │   ├── src/
│   │   │   ├── lib.rs     ← Public API: Runtime::from_ir() -> serve
│   │   │   ├── eval.rs    ← Expression evaluator (concrete values)
│   │   │   ├── transform.rs ← State transformer (apply ensures)
│   │   │   ├── value.rs   ← Runtime value types
│   │   │   └── serve.rs   ← HTTP server (auto-generated REST)
│   │   └── Cargo.toml
│   ├── intent-codegen/    ← Skeleton code generator (Rust, TypeScript, Python, Go)
│   ├── intent-lsp/        ← Language Server Protocol (LSP) server
│   │   ├── src/
│   │   │   ├── main.rs        ← Binary entry point (tokio + tower-lsp)
│   │   │   ├── server.rs      ← LanguageServer trait impl
│   │   │   ├── document.rs    ← Per-file state, LineIndex (byte↔LSP position)
│   │   │   ├── diagnostics.rs ← Parse/check error → LSP diagnostic conversion
│   │   │   ├── hover.rs       ← Hover provider (keywords, types, entities)
│   │   │   ├── navigation.rs  ← Go-to-definition
│   │   │   └── completion.rs  ← Context-aware completions
│   │   └── Cargo.toml
│   └── intent-cli/        ← CLI entry point
│       ├── src/
│       │   └── main.rs
│       └── Cargo.toml
├── editors/
│   └── vscode/            ← VSCode extension
│       ├── syntaxes/      ← TextMate grammar
│       ├── snippets/      ← Code snippets
│       ├── src/           ← TypeScript LSP client
│       └── package.json
├── examples/              ← Example .intent files
│   ├── transfer.intent
│   ├── auth.intent
│   └── shopping_cart.intent
├── tests/                 ← Integration tests
│   ├── valid/             ← Specs that should pass
│   └── invalid/           ← Specs that should fail with known errors
└── Cargo.toml             ← Workspace root
```

## Code Conventions

- **Tests first**: Write a failing test before implementing a feature. Every AST node, every validation rule, every error case gets a test.
- **Error messages matter**: Parser and checker errors should be clear and actionable. Include line/column numbers and suggestions. Users of this tool are humans writing specs — they need helpful feedback.
- **Keep crates focused**: Each crate does one thing. The parser doesn't validate semantics. The checker doesn't render output.
- **Document the grammar**: Every grammar rule in `intent.pest` should have a comment explaining what it matches and linking to the relevant section of SPEC.md.

## Intent Language Quick Reference

```intent
module ModuleName

--- Documentation block (natural language description)

entity EntityName {
  field_name: Type
  other_field: TypeA | TypeB    // union types
}

action ActionName {
  param: Type

  requires {
    // preconditions (boolean expressions)
  }

  ensures {
    // postconditions using old() for pre-state references
  }

  properties {
    key: value
  }
}

invariant InvariantName {
  // universal constraints: forall x: Type => predicate
}

edge_cases {
  when condition => action
}
```

### Types
- Primitives: `UUID`, `String`, `Int`, `Decimal(precision: N)`, `Bool`, `DateTime`
- Domain types: `CurrencyCode`, `Email`, `URL` (extensible)
- Collections: `List<T>`, `Set<T>`, `Map<K, V>`
- Optional: `T?`
- Union: `A | B | C`
- Refinement: inline constraints in requires/ensures blocks

### Operators
- Comparison: `==`, `!=`, `>`, `<`, `>=`, `<=`
- Logical: `&&`, `||`, `!`, `=>`(implies)
- Quantifiers: `forall`, `exists`
- State: `old(expr)` — value of expr before action execution

## Working on This Project

### Before starting any task:
1. Re-read this file and `docs/SPEC.md`
2. Check existing tests to understand current coverage
3. Run `cargo test` to confirm everything passes

### When adding a new language feature:
1. Add example usage to an `.intent` file in `examples/`
2. Add the grammar rule to `intent.pest`
3. Add AST nodes to `ast.rs`
4. Add parser logic and tests
5. Add semantic checks and tests
6. Update the renderer
7. Run full test suite

### When fixing a bug:
1. Write a failing test that reproduces the bug
2. Fix the issue
3. Confirm the test passes
4. Check for similar patterns elsewhere

## Key Design Decisions Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Host language | Rust | Type safety, performance, WASM story |
| Parser | PEG (pest) | Readable grammars, good errors |
| File extension | `.intent` | Clear, not taken |
| Workspace | Cargo workspaces | Modular crate structure |
| Error handling | `miette` crate | Beautiful diagnostic errors with source spans |
| CLI framework | `clap` (derive) | Standard, well-maintained |
| NL generation API | OpenAI-compatible | Model-agnostic, supports any provider |
| NL generation crate | `intent-gen` | Separated from core toolchain, optional dependency |
| LSP server | `tower-lsp` + `tokio` | Standard Rust LSP framework, async runtime |
| VSCode extension | TextMate + TS client | Syntax highlighting + LSP integration |
| Skeleton codegen | `intent-codegen` crate | Deterministic typed stubs, no API key needed |

## Completed Phases

- **Phase 1**: Intent Language MVP — PEG grammar, typed AST, six-pass semantic analysis, Markdown/HTML renderers. CLI: `check`, `render`, `render-html`.
- **Phase 2**: Agent IR — AST → IR lowering, structural verification, coherence analysis. CLI: `compile`, `verify`.
- **Phase 3**: Audit Bridge — Trace maps, coverage summaries, spec-level diffs. CLI: `audit`, `coverage`, `diff`.
- **Phase 4**: Agent Integration — JSON output, structured queries, incremental verification, multi-agent collaboration. CLI: `query`, `lock`, `unlock`, `status`.
- **Phase 5**: Language Polish & NL Generation — `fmt`, `init`, `completions`, list literals. `intent generate` for NL → `.intent` via LLM (Layer 0).
- **Phase 6**: Stateless Runtime — expression evaluator, contract evaluation, HTTP server. CLI: `serve`.
- **Phase 7**: Module Imports — `use` syntax, module resolver (DFS + cycle detection), cross-module type checking, multi-file composition.
- **Phase 8** (in progress): Code Generation — skeleton codegen (`intent codegen`, Rust/TypeScript/Python/Go/Java/C#/Swift), AI-powered implementation generation (`intent implement`, Rust/TypeScript/Python).

### Codegen Roadmap

**Skeleton codegen** (`intent codegen`) generates deterministic typed stubs from specs. Current and planned target languages:

| Language | Status | Notes |
|----------|--------|-------|
| Rust | Shipped | Structs, enums, `r#` keyword escaping, smart imports |
| TypeScript | Shipped | Interfaces, camelCase, inline string literal unions |
| Python | Shipped | Dataclasses, `keyword_` escaping, Literal types |
| Go | Shipped | Structs, `string` union types with const/validation, JSON tags, smart imports |
| Java | Shipped | Records (Java 16+), enums, module wrapper class, Javadoc, smart imports |
| C# | Shipped | Records (C# 10+), `#nullable enable`, file-scoped namespaces, XML doc comments |
| Swift | Shipped | Structs with Codable, String-backed enums, `throws` functions, Foundation imports |
| IntentLang | Planned (self-hosting) | Generate `.intent` specs from `.intent` specs |

**AI-powered codegen** (`intent implement`, planned) — uses LLM to generate full implementations from specs, with contracts as constraints. Same generate-check loop as `intent-gen` but in reverse (spec → code instead of NL → spec).

### Long-Term: Self-Hosting
IntentLang compiles itself — the compiler's spec is written in `.intent` files, agents generate the implementation, the audit bridge verifies conformance. Not a near-term priority, but a planned goal. See the Self-Hosting Roadmap section below for stages and invariants.

### Milestone Definitions
- **Alpha**: Core features working, API unstable, may have missing pieces
- **Beta**: Small real-world system runs end-to-end. Module imports working. API stabilizing
- **Preview**: Post-feedback hardening between beta and production (if needed)
- **Stable (v1.0)**: Production-ready runtime, stable API

Versions are not hardlocked to phases — minor/patch versions ship between milestones as needed.

---

### Self-Hosting Roadmap

#### Goal
IntentLang compiles itself. The compiler's specification is written in the intent layer, its implementation is agent-generated IR, and the audit bridge verifies the compiler conforms to its own spec.

#### Stage 1: Host-Compiled (Current)
- Toolchain written in Rust
- Intent language is specification-only (no execution semantics)
- Rust is the single source of truth for compiler behavior

#### Stage 2: Executable IR (aligns with Phase 6)
- IntentLang gains its own native runtime — specs execute directly via the `intent-runtime` crate
- No WASM or LLVM compilation target; the runtime evaluates IR natively
- Stateless HTTP server auto-generates REST endpoints from action definitions
- Toolchain is still Rust, but IntentLang programs are now self-sufficient

#### Stage 3: Spec-Described Compiler
- The compiler's own behavior is specified in `.intent` files:
  - `compiler/parsing.intent` — grammar rules, AST structure, error recovery
  - `compiler/typechecking.intent` — type rules, constraint satisfaction
  - `compiler/codegen.intent` — IR generation, optimization passes
  - `compiler/audit.intent` — trace map generation, coverage analysis
- Agents generate IR implementations from these specs
- The Rust compiler still bootstraps the first generation
- Audit bridge verifies the agent-produced compiler against its own spec

#### Stage 4: Full Self-Hosting
- The IntentLang compiler compiles itself
- Bootstrap path: a pinned binary of the previous compiler version builds the next (same as Rust, Go, etc.)
- The Rust implementation becomes a historical artifact, retained only as an emergency bootstrap
- Humans maintain the compiler exclusively through intent specs
- Agents own the implementation layer entirely

#### Self-Hosting Invariants
```intent
module CompilerBootstrap

invariant SpecFidelity {
  --- The self-hosted compiler must produce identical output
  --- to the Rust reference compiler for all valid inputs.
  forall input: IntentFile =>
    selfhosted_compile(input) == reference_compile(input)
}

invariant BootstrapStability {
  --- Compiling the compiler with itself must produce
  --- a binary that also compiles itself to the same binary.
  --- (Fixed-point property.)
  selfhosted_compile(compiler_source) ==
    compile_with(selfhosted_compile(compiler_source), compiler_source)
}
```

#### Open Questions (Self-Hosting Specific)

1. **Native runtime scope**: The runtime currently interprets IR. If performance becomes critical for self-hosting, do we add a compilation step (to native or WASM), or is interpretation sufficient?
2. **Verification of the verifier**: When the audit bridge is itself spec'd in IntentLang, who verifies the verifier? This is a known problem in formal methods — at some point you need a trusted kernel. How small can we make it?
3. **Agent trust boundary**: At Stage 4, agents maintain the tool that verifies agent work. What safeguards prevent a subtle drift where the verifier gradually accepts weaker proofs?