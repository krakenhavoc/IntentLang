# CLAUDE.md — IntentLang Project

## Project Overview

IntentLang is a three-layer programming system for human-agent collaboration:

1. **Intent Layer** — A declarative spec language where humans define *what* and *what constraints*, not *how*.
2. **Agent IR** — A dense, formally verifiable intermediate representation that agents generate and maintain.
3. **Audit Bridge** — Tooling that maps between layers so humans can review agent work at the spec level.

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
│   └── intent-cli/        ← CLI entry point
│       ├── src/
│       │   └── main.rs
│       └── Cargo.toml
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

## Future Phases (don't build yet, but design with these in mind)

- **Phase 2**: Agent IR — the intent language compiles to a typed IR that agents can manipulate. The AST should be designed with this compilation step in mind.
- **Phase 3**: Audit Bridge — trace maps from IR back to spec. The AST should carry enough source location info to support this.
- **Phase 4**: Agent API — agents read specs and produce IR via an API. The parser should be embeddable as a library.

---

### Self-Hosting Roadmap

#### Goal
IntentLang compiles itself. The compiler's specification is written in the intent layer, its implementation is agent-generated IR, and the audit bridge verifies the compiler conforms to its own spec.

#### Stage 1: Host-Compiled (Current)
- Toolchain written in Rust
- Intent language is specification-only (no execution semantics)
- Rust is the single source of truth for compiler behavior

#### Stage 2: Executable IR
- Agent IR gains full execution capability (compiles to WASM or native via LLVM)
- Intent specs compile to IR, IR compiles to runnable artifacts
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

1. **Compilation target**: WASM gives portability and sandboxing. LLVM gives native performance. Do we need both, or pick one for bootstrap?
2. **Verification of the verifier**: When the audit bridge is itself spec'd in IntentLang, who verifies the verifier? This is a known problem in formal methods — at some point you need a trusted kernel. How small can we make it?
3. **Agent trust boundary**: At Stage 4, agents maintain the tool that verifies agent work. What safeguards prevent a subtle drift where the verifier gradually accepts weaker proofs?