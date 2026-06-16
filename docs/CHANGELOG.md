# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.3] - 2026-06-16

### Changed
- **crates**: Moved `bacon-pipeline/` to `crates/bacon-pipeline/` (workspace member rename, all paths updated)
- **docs**: Fixed project structure tree in `.bacon/README.md` to reflect `crates/bacon-pipeline/` layout

### Removed
- **cleanup**: Deleted 42 root-level generated artifacts (compiler output, analysis docs, temp files, stale output directories)

### Fixed
- **tests**: Updated `setup_script_exists` test path for `scripts/setup-windows.bat` after root reorganization

## [Unreleased]

### Added
- **lint**: `#![deny(clippy::unwrap_used)]` + `#![deny(clippy::expect_used)]` in both `src/lib.rs` and `crates/bacon-pipeline/src/lib.rs`; CI + `check.ps1` enforcement for `--lib` and `--bins` targets
- **lint**: `#![deny(unsafe_code)]` in `crates/bacon-pipeline/src/lib.rs` (0 unsafe blocks; forward-looking guard)
- **lint**: `#![deny(unsafe_op_in_unsafe_fn)]` in `src/lib.rs` (0 `unsafe fn` definitions; forward-looking guard)
- **audit**: `.cargo/audit.toml` with async-std advisory exception (transitive dep via chromiumoxide)

### Changed
- **deps**: Replaced `serde_yml = "0.0.13"` (unsound, unmaintained) with `serde_yaml = "0.9"` across both workspace crates — 129 occurrences in 18 files; fixed 3 API differences (`Mapping::insert` needs `Value` keys, `Value::as_str()` returns `Option<&str>`)
- **fix**: Fixed PowerShell encoding bug in `scripts/miri.ps1` (UTF-8 em dash → ASCII hyphens on line 281)

### Fixed
- **panic**: Removed `Llm::default()` impl (panicked on LLM init failure; 0 callers)
- **panic**: `UnifiedLLMProcessor::new()` now returns `Result` instead of panicking on LLM init failure
- **panic**: HTTP client `.expect()` in `LlmStrategy` → `.unwrap_or_default()`
- **dead_code**: Wrapped `ENV_LOCK`, `EnvGuard`, `EnvGuard::new()`, `Drop for EnvGuard` in `#[cfg(test)]` in `crates/bacon-pipeline/src/agent/cli.rs` — removed 3 `#[allow(dead_code)]` annotations

### Security
- **audit**: Ran `cargo audit` — 2 advisories: `serde_yml` resolved by migration to `serde_yaml`; `async-std` is transitive, allowed via `.cargo/audit.toml`
- **miri**: Ran `cargo +nightly miri test` — 18/18 duration tests pass, full suite (excluding FFI/async) passes in 2,496s; no undefined behavior detected in the 2 `unsafe` blocks
- **fuzz**: Fixed `fuzz/Cargo.toml` — migrated `serde_yml = "0.0.13"` to `serde_yaml = "0.9"`, fixed TOML table-header bug (dependency was dangling outside `[dependencies]`), updated `fuzz/README.md` references; fuzz workspace compiles cleanly
- **integration**: `cargo nextest --all-features --tests` — 3,896 tests passed, 0 failed (55 skipped)
- **udeps**: `cargo +nightly udeps --all-targets` — `tracing-subscriber` confirmed used by tests; `do-over` confirmed unused (dead `circuit_breaker.rs` module was never declared in `mod.rs`) and removed

### Removed
- **cleanup**: Removed stale `#[allow(deprecated)] // TODO: migrate from serde_yml...` comment from `src/task/dsl/parser.rs`
- **deps**: Removed unused `do-over = "0.1.0"` dependency (dead circuit breaker module, never compiled)
- **cleanup**: Deleted `src/internal/circuit_breaker.rs` (dead file, not declared in `mod.rs`, `BrowserCircuitBreaker` never referenced outside its own tests)

