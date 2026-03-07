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

## Structure
Every file starts with `module ModuleName` (PascalCase).

Documentation blocks start with `---` and contain natural language descriptions.

## Entities
```
entity EntityName {
  field_name: Type
  other_field: TypeA | TypeB
}
```

## Actions
```
action ActionName {
  param: Type

  requires {
    // preconditions (boolean expressions)
  }

  ensures {
    // postconditions, can use old(expr) for pre-state
  }

  properties {
    key: value
  }
}
```

## Invariants
```
invariant InvariantName {
  forall x: Type => predicate
}
```

## Edge Cases
```
edge_cases {
  when condition => action
}
```

## Types
- Primitives: UUID, String, Int, Decimal(precision: N), Bool, DateTime
- Domain types: CurrencyCode, Email, URL
- Collections: List<T>, Set<T>, Map<K, V>
- Optional: T?
- Union: A | B | C (enum-like labels)
- Refinement: inline constraints in requires/ensures blocks

## Operators
- Comparison: ==, !=, >, <, >=, <=
- Logical: &&, ||, !, => (implies)
- Quantifiers: forall, exists
- State: old(expr) — value before action execution
- Arithmetic: +, -, *, /

## Lists
- Literal: [item1, item2, item3]
- Methods: .contains(x), .length, .is_empty";

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
