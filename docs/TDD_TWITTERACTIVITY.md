# TDD Workflow: Twitter Activity Module

**Version:** 1.0  
**Last Updated:** May 21, 2026  
**Scope:** `src/task/twitteractivity.rs` + `src/utils/twitter/*.rs` (18 component files)

---

## Overview

This document defines the Test-Driven Development (TDD) workflow for the
Twitter Activity automation module. It follows the classic **Red-Green-Refactor**
cycle with automated tooling support.

### Why TDD for twitteractivity?

- **18 component files** with complex interactions (navigation, engagement,
  feed scanning, LLM integration, retry logic)
- **Behavioral correctness** is critical — errors mean missed engagements
  or account restrictions
- **Refactoring confidence** — comprehensive test suite prevents regressions
- **Documented behavior** — tests serve as executable specifications

---

## TDD Cycle

```
┌─────────────────────────────────────────────┐
│                1. RED                        │
│   Write a failing test for desired behavior  │
│   Run: .\run-twitter-tests.ps1 -Red         │
│   Expect: test FAILS (behavior unimplemented)│
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│                2. GREEN                      │
│   Write minimal code to make the test pass  │
│   Run: .\run-twitter-tests.ps1 -Green      │
│   Expect: test PASSES (behavior implemented) │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│              3. REFACTOR                     │
│   Clean up code (rename, extract, simplify)  │
│   Run: .\run-twitter-tests.ps1 -Refactor   │
│   Expect: ALL tests PASS (no regressions)    │
└──────────────┬──────────────────────────────┘
               │
               └─── Repeat ──────────────────►
```

---

## Test Organization

### Test Location

| Test Type | Location | Purpose |
|---|---|---|
| Unit tests | `src/task/twitteractivity.rs` → `#[cfg(test)] mod *` | Function-level tests (orchestrator) |
| Unit tests | `src/utils/twitter/*.rs` → `#[cfg(test)] mod tests` | Module-level tests (components) |
| Integration tests | `tests/twitteractivity_integration.rs` | Cross-module and public API tests |
| Test helpers | `tests/common/twitter_helpers.rs` | Reusable builders, configs, assertions |

### Test Naming Conventions

```
[tdd_stage]_[module]_[behavior]_[scenario]

Examples:
  tdd_green_session_state_creation
  tdd_red_engagement_limit_reached_stops_actions
  tdd_refactor_limits_still_enforced_after_extraction
  tdd_edge_session_expiry_edge_case
  tdd_regression_retry_exhaustion_returns_last_error
```

### Test Tagging Convention

All tests should use one of the following prefixes in their function name
for the TDD runner to discover them:

| Prefix | Meaning | Runner Flag |
|---|---|---|
| `tdd_red_*` | Test for unimplemented behavior (expected to fail) | `-Red` |
| `tdd_green_*` | Test for implemented behavior (should pass) | `-Green` |
| `tdd_refactor_*` | Test for behavior that must survive refactoring | `-Refactor` |
| `tdd_edge_*` | Edge case test | `-Red` or `-Green` |
| `tdd_regression_*` | Regression prevention test | `-Green` |

---

## Writing Tests

### 1. Write a RED Test (Phase 1)

Create a test that describes the desired behavior before implementing it.

```rust
// In src/utils/twitter/twitteractivity_limits.rs

#[cfg(test)]
mod tdd_tests {
    use super::*;

    #[test]
    fn tdd_red_engagement_limit_blocks_after_specific_count() {
        // Arrange: Create limits with max_likes = 3
        let limits = EngagementLimits::with_limits(3, 5, 2, 1, 3, 2, 2, 20);
        let mut counters = EngagementCounters::new();

        // Act: Perform 3 likes
        for _ in 0..3 {
            counters.increment_like();
        }

        // Assert: 4th like should be blocked
        assert!(!limits.can_like(&counters),
            "4th like should be blocked when max_likes = 3");
    }
}
```

**Verify:** `.\run-twitter-tests.ps1 -Red`  
**Expected:** Test fails (behavior not yet implemented)

### 2. Make it GREEN (Phase 2)

Write the minimal code to pass the test:

```rust
// In EngagementLimits::can_like()
pub fn can_like(&self, counters: &EngagementCounters) -> bool {
    counters.likes < self.max_likes  // This already exists!
}
```

**Verify:** `.\run-twitter-tests.ps1 -Green`  
**Expected:** Test passes

### 3. REFACTOR (Phase 3)

Clean up the code while keeping tests green:

```rust
// Extract common pattern if needed
pub fn can_perform_action(&self, current: u32, max: u32, total: u32, max_total: u32) -> bool {
    current < max && total < max_total
}
```