### Added
- **errors**: Added `ErrorPattern::RateLimited` variant and LLM-specific transient patterns (`"rate limit"`, `"429"`, `"overloaded"`, `"503"`, `"server error"`, `"model is at capacity"`, `"try again later"`) to shared `classify_error_pattern()` in `result/errors.rs`
- **errors**: Added `TaskErrorKind::ExternalService` variant for LLM API errors (rate limiting, overloaded, unavailable) — separate from `Browser` for semantic clarity
- **errors**: Added 8 LLM transient patterns to `twitteractivity_errors` `ErrorClassifier` so rate limits and server errors are classified as `Transient` (retryable) instead of `Permanent`
- **errors**: Added `test_permanent_errors_unaffected_by_rate_limited_classification` regression guard in `page_nav.rs` to verify permanent errors stay permanent
- **retry**: Added 7 end-to-end integration tests in `twitteractivity_retry.rs` verifying LLM rate-limit retry loop: conservative config (5 attempts / 4 delays), default config (3 attempts / 2 delays), partial retry then success, all patterns covered
- **llm**: Wrapped `generate_reply()` and `generate_quote_commentary()` with `retry_with_backoff` using `RetryConfig::conservative()` — each attempt gets its own 30s timeout, messages cloned per attempt
- **counters**: Added `EngagementCounters::computed_total()` (raw sum of all 7 counter fields) and `assert_total_synced()` (`debug_assert_eq!` consistency check in debug builds) with 3 mutation tests
- **tests**: `test_is_transient_error_rate_limited`, `test_is_transient_error_llm_overloaded`, `test_is_transient_error_try_again_later` in `page_nav.rs`
- **tests**: 8 `anyhow_*_is_transient` tests in `twitteractivity_errors.rs` gap_tests for each LLM pattern

### Changed
- **errors**: `ErrorPattern::RateLimited` now maps to `TaskErrorKind::ExternalService` (was `Browser`)
- **errors**: DRY refactoring — `ErrorClassifier::classify()` now calls `is_rate_limit_error(self)` instead of duplicating 3 inline string checks
- **retry**: `is_retryable()` now includes `ExternalService`; `should_mark_session_unhealthy()` includes `ExternalService`
- **retry**: Session I/O retry (`session_io_evaluate_with_retry`) now delegates to shared `TaskContext::with_retry()` instead of duplicating retry logic
- **engagement**: Extracted `execute_engagement_action()` helper from 3 identical match arms (retweet, follow, bookmark) — each reduced from ~15 to 6 lines; added `ActionMetrics` struct with separate `action_name`/`retry_name` fields
- **actions**: Extracted shared `select_template()` helper from near-identical `generate_reply_text()` and `generate_quote_text()`
- **state**: Moved `build_summary_lines()` from orchestrator standalone fn to `SessionState::build_summary_lines(duration_ms: u64)`; 4 tests relocated to `session.rs`
- **sentiment**: Switched `static SENTIMENT_ANALYZER` from `std::sync::Mutex` to `tokio::sync::Mutex` for async consistency; `modulate_persona_by_sentiment()` now async
- **llm**: Renamed `_api → api` in `generate_reply()`/`generate_quote_commentary()`; updated `#[instrument(skip(api, ...))]`
- **metrics**: Added `ExternalService` to `ERROR_KIND_STRINGS` static map
- **tests**: All `TaskErrorKind` enumeration tests updated (6→7 variants), serde round-trip tests updated, retryable assertions updated

### Fixed
- **tests**: Fixed 6 pre-existing `api_client_integration.rs` failures (403 Forbidden) caused by system HTTP proxy intercepting localhost wiremock — added `ensure_no_proxy()` using `std::sync::Once`
- **tests**: Fixed 2 pre-existing Ollama test failures — added `NO_PROXY=127.0.0.1,localhost` to subprocess env and switched fake Ollama response from NVIDIA Chat Completions format (`choices[]`) to Ollama API format (`{"done":true,"message":{"content":"..."}}`) in `bacon_dry_run_smoke.rs` and `bacon_pipeline_integration.rs`
- **bacon-pipeline**: Fixed 23 pre-existing test failures ("Once poisoned", "ProjectConfig already initialized") caused by parallel test modules racing on `OnceLock::set()` — made `config::init()` idempotent with warning log on re-init, added `ENV_MUTEX` to serialize `LLM_PROVIDER` env var mutations between two racing tests
- **docs**: Fixed 2 pre-existing doc-test failures in `src/session/mod.rs` — changed plain integer literals (`30_000`, `30000`) to `DurationMs::new_const(...)` for `CircuitBreakerConfig.half_open_time_ms` (field type is `DurationMs`, not `u64`)

### Removed
- **cleanup**: Removed standalone `build_summary_lines()` from `src/task/twitteractivity.rs` (moved to `SessionState`)

