# intent-gen

[![crates.io](https://img.shields.io/crates/v/intent-gen.svg)](https://crates.io/crates/intent-gen)
[![docs](https://img.shields.io/badge/docs-mdBook-blue)](https://krakenhavoc.github.io/IntentLang/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/krakenhavoc/IntentLang/blob/main/LICENSE)

Natural language to IntentLang spec generation via LLM (Layer 0) for [IntentLang](https://github.com/krakenhavoc/IntentLang).

Part of the IntentLang toolchain — a declarative specification language for human-AI collaboration.

## What it does

Translates natural language descriptions into valid `.intent` specification files using an OpenAI-compatible LLM API. Implements a generate-check-retry loop: the LLM output is parsed and validated, and if validation fails, errors are fed back to the LLM for correction.

- **Generation** — describe what you want in plain English, get a valid `.intent` spec
- **Editing** — modify existing specs with natural language instructions
- **Validation loop** — auto-validates output via parser/checker, retries with error feedback
- **Model-agnostic** — works with any OpenAI-compatible API (OpenAI, Anthropic, Ollama, vLLM)
- **Confidence levels** — 1 (conservative) to 5 (comprehensive) controls generation scope

## Usage

```rust
use intent_gen::{LlmClient, GenerateOptions, generate};

let client = LlmClient::from_env().unwrap();
let spec = generate(&client, "a user authentication system with login and logout", &GenerateOptions::default()).unwrap();
println!("{}", spec);
```

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `AI_API_KEY` | Yes | — | API key for the LLM provider |
| `AI_API_BASE` | No | `https://api.openai.com/v1` | Base URL for the API |
| `AI_MODEL` | No | `gpt-4o` | Model identifier |

## Modules

| Module | Description |
|--------|-------------|
| `client` | LLM API client (OpenAI-compatible chat completions) |
| `prompt` | System prompt construction with syntax reference |
| `validate` | Generate-check-retry loop with parser/checker feedback |

## Part of IntentLang

This crate is the natural language generation layer (Layer 0). Other crates in the workspace:

- **intent-parser** — PEG parser and typed AST
- **intent-check** — Semantic analysis and type checking
- **intent-render** — Markdown, HTML rendering, and formatting
- **intent-ir** — IR lowering, verification, audit bridge
- **intent-cli** — CLI binary (`intent generate`, `intent check`, etc.)
