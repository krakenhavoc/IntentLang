//! LLM API client for OpenAI-compatible endpoints.
//!
//! Supports any provider that implements the OpenAI chat completions API:
//! OpenAI, Azure OpenAI, Anthropic (via proxy), Ollama, vLLM, etc.
//!
//! For Azure OpenAI managed deployments, set `AI_API_VERSION` to the API
//! version (e.g., `2025-01-01-preview`). The client auto-detects Azure
//! endpoints from the URL and uses `api-key` header auth.

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
    /// Azure API version (e.g., "2025-01-01-preview"). When set, enables
    /// Azure-style auth (`api-key` header) and appends the version as a
    /// query parameter.
    pub api_version: Option<String>,
}

impl ApiConfig {
    /// Create config from environment variables.
    ///
    /// - `AI_API_KEY` — API key (required)
    /// - `AI_API_BASE` — Base URL (default: `https://api.openai.com/v1`)
    /// - `AI_MODEL` — Model name (default: `gpt-4o`)
    /// - `AI_API_VERSION` — Azure API version (optional, e.g., `2025-01-01-preview`)
    pub fn from_env() -> Result<Self, ConfigError> {
        let api_key =
            std::env::var("AI_API_KEY").map_err(|_| ConfigError::MissingKey("AI_API_KEY"))?;

        let api_base = std::env::var("AI_API_BASE")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        let model = std::env::var("AI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());

        let api_version = std::env::var("AI_API_VERSION").ok();

        Ok(Self {
            api_key,
            api_base,
            model,
            api_version,
        })
    }

    /// Returns true if this config targets an Azure OpenAI endpoint.
    fn is_azure(&self) -> bool {
        self.api_version.is_some()
            || self.api_base.contains(".openai.azure.com")
            || self.api_base.contains(".cognitiveservices.azure.com")
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
pub struct Message {
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
    ///
    /// Read timeout defaults to 300s (5 min) but can be overridden via `AI_TIMEOUT` env var (seconds).
    pub fn new(config: ApiConfig) -> Self {
        let timeout_secs: u64 = std::env::var("AI_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300);
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(10))
            .timeout_read(std::time::Duration::from_secs(timeout_secs))
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
    pub fn chat(&self, messages: &[Message]) -> Result<String, ApiError> {
        let base = self.config.api_base.trim_end_matches('/');
        let url = if let Some(version) = &self.config.api_version {
            format!("{base}/chat/completions?api-version={version}")
        } else {
            format!("{base}/chat/completions")
        };

        let body = ChatRequest {
            model: &self.config.model,
            messages,
            temperature: 0.2,
        };

        let mut req = self
            .agent
            .post(&url)
            .set("Content-Type", "application/json");

        req = if self.config.is_azure() {
            req.set("api-key", &self.config.api_key)
        } else {
            req.set("Authorization", &format!("Bearer {}", self.config.api_key))
        };

        let response = req.send_json(serde_json::to_value(&body).map_err(ApiError::Serialization)?);

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