### Fixed
- **panic**: Replaced byte-index string slicing `&text[..N]` with `chars().take(N).collect()` in LLM sentiment analysis to prevent panic on multi-byte Unicode characters (wasm/pyodide-safe compilation)
- **panic**: Replaced manual byte-index in `truncate_to_word_boundary` with `floor_char_boundary()` to prevent panic on non-ASCII text
- **panic**: Guarded `candidate_budget == 0` case in simulation loop with elapsed-ms advancement before `continue` to prevent `gen_range(1..=0)` panic and infinite loop
- **panic**: Added `phrases.is_empty()` guards in `generate_reply_text` and `generate_quote_text` to prevent modulo-by-zero panic on empty sentiment templates
- **race**: Replaced `RwLock<bool>` circuit breaker with `AtomicU8` CAS state machine (`CLOSED`/`HALF_OPEN`/`OPEN`) to eliminate TOCTOU race where multiple concurrent callers could all probe during the reset window
- **race**: Scoped `like_at_position` verification JS from full-page `document` root to nearest `article[data-testid="tweet"]` container to prevent DOM queries scanning unrelated page sections concurrently
- **corruption**: Removed 11 overlapping emojis from `NEUTRAL_EMOJIS` that also existed in `NEGATIVE_EMOJIS` (fixes sentiment HashMap overlap causing corrupted classification)
- **corruption**: Fixed action string mismatch in `available_actions()` — renamed `"thread_dive"` → `"dive"`, `"quote_tweet"` → `"quote"` to match state-machine action tracker keys
- **corruption**: Removed `interest_multiplier` from `effective_probability()` calculation to fix double-count (multiplier is already applied in `get_probability()`, which calls `effective_probability()`)
- **corruption**: Added word-boundary matching (`contains_word` helper) for domain sentiment keywords — prevents false matches like `"bug"` matching inside `"bugatti"` or `"pr"` matching inside `"prayer"`
- **logic**: Increased tweet article detection timeout from 1s to 3s in `dive()` to reduce flaky failures on slow page loads
- **logic**: Added selector-based `like_tweet()` fallback when `extract_tweet_button_position` returns `None` in feed view (missing like button coordinates no longer skips the action)
- **logic**: Removed empty-string `_selector` parameter from `hover_before_click()` signature — was passing `""` which caused unintended behavior
- **logic**: Moved `"connection refused"` from `Fatal` to `Transient` classification in `ErrorClassifier` (was inconsistent with `std::io::Error::ConnectionRefused` which was already transient)
- **docs**: Changed SEARCH/REPLACE format example from ` ``` ` to ` ```text ` so `path/to/file.ext` is not parsed as Rust code in doc-test
- **docs**: Removed references to non-existent functions `scroll_feed` and `get_scroll_progress` from `twitteractivity_feed` module doc example

### Changed
- **sentiment**: Expanded `with_sentiment_modulation` range from `[0.5, 1.0]` to `[0.0, 1.0]` so neutral/low-sentiment content can reduce interest multiplier appropriately
- **sentiment**: Added warning-level logs to stub functions `extract_user_reputation` and `extract_temporal_factors`; excluded `reputation_score` and `temporal_score` from `calculate_factor_agreement()` to avoid false agreement with default values
- **config**: Extracted magic number thresholds (`3`) into `TwitterActivityConfig` fields `max_consecutive_scroll_failures` and `max_consecutive_empty_scans` (configurable via TOML or env vars)
- **config**: Added env var overrides `TWITTER_MAX_CONSECUTIVE_SCROLL_FAILURES` and `TWITTER_MAX_CONSECUTIVE_EMPTY_SCANS` to `apply_env_overrides()`
- **config**: Added validation warnings for threshold values of 0 or >20 in `validate_twitter_activity_config`
- **docs**: Updated `API_REFERENCE.md` with new config fields, env var overrides, and validation rules
- **docs**: Updated `config/default.toml` with inline config fields and env var documentation
- **engagement**: Changed `like_at_position` raw string from `r"..."` to `r#"..."#` syntax to allow literal double quotes in JS selector strings
- **twitteractivity**: Replaced `checked_sub().unwrap()` in `build_summary_lines` with `saturating_sub()` to eliminate panic risk from clock drift
- **twitteractivity**: Removed unused `_payload` parameter from `run_inner` signature
- **twitteractivity**: Simplified `build_persona` to take `&BrowserProfile` instead of `&TaskContext`
- **twitteractivity**: Replaced `HashMap::get().unwrap_or(&0)` in `build_summary_lines` with direct struct field access
- **twitteractivity**: `log_summary` now takes `&Config` and emits guard threshold values in summary log
- **navigation**: Replaced `const { assert!(...) }` with regular `assert!(...)` (unstable Rust construct)
- **navigation**: Changed `log::warn!` to `warn!` for consistency with existing `use log::{info, warn}` import
- **navigation**: Updated module doc to list actual function names (`goto_home`, `goto_notifications`, `verify_login`, `is_feed_visible`, `wait_for_page_ready`) instead of non-existent ones

