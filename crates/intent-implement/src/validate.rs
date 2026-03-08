//! Output validation and generate-check-retry loop.
//!
//! Validates LLM-generated code for structural correctness (expected names
//! present, balanced delimiters, no leftover stubs) and retries on failure.

use intent_codegen::Language;
use intent_gen::{ApiError, LlmClient, Message};
use intent_parser::ast;

use crate::ImplementOptions;
use crate::context;
use crate::prompt;

/// Errors from the implementation pipeline.
#[derive(Debug, thiserror::Error)]
pub enum ImplementError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
    #[error("validation failed after {retries} retries:\n{errors}")]
    ValidationFailed { retries: u32, errors: String },
}

/// Run the implement-check-retry loop.
pub fn implement_with_retry(
    client: &LlmClient,
    file: &ast::File,
    options: &ImplementOptions,
) -> Result<String, ImplementError> {
    let ctx = context::build_context(file, options.language);
    let system = prompt::system_prompt(options.language);
    let user_msg = prompt::user_message(&ctx, options.language);

    let mut messages = vec![
        Message {
            role: "system",
            content: system,
        },
        Message {
            role: "user",
            content: user_msg,
        },
    ];

    let mut last_errors = Vec::new();

    for attempt in 0..=options.max_retries {
        if attempt == 0 {
            eprintln!("Generating implementation from LLM...");
        } else {
            eprintln!(
                "Retry {}/{}: feeding errors back to LLM...",
                attempt, options.max_retries
            );
        }

        let raw = client.chat(&messages)?;
        if options.debug {
            eprintln!("--- RAW LLM RESPONSE ---");
            eprintln!("{raw}");
            eprintln!("--- END RAW RESPONSE ---");
        }

        let code = strip_fences(&raw);
        eprintln!("Validating generated code...");

        match validate_output(&code, file, options.language) {
            Ok(()) => {
                eprintln!("Validation passed.");
                return Ok(code);
            }
            Err(errors) => {
                for e in &errors {
                    eprintln!("  {e}");
                }
                last_errors.clone_from(&errors);

                if attempt < options.max_retries {
                    messages.push(Message {
                        role: "assistant",
                        content: raw,
                    });
                    messages.push(Message {
                        role: "user",
                        content: prompt::retry_message(&code, &errors, options.language),
                    });
                }
            }
        }
    }

    Err(ImplementError::ValidationFailed {
        retries: options.max_retries,
        errors: last_errors.join("\n"),
    })
}