**Verify:** `.\run-twitter-tests.ps1 -Refactor`  
**Expected:** All tests pass

---

## Test Helpers Reference

Located in `tests/common/twitter_helpers.rs`. Import with:

```rust
use crate::tests::twitter_helpers::*;
```

### Tweet Builders

| Function | Description |
|---|---|
| `build_tweet(text, author)` | Standard tweet with given text and author |
| `build_positive_tweet()` | Tweet with positive sentiment content |
| `build_negative_tweet()` | Tweet with negative sentiment content |
| `build_neutral_tweet()` | Tweet with neutral sentiment content |
| `build_tweet_with_media()` | Tweet with image/video media |
| `build_thread_tweet()` | Tweet that starts a thread |
| `build_verified_tweet()` | Tweet from verified account |
| `build_tweet_with_replies(n)` | Tweet with `n` replies |
| `build_tweet_with_metrics(l, rt, rp, v)` | Tweet with specific metric values |
| `build_promotional_tweet()` | Tweet that looks like an ad |

### Config Factories

| Function | Description |
|---|---|
| `test_twitter_config()` | Default `TwitterActivityConfig` |
| `test_task_config()` | Default `TaskConfig` (simulate_only, dry_run) |
| `test_task_config_with_limits(like, rt, f, rp, tot)` | TaskConfig for engagement testing |
| `test_payload()` | Standard JSON test payload |

### Session State Builders

| Function | Description |
|---|---|
| `test_session_state()` | SessionState with default limits, 60s duration |
| `test_session_state_with_limits(...)` | SessionState with custom limits |
| `test_action_tracker(delay_ms)` | TweetActionTracker for chain testing |
| `test_counters_with_actions(l, rt, f, rp)` | Counters with pre-recorded actions |

### Persona Helpers

| Function | Description |
|---|---|
| `test_persona_weights()` | Default persona weights (balanced) |
| `test_persona_favoring_likes()` | 95% like probability |
| `test_persona_favoring_replies()` | 95% reply probability |

### Assertion Helpers

| Function | Description |
|---|---|
| `assert_session_valid(session, max)` | Verify session in valid initial state |
| `assert_action_allowed(session, id, action)` | Assert action allowed + record it |
| `assert_action_blocked(session, action)` | Assert action blocked by limit/cooldown |
| `assert_total_actions(session, expected)` | Check total action count |
| `assert_remaining_time_approx(session, ms, tol)` | Check remaining time within tolerance |
| `assert_tracker_allows(tracker, id, action)` | Assert tracker allows action |
| `assert_tracker_blocks(tracker, id, action)` | Assert tracker blocks action (cooldown) |
| `assert_all_actions_allowed(limits, counters)` | Assert all 7 action types allowed |
| `assert_all_actions_blocked(limits, counters)` | Assert all 7 action types blocked |

### Error Helpers

| Function | Description |
|---|---|
| `transient_error(msg)` | Creates a transient error string |
| `permanent_error(msg)` | Creates a permanent error string |
| `fatal_error(msg)` | Creates a fatal error string |

---

## Running Tests

### Quick Reference

```powershell
# Run all twitter tests
.\run-twitter-tests.ps1

# Run RED tests (expected failures)
.\run-twitter-tests.ps1 -Red

# Run GREEN tests (passing validation)
.\run-twitter-tests.ps1 -Green

# Run ALL tests for refactoring verification
.\run-twitter-tests.ps1 -Refactor

# Watch mode (re-runs on file changes)
.\run-twitter-tests.ps1 -Watch

# With coverage instrumentation
.\run-twitter-tests.ps1 -Coverage

# Profile slow tests
.\run-twitter-tests.ps1 -Profile

# Unit tests only (faster)
.\run-twitter-tests.ps1 -Fast

# Integration tests only
.\run-twitter-tests.ps1 -Integration

# Custom filter
.\run-twitter-tests.ps1 -Filter "session_state"
```

### Direct cargo commands

```bash
# Run all unit tests
cargo test --lib

# Run integration tests only
cargo test --test twitteractivity_integration

# Run a specific test
cargo test --test twitteractivity_integration session_state

# Run with filter on test name
cargo test --lib -- twitteractivity

# Run with full output (no capture)
cargo test -- --nocapture

# Run a single test function
cargo test twitteractivity_session_state_creation -- --exact
```

---

## Test Coverage Requirements

All new code must include tests at the appropriate level:

| Coverage Type | Target | Minimum |
|---|---|---|
| Unit tests | Pure functions, data transformations | 100% of new code |
| Unit tests | Error paths, edge cases | Every new branch |
| Integration tests | Module interactions | Every new public function |
| Integration tests | Configuration parsing | Default + custom values |
| Integration tests | Session lifecycle | Creation, expiry, overflow |

