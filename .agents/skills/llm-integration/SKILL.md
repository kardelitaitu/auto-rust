# LLM Integration

Comprehensive guide to the LLM integration system — provider configuration, model selection, reply strategies, rate limiting, retry logic, and fallback chains.

---

## Architecture

```
User Code → Llm (facade) → LlmClient → Provider (Ollama/OpenRouter/NVIDIA) → API
                      ↕
             UnifiedLLMProcessor
             (batch replies, quotes)
```

The flow:
1. `Llm` is the public facade — wraps `LlmClient`, exposes `generate()`, `chat()`, `chat_with_fallback()`, `health_check()`
2. `LlmClient` handles HTTP transport, rate limiting, provider-specific request formatting, retry/fallback, and response parsing
3. `UnifiedLLMProcessor` orchestrates batch reply generation and quote processing using `reply_engine` + `reply_strategies`
4. `processor.rs` contains pure parsing/sentiment functions (no async dependency) — used by `UnifiedLLMProcessor`

---

## File Map

| File | Purpose |
|---|---|
| `src/llm/mod.rs` | `Llm` facade — `generate()`, `chat()`, `chat_with_fallback()`, `health_check()` |
| `src/llm/models.rs` | All data types: `ChatMessage`, `ChatRequest`, `ChatResponse`, `LlmConfig`, `LlmProvider`, per-provider configs (`OllamaConfig`, `OpenRouterConfig`, `NvidiaConfig`), `Temperature`, `MaxTokens`, `Role`, `ChatChoice`, `OpenRouterResponse` |
| `src/llm/processor.rs` | Pure functions extracted from `unified_processor`: `clean_llm_json_response()`, `parse_batch_response_static()`, `clean_reply_content()`, `analyze_sentiment_from_text()`, sentiment indicators, confidence scoring — all testable without async |
| `src/llm/reply_engine.rs` | Twitter reply/quote system prompts (`reply_engine_system_prompt`, `quote_engine_system_prompt`), user prompt builders, persona system (5 personas), message builders (`build_reply_messages`, `build_quote_messages`) |
| `src/llm/reply_strategies.rs` | 32 reply strategies with weighted random selection, context boosts, `StrategyContext`, conversation type classification (`classify_conversation_type`), `sentiment_to_strategy_context()`, `build_reply_prompt()` |
| `src/llm/unified_processor.rs` | `UnifiedLLMProcessor` — async batch processing of up to 20 replies in one LLM request, quote-with-sentiment processing |
| `src/llm/client/mod.rs` | `LlmClient` setup, HTTP client, `SharedRateLimiter`, `apply_env_overrides()` for env var config, `create_llm_client_from_config()` |
| `src/llm/client/fallback.rs` | Provider dispatch (`chat()`), fallback logic (`chat_with_fallback()`), NVIDIA retry with exponential backoff + jitter, OpenRouter multi-model fallback chain, health checks, `strip_thinking_tags()` |
| `src/llm/client/tests.rs` | Extensive tests: env overrides, rate limiter, wiremock-based OpenRouter fallback chain tests (primary succeeds, chain through all, timeout, rate limit, server error, auth failure, malformed JSON) |
| `config/llm.toml` | Runtime LLM configuration — provider selection, per-provider model/url/timeout settings |

---

## Providers

### Supported Providers

| Provider | Config Struct | Default Model | Default URL | Notes |
|---|---|---|---|---|
| **Ollama** | `OllamaConfig` | `llama3.2:3b` | `http://localhost:11434` | Local; no API key needed; `num_predict` for max tokens |
| **OpenRouter** | `OpenRouterConfig` | `anthropic/claude-3-haiku` | `https://openrouter.ai/api/v1` | API key required; supports fallback model chain |
| **NVIDIA** | `NvidiaConfig` | `meta/llama-3.3-70b-instruct` | `https://integrate.api.nvidia.com/v1` | API key required; has `top_p`; supports thinking/reasoning templates |

### Provider Selection

1. Provider is set in `config/llm.toml` under `provider = "ollama" | "openrouter" | "nvidia"`
2. Overridden by `LLM_PROVIDER` env var (case-insensitive)
3. Defaults to `LlmProvider::Ollama` if neither is set

### Config Loading Flow

```
1. load_env_file() ← reads .env from CARGO_MANIFEST_DIR, sets missing env vars
2. Read config/llm.toml → deserialize into LlmConfig (or use LlmConfig::default())
3. apply_env_overrides(config, get_env) → overrides from env vars
4. Return final LlmConfig
```

---

## Environment Variables

### Provider Selection

| Env Var | Values | Example |
|---|---|---|
| `LLM_PROVIDER` | `"ollama"`, `"openrouter"`, `"nvidia"` | `LLM_PROVIDER=openrouter` |

### Ollama

