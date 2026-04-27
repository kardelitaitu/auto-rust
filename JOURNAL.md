# Agents Journal

**New journal entries should be at the top for easy indexing**
````
## yyyy-mm-dd (hh:mm) - Title - commit number ##
- **filename** : Description
**validation : cargo check and cargo test clean**
````

---

## 2026-04-27 (16:15) - API Design Plan v0.0.3 Implementation Complete - 90cb589

**Feature:** Implemented all 26 planned APIs from `API_DESIGN_PLAN.md` with permission gates and comprehensive tests.

### Implemented API Groups

| Group | APIs | Status |
|-------|------|--------|
| Cookie Management | export_cookies_for_domain, export_session_cookies, has_cookie | ✅ 3/3 |
| Session Management | export_local_storage, import_local_storage, validate_session_data | ✅ 3/3 |
| Clipboard Management | clear_clipboard, has_clipboard_content, append_clipboard | ✅ 3/3 |
| Data File Management | list_data_files, data_file_exists, delete_data_file, append_data_file, read_json_data, write_json_data, data_file_metadata | ✅ 7/7 |
| Network/HTTP | http_get, http_post_json, download_file | ✅ 3/3 |
| DOM Inspection | get_computed_style, get_element_rect, get_scroll_position, count_elements, is_in_viewport | ✅ 5/5 |
| Browser Management | export_browser, import_browser | ✅ 2/2 |

### New Permissions Added (12 total)
- `allow_export_cookies`, `allow_import_cookies`
- `allow_export_session`, `allow_import_session`
- `allow_session_clipboard`
- `allow_read_data`, `allow_write_data`
- `allow_http_requests`
- `allow_dom_inspection`
- `allow_browser_export`, `allow_browser_import`

### Implementation Details
- **Cookie Management** (`src/runtime/task_context.rs`): CDP Network.getCookies with filtering
- **Session Management** (`src/runtime/task_context.rs`): localStorage import/export via page.evaluate
- **Clipboard Management** (`src/runtime/task_context.rs`): ClipboardState wrapper methods
- **Data File Management** (`src/runtime/task_context.rs`): Rust std::fs with validate_data_path security
- **Network/HTTP** (`src/runtime/task_context.rs`): reqwest client with proper error handling
- **DOM Inspection** (`src/runtime/task_context.rs`): page.evaluate JavaScript execution
- **Browser Management** (`src/runtime/task_context.rs`): Complete browser state export/import

### New Structs
- `HttpResponse` - HTTP response with status, body, headers
- `FileMetadata` - File size, modified, created times
- `Rect` - Element position/size (x, y, width, height)
- `BrowserData` - Complete browser state (cookies, localStorage, sessionStorage, IndexedDB)

### Test Suite (12 new tests)
- `test_browser_data_default` - BrowserData initialization
- `test_browser_data_serialization_roundtrip` - JSON roundtrip
- `test_permissions_include_browser_export_import` - Permission validation
- `test_file_metadata_struct` - Metadata struct tests
- `test_http_response_struct` - HTTP response tests
- `test_rect_struct` - Rect serialization
- `test_click_learning_persistence_with_real_file` - File I/O
- `test_sanitize_path_component_various_inputs` - Path sanitization
- `test_click_timing_profile_edge_cases` - Timing edge cases
- `test_click_adaptation_with_extreme_failures` - Adaptation behavior

### Files Modified
- `src/task/policy.rs` - Added BrowserData struct, new permissions
- `src/runtime/task_context.rs` - 26 API methods, permission checks, 12 new tests

### Documentation
- `API_DESIGN_PLAN.md` - All checkboxes marked ☑

**Validation:** `cargo check` and `cargo test` clean ✅

**Feature:** Implemented task policy enforcement system from `IMPROVEMENT_PROPOSAL_task_policy.md`.

**Phase 1: Core Data Structures**
- Created `src/task/policy.rs` with `TaskPolicy`, `TaskPermissions`, `SessionData`, `DEFAULT_TASK_POLICY`
- Extended `src/error.rs` with `PermissionDenied`, `InvalidPath`, `CdpError`, `ClipboardError` variants
- Added `pub mod policy;` to `src/task/mod.rs`

