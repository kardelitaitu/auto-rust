# Configuration System

Teaches agents how the Auto-Rust configuration system works — the 3-layer overlay (TOML → .env → explicit env vars), config struct types, validation, CLI parsing, and how to add new config options.

## Architecture Overview

Config is resolved in this priority order (highest wins):
```
1. Explicit env vars (set before launch) ← highest priority
2. .env file values (not overridden by explicit env vars)
3. config/default.toml (parsed with serde)
4. Hardcoded Rust defaults (in types.rs default_*() functions)
```

```
load_config():
  load_dotenv_defaults()     → reads .env into env (only if not already set)
  if config/default.toml exists:
    parse TOML → Config struct
    apply_env_overrides()    → overlay explicit env vars on top
  else fallback:
    load_code_config()       → hardcoded defaults + ROXYBROWSER_* env vars
```

## File Map

| File | Purpose |
|---|---|
| `src/config/mod.rs` | Entry point: `load_config()`, `validate_config()`, `ConfigValidationReport` |
| `src/config/types.rs` | All config struct definitions + serde default functions |
| `src/config/env.rs` | `.env` loader + `apply_env_overrides()` + `load_code_config()` fallback |
| `src/config/defaults.rs` | `Default` impls for structs that reference sibling types |
| `src/config/validation.rs` | `Validate` trait, range/boundary checks, hex color validation |
| `src/config/tests.rs` | ~3,500+ lines of integration and unit tests |
| `src/cli/mod.rs` | CLI `Args` struct (clap), help/formatting, task lookup |
| `src/cli/parser.rs` | Task group parsing (`then` separator), browser filters, URL formatting |
| `config/default.toml` | Production config file in TOML format |
| `.env.example` | Example/template env file with all available vars |

## Config Struct Hierarchy

```
Config
├── browser: BrowserConfig
│   ├── connection_timeout_ms: DurationMs
│   ├── max_discovery_retries: u32
│   ├── discovery_retry_delay_ms: DurationMs
│   ├── circuit_breaker: CircuitBreakerConfig
│   │   ├── enabled: bool
│   │   ├── failure_threshold: u32
│   │   ├── success_threshold: u32
│   │   └── half_open_time_ms: DurationMs
│   ├── profiles: Vec<BrowserProfile>
│   │   └── name, type, ws_endpoint: String
│   ├── roxybrowser: RoxybrowserConfig
│   │   ├── enabled: bool
│   │   ├── api_url: String
│   │   └── api_key: String
│   ├── user_agent: Option<String>
│   ├── extra_http_headers: BTreeMap<String, String>
│   ├── cursor_overlay_ms: u64
│   ├── cursor_overlay_color: String
│   ├── cursor_overlay_show_trail: bool
│   ├── native_interaction: NativeInteractionConfig
│   │   ├── calibration_mode: NativeClickCalibrationMode [Windows|Mac|Linux]
│   │   ├── native_input_backend: NativeInputBackend [Enigo|Sendinput|Rdev]
│   │   ├── stability_wait_ms: DurationMs
│   │   ├── resolve_timeout_ms: DurationMs
│   │   └── settle_ms: u64
│   ├── max_workers_per_session: usize
│   ├── enable_learning_persistence: bool
│   └── learning_ttl_days: u32
├── orchestrator: OrchestratorConfig
│   ├── max_global_concurrency: usize
│   ├── task_timeout_ms: DurationMs
│   ├── group_timeout_ms: DurationMs
│   ├── worker_wait_timeout_ms: DurationMs
│   ├── task_stagger_delay_ms: u64
│   ├── max_retries: u32
│   └── retry_delay_ms: DurationMs
├── twitter_activity: TwitterActivityConfig
│   ├── feed_scan_duration_ms: DurationMs
│   ├── feed_scroll_count: u32
│   ├── engagement_candidate_count: u32
│   ├── scroll_amount_pixels: i32
│   ├── candidate_scan_interval_ms: u64
│   ├── max_consecutive_scroll_failures: u32
│   ├── max_consecutive_empty_scans: u32
│   ├── persona_file_path: Option<String>
│   ├── probabilities: TwitterProbabilitiesConfig
│   │   ├── like_probability: f64
│   │   ├── retweet_probability: f64
│   │   ├── quote_probability: f64
│   │   ├── follow_probability: f64
│   │   ├── reply_probability: f64
│   │   ├── bookmark_probability: f64
│   │   └── thread_dive_probability: f64
│   ├── engagement_limits: EngagementLimitsConfig
│   │   ├── max_likes: u32
│   │   ├── max_retweets: u32
│   │   ├── max_follows: u32
│   │   ├── max_replies: u32
│   │   ├── max_thread_dives: u32
│   │   ├── max_bookmarks: u32
│   │   ├── max_quote_tweets: u32
│   │   └── max_total_actions: u32
│   ├── llm: TwitterLLMConfig
│   │   ├── enabled: bool
│   │   ├── provider, model: String
│   │   ├── temperature: f64
│   │   ├── max_tokens: u32
│   │   ├── timeout_ms: u64
│   │   ├── reply_probability, quote_tweet_probability: f64
│   │   └── [Default: enabled=false, provider="", model=""]
│   └── persistence_enabled: bool
├── tracing: TracingConfig
│   ├── enabled: bool
│   ├── otlp_endpoint: String
│   └── service_name: String
└── task_discovery: TaskDiscoveryConfig
    ├── enabled: bool
    ├── roots: Vec<String>
    └── extensions: Vec<String> [default: ["task"]]
```