/// Validate generated code for structural correctness.
///
/// Checks:
/// 1. Expected entity/action names are present
/// 2. Delimiters are balanced
/// 3. No leftover `todo!()` / `throw "not implemented"` / `raise NotImplementedError` stubs
/// 4. Contract test functions are present (if spec has test blocks)
pub fn validate_output(code: &str, file: &ast::File, lang: Language) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    // Check expected names
    let expected = expected_names(file, lang);
    for name in &expected {
        if !code.contains(name.as_str()) {
            errors.push(format!("missing expected identifier: {name}"));
        }
    }

    // Check balanced delimiters
    if let Err(e) = check_balanced(code, lang) {
        errors.push(e);
    }

    // Check for leftover stubs
    let stubs = leftover_stubs(code, lang);
    for stub in stubs {
        errors.push(format!("leftover stub found: {stub}"));
    }

    // Check that contract test functions are present
    let test_names = intent_codegen::test_harness::expected_test_names(file);
    for name in &test_names {
        if !code.contains(name.as_str()) {
            errors.push(format!("missing contract test: {name}"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Extract the names we expect to find in the generated code.
fn expected_names(file: &ast::File, lang: Language) -> Vec<String> {
    let mut names = Vec::new();

    for item in &file.items {
        match item {
            ast::TopLevelItem::Entity(e) => {
                names.push(e.name.clone());
            }
            ast::TopLevelItem::Action(a) => {
                // Actions become functions with language-appropriate naming
                let fn_name = match lang {
                    Language::Rust | Language::Python | Language::Go => {
                        intent_codegen::to_snake_case(&a.name)
                    }
                    Language::TypeScript => intent_codegen::to_camel_case(&a.name),
                };
                names.push(fn_name);
            }
            _ => {}
        }
    }

    names
}

/// Check that delimiters are balanced in the code.
fn check_balanced(code: &str, lang: Language) -> Result<(), String> {
    let (braces, parens, brackets) = count_delimiters(code, lang);

    if braces != 0 {
        return Err(format!(
            "unbalanced braces: {} more {} than {}",
            braces.unsigned_abs(),
            if braces > 0 { "opening" } else { "closing" },
            if braces > 0 { "closing" } else { "opening" }
        ));
    }
    if parens != 0 {
        return Err(format!(
            "unbalanced parentheses: {} more {} than {}",
            parens.unsigned_abs(),
            if parens > 0 { "opening" } else { "closing" },
            if parens > 0 { "closing" } else { "opening" }
        ));
    }
    if brackets != 0 {
        return Err(format!(
            "unbalanced brackets: {} more {} than {}",
            brackets.unsigned_abs(),
            if brackets > 0 { "opening" } else { "closing" },
            if brackets > 0 { "closing" } else { "opening" }
        ));
    }
    Ok(())
}

/// Count net delimiters in code, skipping strings and comments.
fn count_delimiters(code: &str, lang: Language) -> (i32, i32, i32) {
    let mut braces = 0i32;
    let mut parens = 0i32;
    let mut brackets = 0i32;

    for line in code.lines() {
        let line = strip_comment(line, lang);
        let mut in_string = false;
        let mut escape_next = false;

        for ch in line.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }
            if ch == '\\' && in_string {
                escape_next = true;
                continue;
            }
            if ch == '"' {
                in_string = !in_string;
                continue;
            }
            // In Rust, ' is for lifetimes ('a, '_, 'static) — not strings.
            // Treating it as a string toggle causes false positives on lines
            // like `Formatter<'_>) -> Result {` where the trailing { is missed.
            // In Go, ' is for rune literals which are short and self-closing.
            // Only Python and TypeScript use ' as a string delimiter.
            if ch == '\'' && matches!(lang, Language::Python | Language::TypeScript) {
                in_string = !in_string;
                continue;
            }
            if in_string {
                continue;
            }

            match ch {
                '{' => braces += 1,
                '}' => braces -= 1,
                '(' => parens += 1,
                ')' => parens -= 1,
                '[' => brackets += 1,
                ']' => brackets -= 1,
                _ => {}
            }
        }
    }

    (braces, parens, brackets)
}

/// Strip single-line comments from a line.
fn strip_comment(line: &str, lang: Language) -> &str {
    match lang {
        Language::Rust | Language::TypeScript | Language::Go => {
            // Find // outside of strings
            let mut in_string = false;
            let mut prev = '\0';
            for (i, ch) in line.char_indices() {
                if ch == '"' && prev != '\\' {
                    in_string = !in_string;
                }
                if !in_string && ch == '/' && prev == '/' {
                    return &line[..i - 1];
                }
                prev = ch;
            }
            line
        }
        Language::Python => {
            let mut in_string = false;
            let mut prev = '\0';
            for (i, ch) in line.char_indices() {
                if (ch == '"' || ch == '\'') && prev != '\\' {
                    in_string = !in_string;
                }
                if !in_string && ch == '#' {
                    return &line[..i];
                }
                prev = ch;
            }
            line
        }
    }
}

/// Check for leftover implementation stubs.
fn leftover_stubs(code: &str, lang: Language) -> Vec<String> {
    let mut stubs = Vec::new();

    match lang {
        Language::Rust => {
            if code.contains("todo!()") {
                stubs.push("todo!()".to_string());
            }
            if code.contains("unimplemented!()") {
                stubs.push("unimplemented!()".to_string());
            }
        }
        Language::TypeScript => {
            if code.contains("throw new Error(\"not implemented\")")
                || code.contains("throw new Error(\"Not implemented\")")
            {
                stubs.push("throw new Error(\"not implemented\")".to_string());
            }
        }
        Language::Python => {
            if code.contains("raise NotImplementedError") {
                stubs.push("raise NotImplementedError".to_string());
            }
        }
        Language::Go => {
            if code.contains("panic(\"not implemented\")") || code.contains("panic(\"TODO\")") {
                stubs.push("panic(\"not implemented\")".to_string());
            }
        }
    }

    stubs
}

/// Strip markdown code fences if the LLM wraps the output in them.
pub fn strip_fences(s: &str) -> String {
    let trimmed = s.trim();

    if let Some(rest) = trimmed.strip_prefix("```") {
        // Skip optional language tag on the first line
        let rest = if let Some(idx) = rest.find('\n') {
            &rest[idx + 1..]
        } else {
            rest
        };
        if let Some(content) = rest.strip_suffix("```") {
            return content.trim().to_string();
        }
    }

    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ast::File {
        intent_parser::parse_file(src).expect("parse failed")
    }

    // ── strip_fences ───────────────────────────────────────

    #[test]
    fn test_strip_fences_no_fences() {
        let input = "fn main() {}";
        assert_eq!(strip_fences(input), input);
    }

    #[test]
    fn test_strip_fences_with_lang() {
        let input = "```rust\nfn main() {}\n```";
        assert_eq!(strip_fences(input), "fn main() {}");
    }

    #[test]
    fn test_strip_fences_without_lang() {
        let input = "```\nfn main() {}\n```";
        assert_eq!(strip_fences(input), "fn main() {}");
    }

    // ── validate_output ────────────────────────────────────

    #[test]
    fn test_validate_valid_rust() {
        let src =
            "module Test\n\nentity Foo {\n  id: UUID\n}\n\naction CreateFoo {\n  name: String\n}\n";
        let ast = parse(src);
        let code = "struct Foo { id: String }\n\nfn create_foo(name: &str) -> Foo {\n    Foo { id: name.to_string() }\n}\n";
        assert!(validate_output(code, &ast, Language::Rust).is_ok());
    }

    #[test]
    fn test_validate_missing_name() {
        let src =
            "module Test\n\nentity Foo {\n  id: UUID\n}\n\naction CreateFoo {\n  name: String\n}\n";
        let ast = parse(src);
        let code = "struct Foo { id: String }\n// function not defined\n";
        let err = validate_output(code, &ast, Language::Rust).unwrap_err();
        assert!(err.iter().any(|e| e.contains("create_foo")));
    }

    #[test]
    fn test_validate_leftover_todo() {
        let src =
            "module Test\n\nentity Foo {\n  id: UUID\n}\n\naction CreateFoo {\n  name: String\n}\n";
        let ast = parse(src);
        let code =
            "struct Foo { id: String }\n\nfn create_foo(name: &str) -> Foo {\n    todo!()\n}\n";
        let err = validate_output(code, &ast, Language::Rust).unwrap_err();
        assert!(err.iter().any(|e| e.contains("todo!()")));
    }

    #[test]
    fn test_validate_unbalanced_braces() {
        let src = "module Test\n\nentity Foo {\n  id: UUID\n}\n";
        let ast = parse(src);
        let code = "struct Foo { id: String\n";
        let err = validate_output(code, &ast, Language::Rust).unwrap_err();
        assert!(err.iter().any(|e| e.contains("unbalanced")));
    }

    // ── TypeScript validation ──────────────────────────────

    #[test]
    fn test_validate_valid_typescript() {
        let src =
            "module Test\n\nentity Foo {\n  id: UUID\n}\n\naction CreateFoo {\n  name: String\n}\n";
        let ast = parse(src);
        let code = "interface Foo { id: string; }\n\nfunction createFoo(name: string): Foo {\n    return { id: name };\n}\n";
        assert!(validate_output(code, &ast, Language::TypeScript).is_ok());
    }

    // ── Python validation ──────────────────────────────────

    #[test]
    fn test_validate_valid_python() {
        let src =
            "module Test\n\nentity Foo {\n  id: UUID\n}\n\naction CreateFoo {\n  name: String\n}\n";
        let ast = parse(src);
        let code = "from dataclasses import dataclass\n\n@dataclass\nclass Foo:\n    id: str\n\ndef create_foo(name: str) -> Foo:\n    return Foo(id=name)\n";
        assert!(validate_output(code, &ast, Language::Python).is_ok());
    }

    #[test]
    fn test_validate_python_leftover_raise() {
        let src =
            "module Test\n\nentity Foo {\n  id: UUID\n}\n\naction CreateFoo {\n  name: String\n}\n";
        let ast = parse(src);
        let code = "class Foo:\n    id: str\n\ndef create_foo(name: str) -> Foo:\n    raise NotImplementedError\n";
        let err = validate_output(code, &ast, Language::Python).unwrap_err();
        assert!(err.iter().any(|e| e.contains("NotImplementedError")));
    }

    // ── expected_names ─────────────────────────────────────

    #[test]
    fn test_expected_names_rust() {
        let src = "module Test\n\nentity Account {\n  id: UUID\n}\n\naction FreezeAccount {\n  id: UUID\n}\n";
        let ast = parse(src);
        let names = expected_names(&ast, Language::Rust);
        assert!(names.contains(&"Account".to_string()));
        assert!(names.contains(&"freeze_account".to_string()));
    }

    #[test]
    fn test_expected_names_typescript() {
        let src = "module Test\n\nentity Account {\n  id: UUID\n}\n\naction FreezeAccount {\n  id: UUID\n}\n";
        let ast = parse(src);
        let names = expected_names(&ast, Language::TypeScript);
        assert!(names.contains(&"Account".to_string()));
        assert!(names.contains(&"freezeAccount".to_string()));
    }

    // ── delimiter counting ─────────────────────────────────

    #[test]
    fn test_balanced_delimiters() {
        let code = "fn foo() { let x = (1 + 2); let arr = [1, 2, 3]; }";
        assert!(check_balanced(code, Language::Rust).is_ok());
    }

    #[test]
    fn test_delimiters_in_strings_ignored() {
        let code = "let s = \"({[\"; let t = \"]})\";";
        // Strings contain unbalanced delimiters but they should be ignored
        let (b, p, br) = count_delimiters(code, Language::Rust);
        assert_eq!(b, 0);
        assert_eq!(p, 0);
        assert_eq!(br, 0);
    }

    #[test]
    fn test_rust_lifetimes_not_treated_as_strings() {
        // Rust lifetimes ('a, '_, 'static) must not toggle string mode,
        // otherwise the { at the end of lines like this gets skipped.
        let code = "impl Foo {\n    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n        Ok(())\n    }\n}";
        assert!(check_balanced(code, Language::Rust).is_ok());
    }

    #[test]
    fn test_python_single_quote_strings() {
        let code = "x = '({['\ny = ']})'\n";
        let (b, p, br) = count_delimiters(code, Language::Python);
        assert_eq!(b, 0);
        assert_eq!(p, 0);
        assert_eq!(br, 0);
    }

    // ── contract test validation ──────────────────────────────

    #[test]
    fn test_validate_missing_contract_test() {
        let src = r#"module Test

entity Foo { id: UUID }

action Bar { x: Int }

test "happy path" {
  given { x = 42 }
  when Bar { x: x }
  then { x == 42 }
}
"#;
        let ast = parse(src);
        // Code has entity + action but missing the contract test function
        let code = "struct Foo { id: String }\n\nfn bar(x: i64) -> Result<(), String> { Ok(()) }\n";
        let err = validate_output(code, &ast, Language::Rust).unwrap_err();
        assert!(
            err.iter()
                .any(|e| e.contains("missing contract test: test_happy_path"))
        );
    }

    #[test]
    fn test_validate_with_contract_test_present() {
        let src = r#"module Test

entity Foo { id: UUID }

action Bar { x: Int }

test "happy path" {
  given { x = 42 }
  when Bar { x: x }
  then { x == 42 }
}
"#;
        let ast = parse(src);
        let code = "struct Foo { id: String }\n\nfn bar(x: i64) -> Result<(), String> { Ok(()) }\n\n#[cfg(test)]\nmod contract_tests {\n    use super::*;\n    #[test]\n    fn test_happy_path() { assert!(true); }\n}\n";
        assert!(validate_output(code, &ast, Language::Rust).is_ok());
    }
}
