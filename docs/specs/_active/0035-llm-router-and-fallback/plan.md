last audited 2026-06-27 by antigravity

# Configurable LLM routing chain and local Ollama load balancer

## Baseline
Currently, `LlmClient` uses a single hardcoded provider/model and a static fallback to OpenRouter that is not easily customizable via the environment. There is no support for a load-balanced Ollama server pool.

## Implementation Steps
1. **Extend Configuration Structure (`src/llm/models.rs`)**:
   - Add optional fields `fallback_enabled: Option<bool>` and `routing_chain: Option<Vec<String>>` to `LlmConfig`.
2. **Support Environment Variable Load (`src/llm/client/mod.rs`)**:
   - Update `apply_env_overrides` to parse `LLM_FALLBACK_ENABLED` (boolean) and `LLM_ROUTING_CHAIN` (comma-separated list of `provider:model` entries).
   - In `LlmClient`, parse `config.ollama.base_url` as a comma-separated list to support multiple load-balanced URLs, and store them as a `Vec<String>` with an atomic counter for round-robin routing.
3. **Refactor Chat Methods (`src/llm/client/fallback.rs`)**:
   - Modify `ollama_chat`, `openrouter_chat`, and `nvidia_chat` to accept an optional `model_override: Option<&str>`.
   - Update `chat()` and `chat_with_fallback()` to pass `None` under normal calls.
4. **Implement Dynamic Routing Chain (`src/llm/client/fallback.rs`)**:
   - Update `chat_with_fallback` to construct a sequence of target provider-model pairs from `routing_chain` (or default back to config/legacy behavior).
   - If `fallback_enabled` is false, truncate the sequence to only 1 element.
   - Run the sequence in order, catching errors and trying the next provider, returning the first successful completion.

## API Changes
No public breaking API changes. `chat_with_fallback` keeps its existing signature.

## Validation
- Add unit tests verifying `apply_env_overrides` correctly parses routing chains and fallback flags.
- Add unit tests verifying `chat_with_fallback` correctly loops through the routing chain and respects the fallback enable/disable flag.
- Run `cargo test llm::client` to verify.