## Adding a New Config Option

### Step 1: Add the field to the struct in `types.rs`

```rust
// In the appropriate struct (BrowserConfig, OrchestratorConfig, etc.)
#[derive(Debug, Deserialize, Clone)]
pub struct SomeConfig {
    // existing fields...
    #[serde(default = "default_new_field")]
    pub new_field: u32,
}

fn default_new_field() -> u32 {
    42
}
```

**Patterns for serde defaults:**
- `#[serde(default)]` → uses `Default::default()` for the type
- `#[serde(default = "function_name")]` → calls the named function
- `#[serde(default = "default_feed_scan_duration")]` → `pub(crate)` functions in `types.rs`

### Step 2: Add default function in `types.rs`

If the field has a custom default value, add a `pub(crate) fn` or private `fn` in `types.rs`:

```rust
pub(crate) fn default_my_timing_ms() -> DurationMs {
    DurationMs::new_const(5_000)
}
```

### Step 3: Update Default impl in `defaults.rs` (if needed)

If the containing struct has a `Manual Default impl` (not `#[derive(Default)]`), add the field there:

```rust
impl Default for SomeConfig {
    fn default() -> Self {
        Self {
            new_field: super::types::default_new_field(),
            // ...
        }
    }
}
```

### Step 4: Add env var override in `env.rs`

Add the override in `apply_env_overrides()`:

```rust
// Parse simple numeric type
if let Ok(val) = env::var("MY_NEW_ENV_VAR") {
    config.some_config.new_field = val
        .parse()
        .unwrap_or(config.some_config.new_field);
}

// Parse DurationMs type
if let Ok(val) = env::var("MY_DURATION_MS") {
    config.some_config.duration_field = val
        .parse::<u64>()
        .ok()
        .and_then(DurationMs::new)
        .unwrap_or(config.some_config.duration_field);
}

// Parse float probability (with comment-stripping)
parse_env_float(
    "MY_PROBABILITY",
    0.05,  // default fallback
    &mut config.some_config.prob_field,
);
```

### Step 5: Add TOML fields to `config/default.toml`

```toml
[some_section]
new_field = 42
# NEW_ENV_VAR=42
```

### Step 6: Add to `.env.example`

```
MY_NEW_ENV_VAR=42
```

### Step 7: (Optional) Add validation in `validation.rs`

In `validate_orchestrator_config()`, `validate_browser_config()`, or `validate_twitter_activity_config()`:

```rust
if config.new_field == 0 {
    return Err(OrchestratorError::Config(ConfigError::InvalidValue {
        field: "section.new_field".to_string(),
        value: config.new_field.to_string(),
        reason: "must be > 0".to_string(),
    }));
}
if config.new_field > 100 {
    warn!("section.new_field ({}) is very high.", config.new_field);
}
```

