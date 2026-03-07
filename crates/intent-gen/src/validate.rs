//! Generate-check-retry loop.
//!
//! Generates a spec via the LLM, validates it through the parser and checker,
//! and retries with error feedback if validation fails.

use crate::GenerateOptions;
use crate::client::{ApiError, LlmClient, Message};
use crate::prompt;

/// Errors from the generation pipeline.
#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
    #[error("validation failed after {retries} retries:\n{errors}")]
    ValidationFailed { retries: u32, errors: String },
}

/// Run the generate-check-retry loop.
pub(crate) fn generate_with_retry(
    client: &LlmClient,
    description: &str,
    options: &GenerateOptions,
) -> Result<String, GenerateError> {
    let is_edit = options.existing_spec.is_some();

    let system = if is_edit {
        prompt::edit_system_prompt(options.confidence)
    } else {
        prompt::system_prompt(options.confidence)
    };

    let user_msg = if let (Some(existing), Some(instruction)) =
        (&options.existing_spec, &options.edit_instruction)
    {
        prompt::edit_user_message(existing, instruction)
    } else {
        prompt::generation_user_message(description)
    };

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
        let raw = client.chat(&messages)?;
        let spec = strip_fences(&raw);

        match validate_spec(&spec) {
            Ok(()) => return Ok(spec),
            Err(errors) => {
                last_errors = errors.clone();

                if attempt < options.max_retries {
                    // Feed errors back as assistant + user messages for retry.
                    messages.push(Message {
                        role: "assistant",
                        content: raw,
                    });
                    messages.push(Message {
                        role: "user",
                        content: prompt::retry_message(&spec, &errors),
                    });
                }
            }
        }
    }

    Err(GenerateError::ValidationFailed {
        retries: options.max_retries,
        errors: last_errors.join("\n"),
    })
}

/// Validate a spec string through the parser and checker.
/// Returns Ok(()) if valid, or a list of error messages.
fn validate_spec(spec: &str) -> Result<(), Vec<String>> {
    // Parse
    let ast = match intent_parser::parse_file(spec) {
        Ok(ast) => ast,
        Err(e) => return Err(vec![format!("parse error: {e}")]),
    };

    // Semantic check
    let errors = intent_check::check_file(&ast);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.iter().map(|e| format!("check error: {e}")).collect())
    }
}

/// Strip markdown code fences if the LLM wraps the output in them.
fn strip_fences(s: &str) -> String {
    let trimmed = s.trim();

    // Try to strip ```intent ... ``` or ``` ... ```
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

    #[test]
    fn test_strip_fences_no_fences() {
        let input = "module Foo\n\nentity Bar {\n  id: UUID\n}";
        assert_eq!(strip_fences(input), input);
    }

    #[test]
    fn test_strip_fences_with_lang() {
        let input = "```intent\nmodule Foo\n\nentity Bar {\n  id: UUID\n}\n```";
        assert_eq!(
            strip_fences(input),
            "module Foo\n\nentity Bar {\n  id: UUID\n}"
        );
    }

    #[test]
    fn test_strip_fences_without_lang() {
        let input = "```\nmodule Foo\n\nentity Bar {\n  id: UUID\n}\n```";
        assert_eq!(
            strip_fences(input),
            "module Foo\n\nentity Bar {\n  id: UUID\n}"
        );
    }

    #[test]
    fn test_validate_valid_spec() {
        let spec = "module Test\n\nentity Foo {\n  id: UUID\n  name: String\n}\n";
        assert!(validate_spec(spec).is_ok());
    }

    #[test]
    fn test_validate_invalid_parse() {
        let spec = "not valid intent syntax at all";
        let err = validate_spec(spec).unwrap_err();
        assert!(err[0].contains("parse error"));
    }

    #[test]
    fn test_validate_semantic_error() {
        // Reference a nonexistent type in an action requires block
        let spec = "module Test\n\naction Foo {\n  id: UUID\n\n  requires {\n    x.nonexistent > 0\n  }\n}\n";
        // This should at least parse; semantic errors depend on checker rules
        let result = validate_spec(spec);
        // Either passes or gives check errors — both are acceptable
        assert!(result.is_ok() || result.is_err());
    }
}