### Removed
- **cleanup**: Removed `confirm_retweet()` function and its `js_confirm_retweet_click` import (dead code, never called)
- **cleanup**: Removed unused `RetryStats` struct from `twitteractivity_retry.rs`
- **cleanup**: Removed stub `dismiss_signup_nag()` function and its unit test from `twitteractivity_popup.rs`
- **cleanup**: Removed 4 unused `TaskValidationError` enum variants: `InvalidDuration`, `InvalidCandidateCount`, `InvalidThreadDepth`, `InvalidMaxActionsPerScan`
- **cleanup**: Updated `TaskValidationError` test to use `InvalidPositiveNumber` instead of removed `InvalidDuration`

### Added
- **tests**: `test_twitter_consecutive_threshold_env_overrides` — verifies env var overrides produce expected values
- **tests**: `test_twitter_consecutive_threshold_env_overrides_invalid_parse_falls_back` — verifies fallback to defaults on invalid env var values
- **tests**: `test_load_config_applies_twitter_consecutive_threshold_env_overrides` — full-path integration test: TOML file + env vars through `load_config()` for consecutive threshold overrides
- **tests**: `test_load_config_applies_twitter_consecutive_threshold_invalid_env_falls_back_to_toml` — full-path integration test: invalid env vars fall back to TOML file values, not hardcoded defaults
- **tests**: `test_load_config_applies_twitter_engagement_limit_env_overrides` — full-path integration test: all 5 engagement limit env vars (`TWITTER_MAX_LIKES`, `TWITTER_MAX_RETWEETS`, `TWITTER_MAX_FOLLOWS`, `TWITTER_MAX_REPLIES`, `TWITTER_MAX_TOTAL_ACTIONS`) override TOML file through `load_config()`
- **tests**: `test_load_config_applies_twitter_engagement_limit_invalid_env_falls_back_to_toml` — full-path integration test: invalid engagement limit env vars fall back to TOML file values
- **tests**: `test_load_config_applies_twitter_probability_env_overrides` — full-path integration test: all 7 probability env vars override TOML file through `load_config()`
- **tests**: `test_load_config_applies_twitter_probability_invalid_env_falls_back_to_toml` — full-path integration test: invalid probability env vars fall back to TOML file values
- **tests**: `test_load_config_applies_browser_orchestrator_env_overrides` — full-path integration test: 5 browser/orchestrator env vars override TOML file through `load_config()`
- **tests**: `test_load_config_applies_browser_orchestrator_invalid_env_falls_back` — full-path integration test: invalid browser/orchestrator env vars fall back to TOML values or safe defaults

## [0.1.1] - 2026-05-05

### Added

#### Enhanced Conditions (8 New Condition Types)
- `text_matches` - Regex pattern matching on element text
- `variable_matches` - Regex matching on variable values
- `numeric_greater_than` / `numeric_less_than` - Numeric comparisons
- `numeric_range` - Inclusive range checks with min/max bounds
- `date_before` / `date_after` - Date/time comparisons with optional format strings
- `array_contains` - Check if array contains value
- `array_length` - Array length validation with min/max/exact

#### DSL Debugging Features
- Breakpoint system with multiple trigger types:
  - Action index breakpoints (pause at specific action)
  - Action type breakpoints (pause on any action of type)
  - Variable watch breakpoints (trigger on variable changes)
  - Conditional breakpoints (custom closure conditions)
- Execution tracing with `DebugEvent` log
- Variable inspection mid-execution via `inspect_state()`
- Pause/resume/step-through execution control
- Debug event types: ActionStart, ActionComplete, ActionError, VariableSet, ConditionEvaluated

#### Performance Optimizations
- **Selector Caching** - LRU cache for DOM queries:
  - 100 entry capacity with automatic eviction
  - 5-second TTL for cache entries
  - Hit rate tracking and statistics via `CacheStats`
  - Smart invalidation on mutations
- **Action Profiling** - Per-action-type performance tracking:
  - Total executions, total/average/min/max duration
  - Failure rate monitoring
  - JSON export via `get_profiler_stats()`

#### New DSL Actions
- `parallel` - Execute actions concurrently with configurable max_concurrency

### Changed
- `WaitFor` action now uses cached element existence checks for better performance
- `DslExecutor` constructors now initialize selector cache and profilers by default

## [0.1.0] - 2026-05-04

### Added