Also add the `Validate` trait check (strict validation, not advisory):

```rust
impl Validate for SomeConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        // hard bounds that should reject invalid config entirely
    }
}
```

### Step 8: Write tests

Add tests in `config/tests.rs` covering:
- Default values
- Custom TOML parsing (via `load_config()` with a temp dir)
- Env var override
- Env var invalid value fallback
- Validation boundary cases

## The Overlay System (How Config Layers Merge)

### Layer 1: TOML file (`config/default.toml`)

Parsed by `serde` via `toml::from_str()`. Uses `#[serde(default = "...")]` for missing fields.

### Layer 2: `.env` file

`load_dotenv_defaults()` reads `.env` and calls `env::set_var()` for keys not already set in the environment:
- Strips matching quotes (both `"` and `'`)
- Skips comments (`#`), empty lines, malformed lines (no `=`)
- Trims whitespace around key/value
- Does NOT override already-set env vars (explicit vars win)

### Layer 3: Explicit env vars

`apply_env_overrides()` reads `env::var("SOME_VAR")` and overwrites the config field:
- String fields (`ROXYBROWSER_API_URL`, `ROXYBROWSER_API_KEY`) → direct assignment
- Numeric fields (`MAX_GLOBAL_CONCURRENCY`, `CURSOR_OVERLAY_MS`) → `.parse().unwrap_or(existing)`
- DurationMs fields (`TASK_TIMEOUT_MS`) → `.parse::<u64>()?.and_then(DurationMs::new).unwrap_or(existing)`
- Enum fields (`native_click_calibration`) → `from_env_value()` with fallback
- Float probabilities (`TWITTER_LIKE_PROBABILITY`) → `parse_env_float()` with comment-stripping
- Semi-colon separated (`BROWSER_EXTRA_HTTP_HEADERS`, `TASK_DISCOVERY_ROOTS`) → split + collect

If an env var fails to parse, the original TOML/default value is preserved (no silent zero).

### Layer 4: Hardcoded defaults in `load_code_config()`

Only used when `config/default.toml` doesn't exist. Provides minimal functional defaults.

## Config Validation System

Two validation layers exist:

### 1. `ConfigValidationReport` (advisory, in `mod.rs`)

Called by `validate_config()`. Produces warnings for questionable values and errors for hard-invalid ones:
- `validate_orchestrator_config()` — concurrency range, timeout sanity, retry limits, cross-field validation
- `validate_browser_config()` — discovery retries, profile uniqueness, URL format, `max_workers_per_session`
- `validate_circuit_breaker()` — threshold ranges, half-open timing
- `validate_twitter_activity_config()` — feed scan duration, scroll count, engagement limits, guard thresholds
- `validate_llm_config()` — temperature, max_tokens, timeout, probability ranges
- `validate_tracing_config()` — endpoint URL format, service name

### 2. `Validate` trait (hard bounds, in `validation.rs`)

Called by `Config::validate()`. Returns `ConfigError::InvalidValue` or `ConfigError::MissingField`:
- `OrchestratorConfig::validate()` — concurrency > 0, timeouts >= 1000ms, retries <= 10
- `BrowserConfig::validate()` — timeout >= 5000ms, discovery retries >= 1, profiles not empty, hex color format

The `Validate` trait is stricter and is used for rejecting config at startup. The `ConfigValidationReport` is more holistic with advisory warnings.

## CLI Parsing

### Args struct (`cli/mod.rs`)

Defined with `#[derive(Parser)]` from clap:

| Flag | Type | Purpose |
|---|---|---|
| `tasks` | `Vec<String>` | Positional: task names with optional payload |
| `--browsers` | `Option<String>` | Comma-separated browser filter |
| `--clear-learning` | `bool` | Clear click learning data |
| `--list-tasks` | `bool` | List all available tasks |
| `--help-task <TASK>` | `Option<String>` | Show help for a specific task |
| `--dry-run` | `bool` | Simulate without executing |
| `--validate-tasks` | `bool` | Validate external task files |
| `--watch` | `bool` | Watch task dirs for changes |
| `--debug` | `bool` | Enable debug logging |

### Task Group Parsing (`cli/parser.rs`)

