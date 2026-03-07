//! Natural language to IntentLang specification generation.
//!
//! This crate translates natural language descriptions into valid `.intent`
//! specifications using an LLM via an OpenAI-compatible API. It implements
//! a generate-check-retry loop: the LLM output is parsed and checked, and
//! if validation fails, errors are fed back to the LLM for correction.

mod client;
mod prompt;
mod validate;

pub use client::{ApiConfig, LlmClient};
pub use validate::GenerateError;

/// Options for spec generation.
pub struct GenerateOptions {
    /// Maximum number of validation retries (default: 2).
    pub max_retries: u32,
    /// Confidence level 1-5. Higher = agent assumes more, asks less.
    pub confidence: u8,
    /// Existing spec content for edit mode.
    pub existing_spec: Option<String>,
    /// Edit instruction (used with existing_spec).
    pub edit_instruction: Option<String>,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            max_retries: 2,
            confidence: 3,
            existing_spec: None,
            edit_instruction: None,
        }
    }
}

/// Generate an `.intent` specification from a natural language description.
///
/// Returns the validated `.intent` source text on success.
pub fn generate(
    client: &LlmClient,
    description: &str,
    options: &GenerateOptions,
) -> Result<String, GenerateError> {
    validate::generate_with_retry(client, description, options)
}
