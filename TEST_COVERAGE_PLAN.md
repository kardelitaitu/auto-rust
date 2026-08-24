# Test Coverage Improvement Plan

> **Date:** August 24, 2026  
> **Current state:** 4,171 tests | 1 pre-existing failure | 6 ignored  
> **Files with zero tests:** ~55 out of ~200 source files  
> **Goal:** Close critical gaps first, then improve ratios

## Current State Summary

### Well-Tested (good ratio, >5 tests/100 lines)
| File | Tests | Lines | Ratio |
|---|---|---|---|
| `result/mod.rs` | 121 | 1114 | 10.9 |
| `result/errors.rs` | 106 | 1148 | 9.2 |
| `utils/text.rs` | 52 | 381 | 13.6 |
| `utils/math.rs` | 44 | 431 | 10.2 |
| `error.rs` | 56 | 855 | 6.5 |
| `utils/blockmedia.rs` | 54 | 540 | 10.0 |

### Completely Untested (zero `#[test]` functions)
These files contain production logic with **no unit tests at all**:

| File | Lines | Risk | Why it matters |
|---|---|---|---|
| `task/validation/validator.rs` | 872 | 🔴 Critical | DSL task validation — wrong validation = broken tasks |
| `runtime/task_context/mod.rs` | 787 | 🔴 Critical | TaskContext creation — core execution context |
| `config/types.rs` | 477 | 🟠 High | All config structs — serialization/deserialization errors |
| `runtime/task_context/session_io.rs` | 417 | 🟠 High | Session save/load — data corruption risk |
| `config/env.rs` | 386 | 🟠 High | Env var overrides — parsing errors silently wrong |
| `task/dsl/api.rs` | 327 | 🟠 High | DSL API layer — task execution errors |
| `session/worker.rs` | 252 | 🔴 Critical | Page acquire/release, circuit breaker — concurrency bugs |
| `utils/mouse/overlay.rs` | 593 | 🟡 Medium | Cursor overlay — visual bugs, not correctness |
| `utils/mouse/cdp.rs` | 209 | 🟡 Medium | CDP mouse commands — browser interaction |
| `runtime/task_context/cookies.rs` | 151 | 🟠 High | Cookie handling — session persistence errors |
| `runtime/task_context/http.rs` | 105 | 🟠 High | HTTP requests in tasks — network error handling |
| `session/lifecycle.rs` | 124 | 🟡 Medium | Health/state management — simple getters but critical path |
| `session/permits.rs` | 33 | 🟢 Low | WorkerPermit RAII — small, straightforward |
| `session/state.rs` | 46 | 🟢 Low | SessionState enum — trivial types |
| `utils/mouse/curves.rs` | 70 | 🟢 Low | Bezier curves — math, low risk |
| `utils/twitter/engagement/scoring.rs` | 147 | 🟡 Medium | Engagement scoring — affects task decisions |

### Under-Tested (low ratio, many lines, few tests)
| File | Tests | Lines | Gap |
|---|---|---|---|
| `task/cookiebot.rs` | 1 | 150 | Only a smoke test, no error path coverage |
| `task/demo-interaction-keyboard.rs` | 1 | 112 | Only a smoke test |
| `utils/twitter/twitteractivity_persistence.rs` | 11 | 335 | Missing edge cases for save/load corruption |
| `adaptive/learning_engine.rs` | 11 | 423 | Missing convergence and error path tests |
| `health_logger.rs` | 10 | 333 | Missing threshold warning tests |
| `llm/unified_processor.rs` | varies | large | Needs integration-level processor tests |

---

## Priority Plan

### Phase 1: Critical Path (Week 1) — Zero-test production logic

**1.1 `session/worker.rs`** (252 lines, 0 tests)
- `acquire_worker()` — timeout path, circuit breaker rejection, semaphore exhaustion
- `acquire_page()` — circuit breaker check, success/failure recording
- `release_page()` — overlay cleanup, page unregister
- `cleanup_managed_pages()` — mixed tracked/untracked pages
- `graceful_shutdown()` — page close, browser close, task abort

**1.2 `task/validation/validator.rs`** (872 lines, 0 tests) — ✅ covered (68 tests)
- `validate_action()` — all action types (navigate, click, type, scroll, wait, etc.)
- `validate_include()` — nested task references, circular detection
- `validate_task_definition()` — required fields, type checking
- Edge cases: empty actions, invalid selectors, unknown action types

**1.3 `runtime/task_context/mod.rs`** (787 lines, 0 tests)
- `TaskContext::new()` — field initialization, policy attachment
- `pause()` / `pause_human()` — variance calculation, cancellation
- `navigate()` — timeout handling, error propagation
- `wait_for_visible()` — timeout, selector not found

**1.4 `session/lifecycle.rs`** (124 lines, 0 tests)
- `is_circuit_breaker_open()` — threshold, timeout, wraparound (already tested in mod.rs but verify)
- `has_available_workers()` — healthy+capacity, unhealthy, full
- Health transition tests (mark_healthy, mark_unhealthy, increment_failure)