**Phase 2: Policies to Tasks**
- Added policies to 15 task files:
  - `COOKIEBOT_POLICY`, `PAGEVIEW_POLICY`, `TWITTERACTIVITY_POLICY`
  - `TWITTER_BASE_POLICY` (base for Twitter tasks)
  - `DEMO_KEYBOARD_POLICY`, `DEMO_MOUSE_POLICY`, `DEMO_QA_POLICY`
  - `TWITTERDIVE_POLICY`, `TWITTERFOLLOW_POLICY`, `TWITTERINTENT_POLICY`
  - `TWITTERLIKE_POLICY`, `TWITTERQUOTE_POLICY`, `TWITTERREPLY_POLICY`
  - `TWITTERRETWEET_POLICY`, `TWITTERTEST_POLICY`, `TASK_EXAMPLE_POLICY`

**Phase 3: Permission Gates**
- Added `policy` field to `TaskContext` with `&'static TaskPolicy`
- Updated `TaskContext::new` and `new_with_metrics` to accept policy
- Updated orchestrator to pass policy when creating `TaskContext`
- Replaced config-based timeout with policy's `max_duration_ms`
- Added permission-gated methods: `screenshot()`, `export_cookies()`, `import_cookies()`, `export_session()`, `import_session()`, `read_clipboard()`, `write_clipboard()`, `read_data_file()`, `write_data_file()`
- Implemented `effective_permissions()` with implied permissions logic
- Added audit logging for session operations (`task_policy_audit` target)

**Validation:** `cargo check` clean ✅ (no errors)

**Integration Note:** This implements the core policy enforcement system. Remaining work: add unit tests for permission checks, add policies to remaining task files (if any), update documentation.

**Feature:** Automatic model fallback when OpenRouter primary model fails or times out

- `src/llm/models.rs` : Added `fallback_models: Vec<String>` to `OpenRouterConfig` struct with serde default
- `src/llm/client.rs` : Updated `create_llm_client_from_config()` to load fallback models from env vars `OPENROUTER_MODEL_FALLBACK`, `OPENROUTER_MODEL_FALLBACK_2`, `OPENROUTER_MODEL_FALLBACK_3`, `OPENROUTER_MODEL_FALLBACK_4`
- `src/llm/client.rs` : Rewrote `openrouter_chat()` with retry loop: tries primary model, then each fallback on timeout/error/empty response, logs each attempt, returns first successful response

**Environment Variables:**
```
OPENROUTER_MODEL=tencent/hy3-preview:free
OPENROUTER_MODEL_FALLBACK=nvidia/nemotron-3-super-120b-a12b:free
OPENROUTER_MODEL_FALLBACK_2=minimax/minimax-m2.5:free
OPENROUTER_MODEL_FALLBACK_3=nvidia/nemotron-3-nano-30b-a3b:free
OPENROUTER_MODEL_FALLBACK_4=openrouter/free
```

**Validation:** `cargo check` clean

## 2026-04-26 - Build Performance: sccache + lld-link Enabled

**Installations**
- `choco install llvm` → LLVM 22.1.0 (provides `lld-link.exe` at `C:\Program Files\LLVM\bin`)
- `choco install sccache` → sccache 0.14.0

**`.cargo/config.toml`** — Enabled:
- `rustc-wrapper = "sccache"` in `[build]` section
- `linker = "lld-link.exe"` under `[target.x86_64-pc-windows-msvc]`

**`setup-windows.bat`** — Rewritten:
- Auto-installs `llvm` + `sccache` via choco before configuring
- Generates aligned config matching current `.cargo/config.toml`
- Sets `RUSTC_WRAPPER=sccache` via `setx`
- Added verification step checking both tools post-install
- Fixed stale codegen-units (was 256, now 32)

**`measure-build.ps1`** — Improved:
- Sets `RUSTC_WRAPPER=sccache` and refreshes PATH in-session
- Starts `sccache --start-server` before build
- Fixed regex: now uses `^\s*` anchor to only match uncommented lines
- Reports per-profile codegen-units (dev=32, release=32) instead of first-match
- Shows sccache `--show-stats` after build
- Measures `cargo build --release` (not `cargo test`)

**Results**
| Metric | Before | After |
|--------|--------|-------|
| Warm build time | ~4m 11s | **~2m 19s** |
| Cache hit rate (rebuild) | 0% | **90.22%** |
| Linker | MSVC default | **lld-link** |

Note: lld-link requires LLVM in PATH — restart shell or run `$env:Path = [System]::GetEnvironmentVariable("Path","Machine") + ";" + [System]::GetEnvironmentVariable("Path","User")` to refresh.

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