Parses CLI args into `Vec<Vec<CliTaskDefinition>>`:
- Each `Vec<CliTaskDefinition>` is a parallel group
- `then` separates sequential groups
- `task=value` → payload with automatic URL detection (adds `https://` if contains `.`)
- `task=42` → numeric payload → `{"value": 42}`
- `.js` suffix is stripped from task names
- `parse_scalar_value()` → tries bool, i64, f64, then string

## Key Env Vars Reference

### Browser
| Env Var | Type | Config Field |
|---|---|---|
| `ROXYBROWSER_API_URL` | String | `browser.roxybrowser.api_url` |
| `ROXYBROWSER_API_KEY` | String | `browser.roxybrowser.api_key` |
| `BROWSER_USER_AGENT` | String | `browser.user_agent` |
| `BROWSER_EXTRA_HTTP_HEADERS` | KV pairs (;) | `browser.extra_http_headers` |
| `CURSOR_OVERLAY_MS` | u64 | `browser.cursor_overlay_ms` |
| `CURSOR_OVERLAY_COLOR` | String | `browser.cursor_overlay_color` (empty = no override) |
| `CURSOR_OVERLAY_SHOW_TRAIL` | bool | `browser.cursor_overlay_show_trail` |
| `native_click_calibration` | enum | `browser.native_interaction.calibration_mode` |
| `NATIVE_CLICK_CALIBRATION` | enum | Same field (uppercase alias) |
| `NATIVE_INPUT_BACKEND` | enum | `browser.native_interaction.native_input_backend` |
| `NATIVE_INTERACTION_STABILITY_WAIT_MS` | u64 | `browser.native_interaction.stability_wait_ms` |
| `NATIVE_INTERACTION_RESOLVE_TIMEOUT_MS` | u64 | `browser.native_interaction.resolve_timeout_ms` |
| `NATIVE_INTERACTION_SETTLE_MS` | u64 | `browser.native_interaction.settle_ms` |

### Orchestrator
| Env Var | Type | Config Field |
|---|---|---|
| `MAX_GLOBAL_CONCURRENCY` | usize | `orchestrator.max_global_concurrency` |
| `TASK_TIMEOUT_MS` | u64 → DurationMs | `orchestrator.task_timeout_ms` |
| `MAX_RETRIES` | u32 | `orchestrator.max_retries` |
| `RETRY_DELAY_MS` | u64 → DurationMs | `orchestrator.retry_delay_ms` |

### Twitter Activity
| Env Var | Type | Config Field |
|---|---|---|
| `TWITTER_MAX_LIKES` | u32 | `twitter_activity.engagement_limits.max_likes` |
| `TWITTER_MAX_RETWEETS` | u32 | `twitter_activity.engagement_limits.max_retweets` |
| `TWITTER_MAX_FOLLOWS` | u32 | `twitter_activity.engagement_limits.max_follows` |
| `TWITTER_MAX_REPLIES` | u32 | `twitter_activity.engagement_limits.max_replies` |
| `TWITTER_MAX_THREAD_DIVES` | u32 | `twitter_activity.engagement_limits.max_thread_dives` |
| `TWITTER_MAX_BOOKMARKS` | u32 | `twitter_activity.engagement_limits.max_bookmarks` |
| `TWITTER_MAX_TOTAL_ACTIONS` | u32 | `twitter_activity.engagement_limits.max_total_actions` |
| `TWITTER_LIKE_PROBABILITY` | f64 | `twitter_activity.probabilities.like_probability` |
| `TWITTER_RETWEET_PROBABILITY` | f64 | `twitter_activity.probabilities.retweet_probability` |
| `TWITTER_QUOTE_PROBABILITY` | f64 | `twitter_activity.probabilities.quote_probability` |
| `TWITTER_FOLLOW_PROBABILITY` | f64 | `twitter_activity.probabilities.follow_probability` |
| `TWITTER_REPLY_PROBABILITY` | f64 | `twitter_activity.probabilities.reply_probability` |
| `TWITTER_BOOKMARK_PROBABILITY` | f64 | `twitter_activity.probabilities.bookmark_probability` |
| `TWITTER_THREAD_DIVE_PROBABILITY` | f64 | `twitter_activity.probabilities.thread_dive_probability` |
| `TWITTER_SCROLL_AMOUNT_PIXELS` | i32 | `twitter_activity.scroll_amount_pixels` |
| `TWITTER_CANDIDATE_SCAN_INTERVAL_MS` | u64 | `twitter_activity.candidate_scan_interval_ms` |
| `TWITTER_MAX_CONSECUTIVE_SCROLL_FAILURES` | u32 | `twitter_activity.max_consecutive_scroll_failures` |
| `TWITTER_MAX_CONSECUTIVE_EMPTY_SCANS` | u32 | `twitter_activity.max_consecutive_empty_scans` |
| `TWITTER_LLM_ENABLED` | bool | `twitter_activity.llm.enabled` |
| `TWITTER_LLM_PROVIDER` | String | `twitter_activity.llm.provider` |
| `TWITTER_LLM_MODEL` | String | `twitter_activity.llm.model` |
| `TWITTER_LLM_REPLY_PROBABILITY` | f64 | `twitter_activity.llm.reply_probability` |
| `TWITTER_LLM_QUOTE_PROBABILITY` | f64 | `twitter_activity.llm.quote_tweet_probability` |

