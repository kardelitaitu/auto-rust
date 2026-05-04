# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
