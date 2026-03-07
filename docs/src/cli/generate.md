# Generating Specs from Natural Language (Layer 0)

The `intent generate` command translates plain English descriptions into validated `.intent` specs using an AI model.

## Quick Start

```bash
# Set your API key
export AI_API_KEY=your-key-here

# Generate a spec from a description
intent generate "I want a program that greets a user by name"

# Write output to a file
intent generate --out hello.intent "a greeting service"
```

## Modes

### Single-shot (default)

Generates a spec, validates it, and outputs the result:

```bash
intent generate "a task tracker with priorities and assignments"
```

### Interactive

Ask clarifying questions before and during generation:

```bash
intent generate --interactive "build me a shopping cart"
```

### Edit existing spec

Modify an existing `.intent` file from a natural language description:

```bash
intent generate --edit cart.intent "add a discount code feature"
intent generate --edit cart.intent --diff "add rate limiting"  # show diff instead
```

## Confidence Levels

The `--confidence` flag (1-5) controls how much the agent asks vs. assumes:

| Level | Behavior |
|-------|----------|
| 1 | Always start interactive — ask clarifying questions before generating |
| 2 | Generate a draft, then ask "does this look right?" before finalizing |
| 3 (default) | Generate and auto-validate. Switch to interactive only if validation fails after retry |
| 4 | Generate, auto-validate, auto-retry. Only prompt if completely stuck |
| 5 | Single-shot. Output whatever the model returns (still validates, but won't retry or prompt) |

## Configuration

### Environment variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `AI_API_KEY` | API key for the LLM provider | (required) |
| `AI_API_BASE` | API base URL | OpenAI-compatible default |
| `AI_MODEL` | Default model name | (provider default) |

### CLI flags

| Flag | Purpose |
|------|---------|
| `--model <name>` | Override the model (e.g., `claude-sonnet-4-6`, `gpt-4o`) |
| `--out <file>` | Write to file instead of stdout |
| `--confidence <1-5>` | Set confidence level |
| `--interactive` | Force interactive mode |
| `--edit <file>` | Modify an existing spec |
| `--diff` | Show diff output (with `--edit`) |
| `--max-retries <N>` | Max validation retry attempts (default: 2) |

## How it works

1. Constructs a system prompt with IntentLang syntax reference and examples
2. Sends the user's description to the LLM via OpenAI-compatible API
3. Extracts the `.intent` code from the response
4. Validates via `intent check` (parser + semantic analysis)
5. If errors, feeds them back to the LLM for correction (up to `--max-retries`)
6. Outputs the validated spec

## Prompt preservation

The natural language prompt used to generate a spec should be committed to version control alongside the `.intent` file. This preserves the original ask so team members can understand the intent behind the spec.