### Phase 2: Config & Serialization (Week 1-2)

**2.1 `config/types.rs`** (477 lines, 0 tests)
- `Config` serde round-trip (serialize → deserialize → compare)
- `BrowserConfig` defaults
- `OrchestratorConfig` defaults
- `TwitterActivityConfig` defaults
- `DurationMs` serde behavior
- Missing fields → default values

**2.2 `config/env.rs`** (386 lines, 0 tests)
- `load_dotenv_defaults()` — key=value parsing, quote stripping, skip existing
- `apply_env_overrides()` — each env var override
- `load_code_config()` — fallback config construction
- Edge cases: empty values, malformed lines, missing = sign

**2.3 `runtime/task_context/session_io.rs`** (417 lines, 0 tests)
- Session data save/load round-trip
- Corrupted file handling
- Missing file handling
- Large session data

### Phase 3: DSL & Task Execution (Week 2)

**3.1 `task/dsl/api.rs`** (327 lines, 0 tests)
- DSL action execution dispatch
- Variable interpolation
- Error propagation from actions
- Timeout handling

**3.2 `task/cookiebot.rs`** (150 lines, 1 test)
- Full run path with mock browser
- Error paths: navigation failure, element not found
- Data file loading

**3.3 `utils/twitter/engagement/scoring.rs`** (147 lines, 0 tests)
- Score calculation for various tweet types
- Edge cases: empty author, missing engagement data

### Phase 4: Mouse & CDP (Week 2-3)

**4.1 `utils/mouse/cdp.rs`** (209 lines, 0 tests)
- CDP mouse event dispatch
- Coordinate transformation
- Error handling for disconnected pages

**4.2 `utils/mouse/overlay.rs`** (593 lines, 0 tests)
- Overlay state machine
- Active page tracking
- Cleanup on page close

**4.3 `utils/mouse/curves.rs`** (70 lines, 0 tests)
- Bezier curve generation
- Point distribution
- Edge cases: zero distance, single point

### Phase 5: Edge Cases & Robustness (Week 3)

**5.1 `runtime/task_context/cookies.rs`** (151 lines, 0 tests)
- Cookie extraction from page
- Format conversion
- Empty cookie handling

**5.2 `runtime/task_context/http.rs`** (105 lines, 0 tests)
- HTTP request execution
- Timeout handling
- Error classification

**5.3 Under-tested files enhancement**
- `twitteractivity_persistence.rs` — corruption recovery, concurrent access
- `adaptive/learning_engine.rs` — convergence guarantees, reset behavior
- `health_logger.rs` — threshold boundary tests

---

## Testing Patterns to Follow

### Unit Test Convention (matching existing codebase)
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_name_expected_behavior() {
        // Arrange
        // Act
        // Assert
    }
}
```

### Async Test Convention
```rust
#[tokio::test]
    async fn function_name_expected_behavior() {
        // Arrange
        // Act
        // Assert
    }
```

### Naming Convention (existing pattern)
- `test_struct_behavior_condition` — e.g., `test_circuit_breaker_opens_at_threshold`
- `tdd_green_description` — for TDD-style tests
- Descriptive, not abbreviated

### Test File Organization
- **Inline:** Tests in the same file as implementation (current pattern for most modules)
- **Separate test files:** Only for complex integration tests (e.g., `src/task/validation/tests.rs`)
- **Test utilities:** Shared helpers in `src/orchestrator/test_utils.rs` pattern

---

## Estimated Impact

| Phase | New Tests | Coverage Gain | Risk Reduction |
|---|---|---|---|
| Phase 1 | ~80-100 | Critical path: 0% → 70% | 🔴 → 🟢 |
| Phase 2 | ~40-50 | Config: 20% → 80% | 🟠 → 🟢 |
| Phase 3 | ~50-60 | DSL/Tasks: 30% → 70% | 🟠 → 🟡 |
| Phase 4 | ~30-40 | Mouse/CDP: 0% → 60% | 🟡 → 🟢 |
| Phase 5 | ~30-40 | Edge cases: 40% → 75% | 🟡 → 🟢 |
| **Total** | **~230-290** | **Overall: ~60% → 80%** | |

---

## Quick Wins (can do now, < 30 min each)

1. **`session/lifecycle.rs`** — Simple getters/setters, easy to test
2. **`session/permits.rs`** — 33 lines, RAII drop test
3. **`session/state.rs`** — 46 lines, enum variant tests
4. **`utils/mouse/curves.rs`** — 70 lines, pure math functions
5. **`task/cookiebot.rs`** — Add error path tests to existing smoke test

---

## What NOT to Test

- `src/bin/*` — Binary entry points, tested via integration tests
- `src/benchmarks/*` — Benchmarks, not correctness
- `src/lib.rs`, `src/main.rs` — Module declarations only
- `src/session/factory.rs` — Requires real browser connections (integration test territory)
- `src/session/connector.rs` — Already well-tested (48 tests), needs only edge cases