| Env Var | Overrides | Example |
|---|---|---|
| `OLLAMA_URL` | `ollama.base_url` | `http://remote-host:11434` |
| `OLLAMA_MODEL` | `ollama.model` | `llama3.1:8b` |
| `OLLAMA_TEMPERATURE` | `ollama.temperature` | `0.8` |

### OpenRouter

| Env Var | Overrides | Example |
|---|---|---|
| `OPENROUTER_API_KEY` | `openrouter.api_key` | `sk-or-v1-...` |
| `OPENROUTER_MODEL` | `openrouter.model` | `gpt-4o` |
| `OPENROUTER_TEMPERATURE` | `openrouter.temperature` | `0.5` |
| `OPENROUTER_MODEL_FALLBACK` | `openrouter.fallback_models[0]` | `gpt-4o-mini` |
| `OPENROUTER_MODEL_FALLBACK_2` | `openrouter.fallback_models[1]` | `claude-3-haiku` |
| `OPENROUTER_MODEL_FALLBACK_3` | `openrouter.fallback_models[2]` | `gemini-1.5-flash` |
| `OPENROUTER_MODEL_FALLBACK_4` | `openrouter.fallback_models[3]` | `mistral-small` |

### NVIDIA

| Env Var | Overrides | Example |
|---|---|---|
| `NVIDIA_API_KEY` | `nvidia.api_key` | `nvapi-...` |
| `NVIDIA_MODEL` | `nvidia.model` | `meta/llama-3.1-8b` |
| `NVIDIA_BASE_URL` | `nvidia.base_url` | `https://custom.api.nvidia.com/v1` |
| `NVIDIA_TEMPERATURE` | `nvidia.temperature` | `1.0` |

### Generic (apply to all providers unless provider-specific set)

| Env Var | Applies To | Example |
|---|---|---|
| `LLM_TEMPERATURE` | All providers' temperature | `0.7` |
| `LLM_PRESENCE_PENALTY` | All providers' presence_penalty | `1.5` |
| `LLM_FREQUENCY_PENALTY` | All providers' frequency_penalty | `2.0` |

### Rate Limiting

| Env Var | Description |
|---|---|
| `LLM_RATE_LIMIT_CAPACITY` | Burst capacity (e.g., `10.0`) |
| `LLM_RATE_LIMIT_REFILL_RATE` | Tokens per second (e.g., `1.0`) |

---

## Rate Limiting

The `SharedRateLimiter` is a token-bucket rate limiter wrapped in `Arc<Mutex<...>>` for thread-safe sharing.

```rust
let limiter = SharedRateLimiter::new(capacity, refill_rate_per_sec);
limiter.acquire().await; // Blocks until a token is available
```

- Configured via `LLM_RATE_LIMIT_CAPACITY` and `LLM_RATE_LIMIT_REFILL_RATE` env vars
- Only active when both env vars are set AND > 0.0
- If not configured, rate limiter is `None` and all requests pass through
- Used by `chat_with_fallback()` before dispatching

---

## NVIDIA Retry Logic

NVIDIA API calls (`nvidia_chat()`) use retry with exponential backoff:

| Parameter | Value |
|---|---|
| **MAX_ATTEMPTS** | 3 |
| **Base delay** | 1s |
| **Multiplier** | 2x per attempt |
| **Max delay** | 60s |
| **Jitter** | ±25% using golden-angle pseudo-random from attempt number |

**Retryable status codes**: 429 (TOO_MANY_REQUESTS), 408 (REQUEST_TIMEOUT), 5xx (server errors)
**Retryable request errors**: timeout, connection errors

**Retry-After header**: If the server sends a `Retry-After` header, it takes priority (max of computed delay and server delay).

Empty responses are NOT retried — they're returned as errors with the `finish_reason`.

---

## OpenRouter Fallback Chain

OpenRouter's `openrouter_chat()` implements a multi-model fallback chain:

### Flow
```
Primary model → Fallback 1 → Fallback 2 → Fallback 3 → Fallback 4 → error
```

### Triggers for fallback
- **HTTP error**: 4xx/5xx status with API error in response body
- **Timeout**: request exceeds `timeout_ms` (default 60s)
- **Empty content**: response with empty `content` field
- **Parse error**: malformed JSON response

### Delay between fallback attempts
- ~1s computed delay (same exponential backoff as NVIDIA attempt 1)
- In test mode, delay is 1ms

### Fallback configuration
- Primary model: `OPENROUTER_MODEL` env var or `openrouter.model` in config
- Fallbacks: `OPENROUTER_MODEL_FALLBACK` through `_FALLBACK_4` env vars
- All models are tried sequentially; if all fail, the last error is returned with count of fallen-back models

---

## Ollama Chat

