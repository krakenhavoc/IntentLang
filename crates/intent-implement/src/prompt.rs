//! Prompt construction for AI-powered implementation generation.
//!
//! Builds system prompts and user messages that give the LLM enough context
//! to produce compilable, contract-aware implementations from intent specs.

use crate::context::PromptContext;
use intent_codegen::Language;

/// Build the system prompt for implementation generation.
pub fn system_prompt(lang: Language) -> String {
    let lang_name = language_name(lang);
    let lang_guidance = language_guidance(lang);

    format!(
        "{ROLE}\n\n\
         ## Target Language: {lang_name}\n\n\
         {lang_guidance}\n\n\
         {IMPLEMENTATION_RULES}"
    )
}

/// Build the user message with spec, skeleton, contracts, and test harness.
pub fn user_message(context: &PromptContext, lang: Language) -> String {
    let lang_name = language_name(lang);
    let lang_tag = language_tag(lang);

    let mut msg = format!(
        "Generate a complete {lang_name} implementation for the following IntentLang specification.\n\n\
         ## Specification\n\n\
         ```intent\n{spec}\n```\n\n\
         ## Skeleton Code\n\n\
         Start from this skeleton and implement all function bodies:\n\n\
         ```{lang_tag}\n{skeleton}\n```\n\n\
         ## Contracts\n\n\
         {contracts}",
        spec = context.spec_source.trim(),
        skeleton = context.skeleton.trim(),
        contracts = context.contracts.trim(),
    );

    if !context.test_harness.is_empty() {
        msg.push_str(&format!(
            "\n\n## Contract Tests\n\n\
             Include this test module at the bottom of your file. \
             Your implementation MUST make all tests pass. \
             Entity-typed parameters must accept `&mut` references so tests can \
             verify postconditions on the mutated state.\n\n\
             ```{lang_tag}\n{harness}\n```",
            harness = context.test_harness.trim(),
        ));
    }

    msg.push_str(
        "\n\nRespond with ONLY the complete source file. No markdown fences, no explanation.",
    );
    msg
}

/// Build a retry message with validation errors.
pub fn retry_message(code: &str, errors: &[String], lang: Language) -> String {
    let lang_name = language_name(lang);
    let error_list = errors
        .iter()
        .enumerate()
        .map(|(i, e)| format!("{}. {}", i + 1, e))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "The generated {lang_name} code has validation errors:\n\n\
         {error_list}\n\n\
         Here was the code:\n```\n{code}\n```\n\n\
         Fix the errors and respond with ONLY the corrected source file. \
         No explanation, no markdown fences."
    )
}

fn language_name(lang: Language) -> &'static str {
    match lang {
        Language::Rust => "Rust",
        Language::TypeScript => "TypeScript",
        Language::Python => "Python",
        Language::Go => "Go",
    }
}

fn language_tag(lang: Language) -> &'static str {
    match lang {
        Language::Rust => "rust",
        Language::TypeScript => "typescript",
        Language::Python => "python",
        Language::Go => "go",
    }
}

fn language_guidance(lang: Language) -> &'static str {
    match lang {
        Language::Rust => RUST_GUIDANCE,
        Language::TypeScript => TYPESCRIPT_GUIDANCE,
        Language::Python => PYTHON_GUIDANCE,
        Language::Go => GO_GUIDANCE,
    }
}

const ROLE: &str = "\
You are a code implementation generator. Given an IntentLang specification, \
its skeleton code (typed stubs), and a contracts summary, you produce a \
complete, working implementation. You output ONLY raw source code — no \
markdown fences, no explanations, no commentary.";

const IMPLEMENTATION_RULES: &str = "\
## Implementation Rules

1. Start from the provided skeleton code (structs, function signatures, types).
2. Implement every function body — replace all `todo!()`, `throw`, or `raise` stubs.
3. Honor all preconditions (requires) as runtime checks. Return an error or panic \
   if a precondition is violated.
4. Honor all postconditions (ensures). Where `old(x)` appears, capture the \
   pre-state value before mutation and verify the postcondition holds.
5. Honor all invariant constraints in the implementation logic.
6. Handle all edge cases listed in the contracts summary.
7. Use idiomatic patterns for the target language.
8. Do not add new public types or functions beyond what the skeleton defines. \
   You may add private helpers.
