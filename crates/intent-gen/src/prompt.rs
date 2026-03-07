//! System prompt construction for LLM-based spec generation.
//!
//! Builds a system prompt containing the IntentLang syntax reference and
//! generation instructions, so the LLM can produce valid `.intent` files.

/// Build the system prompt for generating a new spec from scratch.
pub(crate) fn system_prompt(confidence: u8) -> String {
    let ci = confidence_instruction(confidence);
    format!("{ROLE}\n\n{SYNTAX_REFERENCE}\n\n{GENERATION_RULES}\n\n{ci}")
}

/// Build the system prompt for editing an existing spec.
pub(crate) fn edit_system_prompt(confidence: u8) -> String {
    let ci = confidence_instruction(confidence);
    format!("{ROLE}\n\n{SYNTAX_REFERENCE}\n\n{EDIT_RULES}\n\n{ci}")
}

/// Build a user message for generating a new spec.
pub(crate) fn generation_user_message(description: &str) -> String {
    format!(
        "Generate an IntentLang specification for the following:\n\n{description}\n\n\
         Respond with ONLY the `.intent` file content. No explanation, no markdown fences."
    )
}

/// Build a user message for editing an existing spec.
pub(crate) fn edit_user_message(existing: &str, instruction: &str) -> String {
    format!(
        "Here is the existing IntentLang specification:\n\n```\n{existing}\n```\n\n\
         Apply the following changes: {instruction}\n\n\
         Respond with ONLY the complete updated `.intent` file content. \
         No explanation, no markdown fences."
    )
}

/// Build a retry message with validation errors.
pub(crate) fn retry_message(spec: &str, errors: &[String]) -> String {
    let error_list = errors
        .iter()
        .enumerate()
        .map(|(i, e)| format!("{}. {}", i + 1, e))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "The generated spec has validation errors:\n\n{error_list}\n\n\
         Here was the spec:\n```\n{spec}\n```\n\n\
         Common mistakes to avoid:\n\
         - `---` is a LINE PREFIX, not a separator. Write `--- text here` NOT `---` alone on a line\n\
         - Do NOT use import/use/fn/let/return/if-else — IntentLang has none of these\n\
         - Do NOT wrap output in markdown code fences\n\
         - Each requires/ensures condition must be on its own line\n\
         - Union variants are bare identifiers (Active, not \"Active\")\n\
         - old() is only valid inside ensures blocks\n\n\
         Fix the errors and respond with ONLY the corrected `.intent` file content. \
         No explanation, no markdown fences."
    )
}

fn confidence_instruction(level: u8) -> String {
    match level {
        1 => "Be very conservative. Only include fields and constraints you are highly \
              confident about from the description. Prefer fewer items with correct types \
              over comprehensive coverage."
            .to_string(),
        2 => "Be somewhat conservative. Include what is clearly described and make \
              reasonable inferences for directly implied fields."
            .to_string(),
        3 => "Balance coverage and accuracy. Include what is described and make reasonable \
              inferences. Use common domain patterns where appropriate."
            .to_string(),
        4 => "Be thorough. Include described items plus inferred fields, constraints, \
              and edge cases that are standard for this domain."
            .to_string(),
        5 => "Be comprehensive. Include everything described plus all reasonable \
              inferences, common invariants, edge cases, and domain-standard patterns. \
              Aim for a production-ready specification."
            .to_string(),
        _ => confidence_instruction(3),
    }
}

const ROLE: &str = "\
You are an IntentLang specification generator. Your job is to translate natural \
language descriptions into valid `.intent` specification files. You produce ONLY \
raw `.intent` source code — no markdown fences, no explanations, no commentary.";

const SYNTAX_REFERENCE: &str = "\
# IntentLang Syntax Reference

IntentLang is a declarative specification language. It is NOT a general-purpose \
programming language. There are no functions, no import statements, no return \
statements, no loops, no variable assignments. You define entities (data), \
actions (operations with pre/postconditions), invariants (universal rules), and \
edge cases.