#### Core Framework
- Multi-browser automation framework with Tokio async runtime
- Support for Brave, Chrome, and RoxyBrowser via Chrome DevTools Protocol
- Session management with health scoring and automatic recovery
- Circuit breaker pattern for failure handling
- Graceful shutdown with cancellation token propagation
- Human-like behavior with 6 cursor path styles and 21 personas

#### DSL Task System
- YAML-based task definitions with parameterized actions
- Task registry with 16 built-in tasks (cookiebot, pageview, twitteractivity, etc.)
- Variable substitution with `{{variable}}` syntax
- Action types:
  - Basic: `navigate`, `click`, `type`, `wait`, `screenshot`, `extract`, `scroll`
  - Control Flow: `if`/`then`/`else`, `while`, `foreach`, `retry`, `try`/`catch`/`finally`
  - Utility: `log`, `set`, `include`

#### Control Flow Actions
- **`If`**: Conditional execution with 7 condition types
  - `element_visible`, `element_exists`, `element_not_exists`
  - `variable_set`, `variable_not_set`, `variable_equals`
  - `comparison` (with `==`, `!=`, `<`, `>`, `<=`, `>=`)
  
- **`Foreach`**: Iterate over collections
  - Array: `["a", "b", "c"]`
  - Range: `{start: 0, end: 5}`
  - Elements: DOM elements matching selector
  - Variable: Reference to array variable
  - Configurable `max_iterations` (default: 100)

- **`While`**: Condition-based looping
  - Evaluates condition before each iteration
  - Same condition types as `If`
  - Safety limit with `max_iterations` (default: 1000)

- **`Retry`**: Automatic retry with exponential backoff
  - Configurable `max_attempts` (default: 3)
  - `initial_delay_ms` with exponential backoff
  - `backoff_multiplier` (default: 2.0)
  - `jitter` to prevent thundering herd (default: true)
  - `retry_on` error pattern matching

- **`Try/Catch/Finally`**: Error handling
  - `try_actions`: Actions to attempt
  - `catch_actions`: Recovery on error (optional)
  - `error_variable`: Store error message (optional)
  - `finally_actions`: Always execute (optional)

#### Parameter System
- 5 parameter types: `string`, `integer`, `boolean`, `url`, `selector`
- Required vs optional parameters
- Default values for optional parameters
- Type validation with helpful error messages
- URL validation (requires http:// or https://)
- CSS selector validation (balanced brackets, quotes, parens)

#### Plugin System
- WASM-based plugin architecture (extensibility foundation)
- Plugin manifest format (TOML/YAML/JSON)
- Plugin registry with dependency management
- Plugin loader with allowlist/denylist filtering
- Hook system for task lifecycle events

#### Configuration
- TOML configuration with environment variable overrides
- Browser profiles (Brave, Chrome, RoxyBrowser)
- Task policies with configurable timeouts and retry logic
- Logging with structured output

#### Examples & Documentation
- 20+ example task templates across 3 difficulty levels:
  - **Basic**: form-submission, page-screenshot, simple-navigation
  - **Intermediate**: data-extraction-pipeline, handle-errors, wait-for-loading
  - **Advanced**: retry-flaky-operation, process-multiple-items
- Plugin development guide with API reference
- Comprehensive rustdoc documentation

#### Testing & Quality
- 2166+ unit tests covering all major components
- CI with `cargo test`, `cargo fmt`, `cargo clippy`
- Code coverage tracking

### Performance
- **10x throughput** vs Node.js baseline (~50 tasks/sec with 20 sessions)
- **<2 second startup** including browser discovery
- **~50-200 MB memory** footprint per session
- **Zero-allocation hot paths** for critical operations

### Security
- No hardcoded credentials
- Input validation on all parameters
- Safe YAML parsing with size limits
- Circuit breaker prevents cascade failures

## Roadmap

### v0.2.0 (Planned)
- [ ] Task composition (tasks calling other tasks)
- [ ] Parallel action execution
- [ ] Pre-flight validation (validate entire task before execution)
- [ ] Enhanced condition types (regex matching, date comparisons)
- [ ] DSL debugging mode with step-through execution

### v0.3.0 (Planned)
- [ ] WASM plugin runtime (execute plugins, not just load)
- [ ] Built-in plugin marketplace
- [ ] Task scheduler with cron expressions
- [ ] Distributed execution across multiple machines
- [ ] Web dashboard for monitoring and management

### v1.0.0 (Future)
- [ ] Stable plugin API
- [ ] Visual task builder
- [ ] AI-powered task generation from natural language
- [ ] Enterprise features (SSO, audit logging, compliance)

---

[0.1.0]: https://github.com/kardelitaitu/auto-rust/releases/tag/v0.1.0
