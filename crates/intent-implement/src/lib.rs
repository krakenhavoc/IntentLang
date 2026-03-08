//! AI-powered full implementation generation from IntentLang specifications.
//!
//! This crate translates parsed `.intent` specs into complete, working
//! implementations using an LLM. It builds on `intent-codegen` (skeleton stubs)
//! and `intent-gen` (LLM client) to produce contract-aware code.
//!
//! The generation pipeline:
//! 1. Format the spec, generate skeleton code, extract contracts
//! 2. Send to LLM with implementation-focused system prompt
//! 3. Validate the output (expected names, balanced delimiters, no stubs)
//! 4. Retry with error feedback if validation fails

pub mod context;
pub mod prompt;
pub mod validate;

pub use intent_codegen::Language;
pub use validate::ImplementError;

/// Options for implementation generation.
pub struct ImplementOptions {
    /// Target language.
    pub language: Language,
    /// Maximum number of validation retries (default: 2).
    pub max_retries: u32,
    /// Print raw LLM responses to stderr for debugging.
    pub debug: bool,
}

impl Default for ImplementOptions {
    fn default() -> Self {
        Self {
            language: Language::Rust,
            max_retries: 2,
            debug: false,
        }
    }
}

/// Generate a full implementation from a parsed `.intent` AST.
///
/// Returns the complete source code for the target language on success.
pub fn implement(
    client: &intent_gen::LlmClient,
    file: &intent_parser::ast::File,
    options: &ImplementOptions,
) -> Result<String, ImplementError> {
    validate::implement_with_retry(client, file, options)
}
