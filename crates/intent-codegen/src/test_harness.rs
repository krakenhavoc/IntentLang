//! Contract test harness generator.
//!
//! Translates `test` blocks from IntentLang specs into executable test
//! modules in the target language. The generated tests verify that an
//! implementation honors the spec's contracts (requires/ensures).

use intent_parser::ast;

use crate::Language;

/// Generate a contract test harness from a parsed intent file.
///
/// Returns a test module string (e.g., `#[cfg(test)] mod contract_tests { ... }`
/// for Rust), or an empty string if the spec contains no test blocks.
pub fn generate(file: &ast::File, lang: Language) -> String {
    match lang {
        Language::Rust => crate::rust_tests::generate(file),
        _ => String::new(), // not yet supported
    }
}

/// List the expected test function names that the harness generates.
///
/// Used by `intent-implement` to validate that the LLM output includes
/// all contract tests.
pub fn expected_test_names(file: &ast::File) -> Vec<String> {
    file.items
        .iter()
        .filter_map(|i| match i {
            ast::TopLevelItem::Test(t) => Some(format!("test_{}", slugify(&t.name))),
            _ => None,
        })
        .collect()
}

/// Slugify a test name for use as a function name.
/// "successful transfer" -> "successful_transfer"
pub fn slugify(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("successful transfer"), "successful_transfer");
        assert_eq!(
            slugify("frozen account rejected"),
            "frozen_account_rejected"
        );
        assert_eq!(slugify("  spaces  and  gaps  "), "spaces_and_gaps");
        assert_eq!(slugify("CamelCase Name"), "camelcase_name");
    }

    #[test]
    fn test_expected_test_names() {
        let src = r#"module Test

entity Foo { id: UUID }

action Bar { x: Int }

test "happy path" {
  given { x = 42 }
  when Bar { x: x }
  then { x == 42 }
}

test "sad path" {
  given { x = 0 }
  when Bar { x: x }
  then fails
}
"#;
        let file = intent_parser::parse_file(src).expect("parse");
        let names = expected_test_names(&file);
        assert_eq!(names, vec!["test_happy_path", "test_sad_path"]);
    }

    #[test]
    fn test_empty_for_no_tests() {
        let src = "module Test\n\nentity Foo { id: UUID }\n";
        let file = intent_parser::parse_file(src).expect("parse");
        assert!(generate(&file, Language::Rust).is_empty());
    }
}
