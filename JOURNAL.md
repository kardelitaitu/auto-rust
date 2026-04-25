# Agents Journal

**New journal entries should be at the top for easy indexing**
```
**Date - Title - commit number**
- filename : Description
```

---

## 2026-04-25 - Phase 3 Completion - eed0a61
- `browser.rs` : Added integration tests for `discover_browsers_with_filters`:
  - `test_discover_browsers_with_filters_empty_config_no_filter` - verifies warn path when no filters
  - `test_discover_browsers_with_filters_empty_config_with_filter` - verifies error path when filters active but no matches
  - `test_discover_browsers_with_filters_empty_filter_string` - verifies empty filter treated as no filter
- Tests use minimal config with `max_discovery_retries: 0` to avoid actual browser connections
- Validation: 1,607 tests passed (3 new tests added)

## 2026-04-25 - Phase 1 & 2 Orchestrator Reliability Improvements - 78ba194, 041117e

**Phase 1: Code Cleanup (Commit: 041117e)**
- `task/mod.rs` : Removed misleading `_max_retries` parameter from `perform_task()` - retry is intentionally disabled (fail-fast design handled at orchestrator level)
- `health_monitor.rs` : Fixed `HealthLogger::stop()` - shutdown flag was created but never checked in logging loop, causing task to run forever
- `orchestrator.rs` : Added design comments explaining intentional session state check-then-act pattern for broadcast fan-out execution model
- Validation: 1,604 tests passed

**Phase 2: Circuit Breaker & Connection Timeouts (Commit: 78ba194)**
- `session.rs` : Integrated circuit breaker with `acquire_worker()` - sessions with open circuits now fast-fail, preventing task assignment to failing sessions
- `browser.rs` : Added connection timeout to all 3 `Browser::connect` calls:
  - Config-based browser connection (profile-based)
  - Brave auto-discovery on ports 9001-9050
  - Roxybrowser cloud API discovery
- Timeout defaults to max(5000ms, config.browser.connection_timeout_ms) to prevent indefinite hangs when browser accepts TCP but doesn't complete WebSocket handshake
- Validation: 1,604 tests passed, `cargo check` clean

## 2026-04-22 - Phase 1 Completion - [commit from that date]
- `orchestrator.rs` : Moved global semaphore gating from per-task entry to per task-session execution slot acquisition
- `orchestrator.rs` : Added cancellation-aware global slot acquisition so waiting fan-out units cancel immediately on group shutdown
- `orchestrator.rs` : Added regression tests:
  - `test_global_execution_slot_enforces_hard_concurrency_bound`
  - `test_global_execution_slot_cancels_while_waiting_for_permit`
- Validation: `cargo test --quiet` full suite passed after patch

## 2026-04-22 - Documentation Improvements

#### Rust Documentation Enhancements
- `Cargo.toml` : Added documentation metadata (authors, description, license, repository, keywords, categories)
- `session.rs` : Added comprehensive rustdoc comments for Session struct and core methods
- `browser.rs` : Added rustdoc comments for browser discovery functions
- `orchestrator.rs` : Added rustdoc comments for orchestration functions
- `task_context.rs` : Added comprehensive API documentation

#### Markdown Documentation Structure
- Created `docs/SUMMARY.md` for better documentation organization
- Added "Documentation" section to README

#### Documentation Generation
- Tested `cargo doc --all-features` - builds successfully

#### Log Noise Reduction
- Fixed rustdoc warnings by escaping brackets in logger.rs and twitteractivity_persona.rs
- Suppressed chromiumoxide WebSocket deserialization errors in FileLogger

#### Health Monitoring Improvements
- Changed memory warning from fixed bytes (1 GiB) to percentage-based threshold (86%)