## Structure
Every file starts with `module ModuleName` (PascalCase). Nothing else may appear \
before the module declaration.

Documentation blocks use `---` as a LINE PREFIX (not a separator). Each doc line \
must start with `--- ` followed by text on the SAME line. Example:
```
--- This is a doc line.
--- This is another doc line.
```
WRONG (do NOT do this):
```
---
This text is NOT a doc block, it will cause parse errors.
---
```

## Entities
Define domain objects with typed fields:
```
entity Account {
  id: UUID
  owner: String
  balance: Decimal(precision: 2)
  status: Active | Frozen | Closed
  created_at: DateTime
  email: Email?
}
```

## Actions
Define operations with parameters, preconditions, postconditions, and properties:
```
action Transfer {
  --- Move funds between accounts.
  from: Account
  to: Account
  amount: Decimal(precision: 2)

  requires {
    from.status == Active
    to.status == Active
    amount > 0
    from.balance >= amount
  }

  ensures {
    from.balance == old(from.balance) - amount
    to.balance == old(to.balance) + amount
  }

  properties {
    idempotent: true
    atomic: true
  }
}
```

## Invariants
Universal constraints using `forall` or `exists` over entity/action types:
```
invariant NoNegativeBalances {
  forall a: Account => a.balance >= 0
}
```

## Edge Cases
Conditional rules using `when condition => action(...)`:
```
edge_cases {
  when amount > 10000 => require_approval(level: \"manager\")
  when from.id == to.id => reject(\"Cannot transfer to same account\")
}
```

## Types
- Primitives: UUID, String, Int, Decimal(precision: N), Bool, DateTime
- Domain types: CurrencyCode, Email, URL
- Collections: List<T>, Set<T>, Map<K, V>
- Optional: T? (nullable)
- Union: Active | Frozen | Closed (enum-like labels, NOT type references)
- Union variants are bare uppercase identifiers, not strings

## Operators
- Comparison: ==, !=, >, <, >=, <=
- Logical: &&, ||, !, => (implies)
- Quantifiers: forall x: Type => predicate, exists x: Type => predicate
- State: old(expr) — value before action execution (only in ensures blocks)
- Arithmetic: +, -, *, /

## Critical Rules
- `---` is a LINE PREFIX, not a separator. Write `--- text here` NOT `---` on its own line
- Each requires/ensures condition goes on its OWN LINE — no semicolons, no commas
- Union variants (Active, Frozen, etc.) are bare identifiers, NOT quoted strings
- old() is ONLY valid inside `ensures` blocks
- forall/exists bind a variable to an entity or action type defined in the same file
- properties values can be: true, false, quoted strings, or numbers
- There is NO `import`, `use`, `fn`, `let`, `return`, `if/else`, or `match` syntax
- Do NOT wrap output in markdown code fences";

const GENERATION_RULES: &str = "\
# Generation Rules
1. Always start with `module ModuleName` — derive the name from the description.
2. Add a `---` documentation block after the module declaration.
3. Define entities for all domain objects mentioned or implied.
4. Define actions for all operations described.
5. Add `requires` blocks for preconditions and `ensures` blocks for postconditions.
6. Add `invariant` blocks for domain rules that must always hold.
7. Add `edge_cases` for error handling and boundary conditions.
8. Use appropriate types — prefer specific types (Email, URL, CurrencyCode) over String.
9. Use union types for status fields and enums (e.g., Active | Inactive).
10. Every field must have a type. Every entity/action must have at least one field.";

const EDIT_RULES: &str = "\
# Edit Rules
1. Preserve the existing module name and structure unless the edit requires changing them.
2. Apply the requested changes precisely — do not remove or modify unrelated parts.
3. If adding new entities/actions, follow the style of existing ones.
4. Maintain all existing invariants unless explicitly asked to change them.
5. The output must be a complete, valid `.intent` file (not a diff or partial).";
