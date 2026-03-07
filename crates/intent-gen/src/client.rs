//! LLM API client for OpenAI-compatible endpoints.
//!
//! Supports any provider that implements the OpenAI chat completions API:
//! OpenAI, Anthropic (via proxy), Ollama, vLLM, etc.

use serde::{Deserialize, Serialize};

/// Configuration for the LLM API.
#[derive(Clone)]
pub struct ApiConfig {
    /// API key for authentication.
    pub api_key: String,
    /// Base URL for the API (e.g., "https://api.openai.com/v1").
    pub api_base: String,
    /// Model identifier (e.g., "gpt-4o", "claude-sonnet-4-20250514").
    pub model: String,
}

impl ApiConfig {
    /// Create config from environment variables.
    ///
    /// - `AI_API_KEY` — API key (required)
    /// - `AI_API_BASE` — Base URL (default: `https://api.openai.com/v1`)
    /// - `AI_MODEL` — Model name (default: `gpt-4o`)
    pub fn from_env() -> Result<Self, ConfigError> {
        let api_key =
            std::env::var("AI_API_KEY").map_err(|_| ConfigError::MissingKey("AI_API_KEY"))?;

        let api_base = std::env::var("AI_API_BASE")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        let model = std::env::var("AI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());

        Ok(Self {
            api_key,
            api_base,
            model,
        })
    }
}

/// Error creating API config.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing environment variable: {0}")]
    MissingKey(&'static str),
}

/// A message in the chat conversation.
#[derive(Clone, Serialize)]
pub(crate) struct Message {
    pub role: &'static str,
    pub content: String,
}

/// Client for sending chat completion requests.
pub struct LlmClient {
    config: ApiConfig,
    agent: ureq::Agent,
}

impl LlmClient {
    /// Create a new client with the given API config.
    pub fn new(config: ApiConfig) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(10))
            .timeout_read(std::time::Duration::from_secs(120))
            .build();
        Self { config, agent }
    }

    /// Create a client from environment variables.
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self::new(ApiConfig::from_env()?))
    }

    /// Override the model for this client.
    pub fn with_model(mut self, model: String) -> Self {
        self.config.model = model;
        self
    }

    /// Send a chat completion request and return the assistant's response text.
    pub(crate) fn chat(&self, messages: &[Message]) -> Result<String, ApiError> {
        let url = format!(
            "{}/chat/completions",
            self.config.api_base.trim_end_matches('/')
        );

        let body = ChatRequest {
            model: &self.config.model,
            messages,
            temperature: 0.2,
        };

        let response = self
            .agent
            .post(&url)
            .set("Authorization", &format!("Bearer {}", self.config.api_key))
            .set("Content-Type", "application/json")
            .send_json(serde_json::to_value(&body).map_err(ApiError::Serialization)?);

        let response = match response {
            Ok(r) => r,
            Err(ureq::Error::Status(status, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                return Err(ApiError::Http(status, body));
            }
            Err(ureq::Error::Transport(e)) => {
                return Err(ApiError::Transport(e.to_string()));
            }
        };

        let resp: ChatResponse = response
            .into_json()
            .map_err(|e| ApiError::Deserialization(e.to_string()))?;

        resp.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or(ApiError::EmptyResponse)
    }
}

/// Errors from LLM API calls.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("HTTP {0}: {1}")]
    Http(u16, String),
    #[error("transport error: {0}")]
    Transport(String),
    #[error("serialization error: {0}")]
    Serialization(#[source] serde_json::Error),
    #[error("deserialization error: {0}")]
    Deserialization(String),
    #[error("empty response from API")]
    EmptyResponse,
}

// ── Request/Response types ───────────────────────────────

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [Message],
    temperature: f32,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}