#### Log Noise Reduction & Consolidation
- Removed debug markers (#[instrument] attributes) from orchestration functions
- Combined orchestration logs to reduce redundancy
- Reduced discovery verbosity

### Current Status

| Item | Status |
|------|--------|
| Build | ✅ Pass |
| Tests | ✅ 68 passed |
| cargo clippy | ✅ Clean |
| Documentation | ✅ Enhanced |

## 2026-04-21 - Session Progress

### Accomplished This Session

#### Documentation Updates
- Clarified that task groups are intentionally broadcast to every active browser session
- Added `pageview` payload alias guidance
- Added a short task-authoring checklist
- Softened the README status block by adding a last-verified date
- Documented the new run-summary session-health fields
- Added the live warning rule for degraded healthy-session coverage

#### Human-Like Mouse Simulation Features
- **Clustered pauses** (`clustered_pause()` in timing.rs): Adds micro-movements between pauses
- **Pointer events** (`dispatch_pointer_event()` in mouse.rs): Dispatches pointer events around clicks
- **Element-aware hover** (`hover_before_click()`): Detects element type and applies appropriate hover duration

#### Task-API Timing Contract
- `api.pause(base_ms)` now uses a uniform 20% deviation band
- High-level task-api verbs now add a built-in post-action settle pause
- `api.click(selector)` is the default click path

#### Code Review Fixes
- Fixed clippy warnings in reply_strategies.rs and twitteractivity_limits.rs
- Removed unused #[allow(dead_code)] from BrowserProfile, Session.profile_type, RoxybrowserConfig
- Added max_workers_per_session to BrowserConfig with validation
- Fixed orchestrator.rs timeout error handling
- Added shared `pageview` target resolver
- Added cooperative group timeout cancellation
- Added scoped log context cleanup and richer metrics
- Added run-summary export fields

#### LLM Integration
- Full OpenRouter support with automatic Ollama→OpenRouter fallback
- Standardized environment variables: LLM_PROVIDER, OLLAMA_URL

#### Configuration Updates
- Updated .env with current configuration variables
- Updated .env.example with comprehensive documentation
- Updated setup-windows.bat .env generator

### Current Status

| Item | Status |
|------|--------|
| Build | ✅ Pass |
| Tests | ✅ 68 passed |
| cargo clippy | ✅ Clean |

### Available Tasks
```
cookiebot, pageview, demo-keyboard, demo-mouse, demoqa, twitterfollow, twitterreply, twitteractivity
```

---

## Completed Phases (All Checkboxes ✅)

### Phase 0 - Foundations
- [x] Unified result types, error typing

### Phase 1 - Orchestrator Reliability
- [x] Per-task timeout, group timeout, retry metadata, health checks

### Phase 2 - Session Lifecycle
- [x] Full connect_to_browser, session state tracking, page registry, graceful shutdown

### Phase 3 - Config + Validation
- [x] TOML config loader, task validator, parser parity with Node.js, startup validation

### Phase 4 - API Utility Layer
- [x] HTTP client with retry, circuit breaker (feature-flagged)
- [ ] Provider fallback strategy (optional - not required for core)

### Phase 5 - Observability
- [x] Metrics collector, task history ring buffer, run-summary.json export, health logging

### Phase 6 - Utility Hardening
- [x] JS fallback path, deterministic utility tests, integration tests

## 2026-04-22 - Next 8-Phase Reliability Agenda (Task List)

### Phase 1 - Fan-Out Concurrency Bounding
- [x] Enforce global concurrency using `tasks x sessions` execution tokens (not per-task-only throttling).
- [x] Add tests proving hard upper bound under broadcast fan-out.

### Phase 2 - Session Supervisor Loop
- [ ] Add supervisor loop that enforces `Idle/Busy/Failed` state transitions for scheduling.
- [ ] Prevent task dispatch to non-idle or failed sessions by policy, not convention.

### Phase 3 - Zero-Match Browser Filter Hard Fail
- [x] Treat `--browsers` filter with zero matched sessions as startup error.
- [x] Add startup test coverage for valid/invalid filter scenarios.

### Phase 4 - Structured Run Report Upgrades
- [ ] Add run summary fields for planned vs executed fan-out.
- [ ] Add structured cancellation-cause breakdown (shutdown, timeout, worker wait, etc.).

### Phase 5 - Chaos Testing
- [ ] Add chaos tests for browser disconnect mid-task.
- [ ] Add chaos tests for page creation failures and timeout storms.

### Phase 6 - Deterministic Shutdown SLO Checks
- [ ] Add shutdown SLO assertions: pages closed, handler tasks aborted, no zombie process leftovers.
- [ ] Add deterministic tests for graceful completion and forced interruption paths.

### Phase 7 - CI Quality Gates
- [ ] Add CI gates for `cargo fmt --all -- --check`.
- [ ] Add CI gates for `cargo clippy --all-targets --all-features -D warnings`.
- [ ] Add CI gates for full test suite, doctests, and smoke run.

### Phase 8 - Node Parity Contract Tests
- [ ] Add compatibility contract tests against `.nodejs-reference` behavior for key orchestrator flows.
- [ ] Track parity drift in assertions for task parsing, dispatch, retry, and summary outputs.