- Endpoint: `{base_url}/api/chat`
- Options: temperature, num_predict, presence_penalty, frequency_penalty
- Response parsed into `ChatResponse` struct
- Reasoning content logged at info level when present
- Default timeout: 120s (240s in LlmClient builder, overridden per-request)
- No retry logic (single attempt, fails fast)

---

## UnifiedLLMProcessor

The `UnifiedLLMProcessor` is a higher-level orchestrator for Twitter-specific LLM tasks:

### `process_replies_batch()`
- Generates replies for up to 20 tweets in a single LLM request
- Builds `StrategyContext` with conversation type classification
- Uses `build_reply_prompt()` with `batch_mode=true`
- Parses batch response using `processor::parse_batch_response()`
- Falls back to line-based parsing if JSON parsing fails

### `process_quote_with_sentiment()`
- Generates a single quote tweet with sentiment analysis
- Uses `build_quote_messages()` with persona and strategy context
- Returns `UnifiedQuoteResponse` with sentiment, content, and confidence
- Sentiment extracted from quote text via `processor::extract_sentiment_from_quote()`

---

## Reply Strategies

### 32 Strategies categorized:

| Category | Strategies |
|---|---|
| **Positive** | COMPLIMENT, HYPEMAN, HYPE_REPLY, SIMP, WHOLEsome, LOWKEY |
| **Personal** | NOSTALGIC, RELATABLE |
| **Humor** | WITTY, DRY_WIT, SARCASTIC, TROLL, NITPICK, UNHINGED |
| **Skepticism** | CONTRARIAN, CALLOUT, DISMISSIVE |
| **Expertise** | CLOUT, HOT_TAKE, HELPFUL |
| **Observation** | OBSERVATION, CURIOUS, QUESTION |
| **Short/Minimal** | MINIMALIST, SLANG, REACTION, CONFUSED |
| **Persona** | GEN_Z, BOOMER, NPC, ZEN, SMUG |

### Context Boosts

20 context keys (e.g., `"tech"`, `"humorous"`, `"politics"`, `"wholesome"`, `"critical"`, `"gaming"`, `"food"`, etc.) each boost 4-6 strategies with multipliers from 2x to 4x.

When multiple context keys match (e.g., sentiment + conversation type), the maximum multiplier for each strategy is used.

### Selection Algorithm

```
1. Build boost map from matching context keys
2. Apply boosts to base weights (all start at 1)
3. Weighted random pick from total weight sum
4. Return STRATEGY_INSTRUCTIONS matching the picked strategy
```

### Strategy Instructions

Each strategy has a `CRITICAL INSTRUCTION` that is injected into the prompt, including:
- Tone/persona guidance
- Format constraints (length, @mentions, emojis, hashtags)
- Explicit `NEVER write "Okay" or "Yes"` guardrail
- Banned AI-sounding words list

### Conversation Type Classification

`classify_conversation_type()` detects 9 topics from tweet text using keyword matching:
- tech (34 keywords), politics (20), gaming (20), food (23), science (20), finance (20), entertainment (22), news (9), debate (8)
- Case-insensitive matching
- Wins by keyword count (most keyword hits determines topic)
- Returns empty string if no keywords match

### Sentiment → Strategy Context

| Sentiment | Strategy Context |
|---|---|
| `Positive` | `"wholesome"` |
| `Neutral` | `""` (empty — no boosts) |
| `Negative` | `"critical"` |

---

## Persona System (`reply_engine.rs`)

5 Twitter personas for reply/quote generation:

| Persona | Style | Hashed session selection |
|---|---|---|
| `Default` | Opinionated, casual, assertive | hash % 5 == 0 |
| `GenZ` | Internet slang, high energy | hash % 5 == 1 |
| `Professional` | Insightful, articulate, logical | hash % 5 == 2 |
| `Satirical` | Dry humor, ironic, witty | hash % 5 == 3 |
| `Brief` | Minimal, 3-6 words | hash % 5 == 4 |

Persona is selected deterministically from session ID: `TwitterPersona::select_for_session(session_id)`.

---

## Thinking Tags Handling

The `strip_thinking_tags()` function removes `<think>...</think>` blocks from LLM responses:

- Removes complete blocks (`<think>...</think>`)
- Handles cutoff/unclosed blocks (`<think>...`) — removes everything from `<think>` onward
- Handles multiple blocks
- Trims resulting output

Used by both NVIDIA and OpenRouter response processing after content extraction.

---

## Adding a New LLM Provider

### 1. Add the provider variant

In `src/llm/models.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    #[default]
    Ollama,
    OpenRouter,
    Nvidia,
    NewProvider,  // Add here
}
```

### 2. Create provider config

In `src/llm/models.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NewProviderConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub timeout_ms: u64,
    pub temperature: Temperature,
    pub max_tokens: MaxTokens,
    // ...provider-specific params
}
```