9. Do not import external crates/packages beyond the standard library unless \
   the skeleton already imports them.
10. Output ONLY the complete source file. No markdown fences, no explanation.";

const RUST_GUIDANCE: &str = "\
- Use `Result<T, E>` for fallible operations. Define a local error enum if needed.
- For `old(expr)`: clone the value before mutation, then assert the postcondition.
- Use `#[derive(Debug, Clone, PartialEq)]` on structs.
- Decimal types map to `f64` (or a decimal library if one is in scope).
- UUID maps to `String` unless a uuid crate is imported.
- Use `assert!()` or return `Err` for precondition/postcondition violations.";

const TYPESCRIPT_GUIDANCE: &str = "\
- Use `throw new Error(...)` for precondition violations.
- For `old(expr)`: use spread/destructuring to capture pre-state before mutation.
- Decimal types map to `number`.
- UUID maps to `string`.
- Use TypeScript strict mode idioms (explicit types, no `any`).";

const PYTHON_GUIDANCE: &str = "\
- Use `raise ValueError(...)` for precondition violations.
- For `old(expr)`: use `copy.deepcopy()` or explicit capture before mutation.
- Decimal types map to `Decimal` from the `decimal` module.
- UUID maps to `str` (or `uuid.UUID` if imported).
- Use type hints throughout.";

const GO_GUIDANCE: &str = "\
- Return `error` values for precondition violations. Use `fmt.Errorf(...)`.
- For `old(expr)`: copy the struct value before mutation, then verify postconditions.
- Decimal types map to `float64`.
- UUID maps to `string`.
- Use exported (PascalCase) names for public structs and functions.
- Use JSON struct tags matching the field names.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_contains_role_and_rules() {
        let prompt = system_prompt(Language::Rust);
        assert!(prompt.contains("code implementation generator"));
        assert!(prompt.contains("Implementation Rules"));
        assert!(prompt.contains("Rust"));
    }

    #[test]
    fn test_system_prompt_language_specific() {
        let ts = system_prompt(Language::TypeScript);
        assert!(ts.contains("TypeScript"));
        assert!(ts.contains("throw new Error"));

        let py = system_prompt(Language::Python);
        assert!(py.contains("Python"));
        assert!(py.contains("raise ValueError"));
    }

    #[test]
    fn test_user_message_includes_all_sections() {
        let ctx = crate::context::PromptContext {
            spec_source: "module Test\n".to_string(),
            skeleton: "struct Test {}\n".to_string(),
            contracts: "### Action: Foo\n".to_string(),
            test_harness: String::new(),
        };
        let msg = user_message(&ctx, Language::Rust);

        assert!(msg.contains("module Test"));
        assert!(msg.contains("struct Test"));
        assert!(msg.contains("Action: Foo"));
        assert!(msg.contains("No markdown fences"));
    }

    #[test]
    fn test_user_message_includes_test_harness() {
        let ctx = crate::context::PromptContext {
            spec_source: "module Test\n".to_string(),
            skeleton: "struct Test {}\n".to_string(),
            contracts: "### Action: Foo\n".to_string(),
            test_harness: "#[cfg(test)]\nmod contract_tests {\n    fn test_foo() {}\n}\n"
                .to_string(),
        };
        let msg = user_message(&ctx, Language::Rust);

        assert!(msg.contains("Contract Tests"));
        assert!(msg.contains("mod contract_tests"));
        assert!(msg.contains("&mut"));
        assert!(msg.contains("MUST make all tests pass"));
    }

    #[test]
    fn test_user_message_skips_empty_harness() {
        let ctx = crate::context::PromptContext {
            spec_source: "module Test\n".to_string(),
            skeleton: "struct Test {}\n".to_string(),
            contracts: "### Action: Foo\n".to_string(),
            test_harness: String::new(),
        };
        let msg = user_message(&ctx, Language::Rust);

        assert!(!msg.contains("Contract Tests"));
    }

    #[test]
    fn test_retry_message_lists_errors() {
        let msg = retry_message(
            "fn main() {}",
            &["missing struct Foo".into(), "unbalanced braces".into()],
            Language::Rust,
        );
        assert!(msg.contains("1. missing struct Foo"));
        assert!(msg.contains("2. unbalanced braces"));
        assert!(msg.contains("fn main()"));
    }
}