### Task Discovery
| Env Var | Type | Config Field |
|---|---|---|
| `TASK_DISCOVERY_ENABLED` | bool | `task_discovery.enabled` |
| `TASK_DISCOVERY_ROOTS` | paths (;) | `task_discovery.roots` |
| `TASK_DISCOVERY_EXTENSIONS` | extensions (;) | `task_discovery.extensions` |

### Tracing
| Env Var | Type | Config Field |
|---|---|---|
| `TRACING_ENABLED` | bool | `tracing.enabled` |
| `TRACING_OTLP_ENDPOINT` | String | `tracing.otlp_endpoint` |
| `TRACING_SERVICE_NAME` | String | `tracing.service_name` |

## Testing Your Config Changes

Run targeted tests:

```powershell
# All config tests
cargo test --lib config::tests
cargo test --lib validation::validation::tests
cargo test --lib cli::mod::tests
cargo test --lib cli::parser::tests

# Specific test patterns
cargo test --lib config::tests::test_load_config_applies_env_overrides
cargo test --lib config::tests::test_cursor_overlay_color_env
```

## Common Pitfalls

1. **`#[serde(default)]` vs `#[serde(default = "...")]`**: The first uses `Default::default()`, the second calls a named function. For types without `Default` impl (like `DurationMs`), always use a named function.

2. **TOML boolean format**: TOML uses lowercase `true`/`false`. YAML-style `True`/`Yes` will fail silently.

3. **Env var fallback behavior**: When parse fails, the existing config value is preserved. If the config also has default 0, the user gets 0 silently. Always set a sensible default.

4. **`.env` quote stripping**: Only when both opening AND closing quotes match. `"value` or `value"` does NOT strip.

5. **`parse_env_float` comment stripping**: Strips everything after `#` in the value. So `TWITTER_LIKE_PROBABILITY=0.02 # 2%` correctly parses as `0.02`.

6. **Explicit env vs .env**: Explicit env vars (set via `$env:VAR=value` in PowerShell or `set VAR=value` in CMD) take precedence over `.env` because `load_dotenv_defaults()` skips keys that already exist.

7. **`load_code_config()` mismatch**: This fallback config in `env.rs` must stay in sync with `config/default.toml`. If you add a field but forget to update it, the fallback path will be missing that field (uses Default).

8. **Tests that modify env vars**: Config tests use a global `Mutex` lock to prevent parallel test interference when modifying env vars. Always use `config_test_lock()` when writing tests that set env vars.

9. **`cursor_overlay_color` empty string**: Empty string is treated as "no override" in `apply_env_overrides()`, preserving the TOML/default value. This allows unsetting the env var without side effects.

10. **`Validate` trait vs `ConfigValidationReport`**: The `Validate` trait in `validation.rs` is strict (returns `Err` for hard constraints). The `ConfigValidationReport` in `mod.rs` is advisory (returns `warn!()` for soft issues and `Err` for hard limits). Both are called during startup — first `load_config()`, then `validate_config()`.
