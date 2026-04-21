# Agents Journal

## 2026-04-21 - Session Progress

### Accomplished This Session

#### Documentation Updates
- Clarified that task groups are intentionally broadcast to every active browser session.
- Added `pageview` payload alias guidance so `url` and legacy `value` stay aligned.
- Added a short task-authoring checklist near the API example.
- Softened the README status block by adding a last-verified date.

#### Human-Like Mouse Simulation Features
- **Clustered pauses** (`clustered_pause()` in timing.rs): Adds micro-movements between pauses to reduce detection patterns
- **Pointer events** (`dispatch_pointer_event()` in mouse.rs): Dispatches pointerenter, pointerleave, pointermove, pointerout around clicks
- **Element-aware hover** (`hover_before_click()`): Detects element type (button, link, input, checkbox, dropdown) and applies appropriate hover duration (60-400ms)

#### Task-API Timing Contract
- `api.pause(base_ms)` now uses a uniform 20% deviation band.
- High-level task-api verbs now add a built-in post-action settle pause after interaction.
- `api.click(selector)` is the default click path; coordinate clicks are escape hatches, not the preferred flow.

#### Code Review Fixes
- Fixed clippy warnings in reply_strategies.rs and twitteractivity_limits.rs
- Removed unused #[allow(dead_code)] from BrowserProfile, Session.profile_type, RoxybrowserConfig
- Added max_workers_per_session to BrowserConfig with validation
- Fixed orchestrator.rs timeout error handling (moved drop(task_ctx) after match)
- Added shared `pageview` target resolver for validation + task execution
- Added cooperative group timeout cancellation so cleanup can complete

#### LLM Integration
- Full OpenRouter support with automatic Ollama→OpenRouter fallback
- Standardized environment variables: LLM_PROVIDER, OLLAMA_URL (was OLLAMA_BASE_URL)

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