Add to `LlmConfig`:
```rust
pub struct LlmConfig {
    pub provider: LlmProvider,
    pub ollama: OllamaConfig,
    pub openrouter: OpenRouterConfig,
    pub nvidia: NvidiaConfig,
    pub new_provider: NewProviderConfig,  // Add here
}
```

### 3. Add env var overrides

In `src/llm/client/mod.rs`, add to `apply_env_overrides()`:

```rust
if let Some(provider) = get_env("LLM_PROVIDER") {
    match provider.to_lowercase().as_str() {
        "newprovider" => config.provider = LlmProvider::NewProvider,
        // ...
    }
}

// Provider-specific overrides
if let Some(api_key) = get_env("NEWPROVIDER_API_KEY") {
    config.new_provider.api_key = api_key;
}
```

### 4. Implement the client method

In `src/llm/client/fallback.rs`, add the chat method:

```rust
async fn new_provider_chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
    // Build request, send HTTP POST, parse response, return content
}
```

### 5. Wire up dispatch

In `src/llm/client/fallback.rs`, update `chat()` and `chat_with_fallback()`:

```rust
pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
    match self.config.provider {
        LlmProvider::Ollama => self.ollama_chat(messages).await,
        LlmProvider::OpenRouter => self.openrouter_chat(messages).await,
        LlmProvider::Nvidia => self.nvidia_chat(messages).await,
        LlmProvider::NewProvider => self.new_provider_chat(messages).await,
    }
}
```

And add a health check method in the `health_check` / `health_check_result` dispatch.

### 6. Add config to `config/llm.toml`

```toml
[new_provider]
api_key = ""
base_url = "https://api.newprovider.com/v1"
model = "default-model"
timeout_ms = 60000
```

### 7. Add tests

- Env var override tests in `tests.rs`
- Wiremock integration tests (primary succeeds, fallback chain, timeouts, errors)

---

## Pitfalls

| # | Pitfall | Explanation |
|---|---|---|
| 1 | **Env vars not applied** | `apply_env_overrides()` is called during config creation, but only if `create_llm_client_from_config()` is used. Direct `LlmConfig::default()` skips env overrides. |
| 2 | **Ollama model name mismatch** | Ollama uses local model names (e.g., `llama3.2:3b`) while OpenRouter uses provider/model format (e.g., `anthropic/claude-3-haiku`). These are NOT interchangeable. |
| 3 | **OpenRouter fallback delay** | The delay between fallback attempts is ~1s (NVIDIA retry delay). With 4 fallbacks, total fallback time could be ~4s. |
| 4 | **Rate limiter not active by default** | Rate limiter is `None` unless BOTH `LLM_RATE_LIMIT_CAPACITY` and `LLM_RATE_LIMIT_REFILL_RATE` are set. No rate limiting = potential for rate limit errors from APIs. |
| 5 | **NVIDIA empty response** | Empty responses from NVIDIA are NOT retried — they're errors with `finish_reason`. This is intentional because empty responses usually indicate content filtering or context overflow. |
| 6 | **Ollama no retry** | Ollama has ZERO retry logic — a single failure fails immediately. Unlike NVIDIA (3 retries) and OpenRouter (fallback chain). |
| 7 | **Strip thinking tags after extraction** | `strip_thinking_tags()` is called AFTER extracting content from the response. If you process the raw response before this, thinking blocks may appear in output. |
| 8 | **Persona determinism** | `TwitterPersona::select_for_session()` uses byte hash. Adding or removing characters from session IDs changes persona selection. |
| 9 | **Strategy context boosts max, not sum** | When multiple context keys boost the same strategy, the MAX multiplier is used, not the sum. A strategy boosted to 3x by one context and 4x by another gets 4x, not 7x. |
| 10 | **Conversation type classification is keyword-only** | It's a flat keyword match — no NLP, no semantic understanding. A tweet containing "I love Python for cooking" would classify as "tech" (Python) not "food". Wins by keyword count. |

---

## Testing

| Test Location | Command |
|---|---|
| LLM unit tests | `cargo test --lib llm::` |
| Processor proptests | `cargo test --lib processor::fuzz_tests` |
| Reply engine tests | `cargo test --lib reply_engine::tests` |
| Reply strategies tests | `cargo test --lib reply_strategies::tests` |
| Client tests | `cargo test --lib llm::client::tests` |
| Env override tests | `cargo test --lib apply_env_` |
| Wiremock integration | `cargo test --lib openrouter_fallback_` |
| Unified processor (requires LLM) | `cargo test --lib unified_processor::tests -- --ignored --nocapture` |

Notable: Unified processor tests (`test_process_replies_batch`, `test_process_quote_with_sentiment`) are **not** marked `#[ignore]` but skip gracefully if LLM config is unavailable via early return.