### Coverage command

```bash
cargo tarpaulin --lib --test twitteractivity_integration --out Html
```

Or with the custom script:

```bash
.\run-twitter-tests.ps1 -Coverage
```

---

## TDD Best Practices

### DO's

- **Start with a RED test** — always write the test first
- **Test one behavior per test** — single assertion preferred
- **Use descriptive test names** — `tdd_green_like_limit_respected_when_exceeded`
- **Run the RED phase first** — verify the test fails before implementing
- **Run GREEN after implementation** — verify the test passes
- **Run REFACTOR after cleanup** — verify no regressions
- **Use test helpers** — `tests/common/twitter_helpers.rs` has builders for everything
- **Tag tests by TDD phase** — prefix with `tdd_red_`, `tdd_green_`, or `tdd_refactor_`

### DON'Ts

- **Don't write implementation without a test** — TDD is test-first
- **Don't skip the RED phase** — you need to see it fail to trust it passes
- **Don't write tests that depend on other tests** — each test must be independent
- **Don't test implementation details** — test public behavior only
- **Don't use sleep() in tests** — use mock time or instant advances
- **Don't skip edge cases** — empty state, zero values, max values, error conditions

### Integration test guidelines

- Integration tests that require a browser are marked `#[ignore]`
- All other tests must be fast and deterministic
- Use `simulate_only: true` for task-level tests
- Mock `TaskContext` with `MockPageContext` for browser-dependent tests

---

## Test File Template

When adding a new component file under `src/utils/twitter/`, include:

```rust
//! [Component description]

// ... implementation ...

#[cfg(test)]
mod tdd_tests {
    use super::*;

    // === RED Tests (unimplemented behavior) ===

    #[test]
    fn tdd_red_my_function_expected_behavior() {
        // TODO: Write test for desired behavior
    }

    // === GREEN Tests (implemented behavior) ===

    #[test]
    fn tdd_green_my_function_works_as_expected() {
        // Verify working behavior
    }

    // === EDGE Case Tests ===

    #[test]
    fn tdd_edge_my_function_handles_empty_input() {
        // Test with empty/zero/null input
    }

    // === REGRESSION Tests ===

    #[test]
    fn tdd_regression_my_function_does_not_regress() {
        // Test for previously fixed bugs
    }
}
```

---

## Troubleshooting

| Problem | Solution |
|---|---|
| RED tests pass unexpectedly | The behavior is already implemented — move test to GREEN |
| GREEN tests fail | Fix implementation before refactoring |
| Watch mode not available | `cargo install cargo-watch` |
| Coverage tools not found | `cargo install grcov` + `rustup component add llvm-tools-preview` |
| Slow tests identified by -Profile | Add `#[ignore]` to slow tests, move to integration test file |
| Test panics with "already borrowed" | Use separate tweets/IDs for each test |
| Integration test requires browser | Mark with `#[ignore]` and document required env vars |

---

## Appendix: File Map

```
src/
├── task/
│   └── twitteractivity.rs          ← Orchestrator (~100 lines)
└── utils/twitter/
    ├── mod.rs                      ← Re-exports
    ├── twitteractivity_constants.rs ← Timing constants
    ├── twitteractivity_dive.rs      ← Thread diving logic
    ├── twitteractivity_engagement.rs← Tweet processing
    ├── twitteractivity_errors.rs    ← Error classification
    ├── twitteractivity_feed.rs      ← Feed scanning
    ├── twitteractivity_humanized.rs ← Human-like pauses
    ├── twitteractivity_interact.rs  ← DOM interactions
    ├── twitteractivity_limits.rs    ← Engagement limits
    ├── twitteractivity_llm.rs       ← LLM integration
    ├── twitteractivity_llm_execute.rs ← LLM execution
    ├── twitteractivity_llm_validation.rs ← Reply validation
    ├── twitteractivity_navigation.rs← Navigation/entry points
    ├── twitteractivity_persona.rs   ← Behavior profiles
    ├── twitteractivity_popup.rs     ← Popup handling
    ├── twitteractivity_retry.rs     ← Retry/circuit breaker
    ├── twitteractivity_selectors.rs ← DOM selectors
    ├── twitteractivity_simulation.rs← Dry-run simulation
    └── twitteractivity_state.rs     ← State/context types
tests/
├── common/
│   ├── mod.rs                      ← Test infrastructure
│   └── twitter_helpers.rs          ← TDD helpers (NEW)
└── twitteractivity_integration.rs  ← Integration tests
```

---

*This TDD workflow is designed to maintain code quality and prevent
regressions as the twitteractivity module evolves. All team members
are expected to follow the Red-Green-Refactor cycle when adding
or modifying functionality.*
