# Architecture Details

## Crate Structure
- **intent-parser**: PEG grammar (pest) → typed AST. Grammar at grammar/intent.pest.
  - ast.rs: All AST node types with Span for source locations and Serialize for JSON output
  - parser.rs: pest wrapper, Pair→AST builders, unit tests against example files
- **intent-check**: Semantic analysis (depends on intent-parser)
  - types.rs: TypeEnv built from AST, validates type references
  - constraints.rs: Constraint validation (stub)
- **intent-render**: AST → Markdown/HTML (depends on intent-parser)
  - markdown.rs: Working basic renderer
  - html.rs: Stub
- **intent-cli**: Binary `intent` with subcommands `check` and `render` (clap derive)

## Key Dependencies
- pest/pest_derive 2.x for PEG parsing
- miette 7.x for diagnostic errors (fancy feature for CLI)
- thiserror 2.x for error types
- clap 4.x (derive) for CLI
- serde 1.x for AST serialization
- insta for snapshot testing (dev-dependency of parser)

## Grammar Design Decisions
- WHITESPACE includes newlines — the language is whitespace-insensitive
- `when` and `edge_rule` conditions parse as `or_expr` (stops before `=>`), preventing ambiguity with the implies operator
- `primary = { atom ~ ("." ~ ident)* }` handles both field access and post-call field access (e.g., `lookup(User, email).status`)
- `call_or_ident` unifies function calls and identifiers — the `(` disambiguates
- Keywords (forall, exists, old, when, null, true, false) are context-sensitive, not reserved — PEG ordered choice handles them
- `null_literal` and `bool_literal` use `!ASCII_ALPHANUMERIC` word boundary to prevent matching prefixes
- `doc_line` is atomic to preserve doc content from whitespace consumption

## Test Coverage
- 7 unit tests in parser::tests covering: minimal module, entity with all type forms, action with requires/ensures, invariant with quantifier, edge cases, full transfer.intent, full auth.intent
- 4 valid test fixtures: minimal, entity_only, all_types, full_action
- 3 invalid test fixtures: missing_module, unclosed_brace, bad_type
- 3 examples: transfer.intent, auth.intent, shopping_cart.intent
