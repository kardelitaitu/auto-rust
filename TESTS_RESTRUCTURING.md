# TESTS_RESTRUCTURING.md

## Objective
Clean up `src/tests/` directory by eliminating redundant tests and moving policy/unit tests to their appropriate locations, while maintaining Rust testing conventions and preserving test coverage.

---

## Current State Analysis

### Key Discovery: Redundant Tests Found
| Source File | Status | Action |
|-------------|--------|--------|
| `src/result.rs` (lines 258-953) | **Already has 50+ inline tests** | **DELETE** `src/tests/result_tests.rs` (3 redundant tests) |
| `src/validation/task_registry.rs` (lines 120-213+) | **Already has 8 inline tests** | **DELETE** `src/tests/task_registry_policy_tests.rs` (1 redundant test) |
| `src/tests/mod.rs` → `config_tests` | `src/config.rs` has NO test module | **MOVE** to `src/config.rs` |
| `src/tests/mod.rs` → `metrics_tests` | `src/metrics.rs` has NO test module | **MOVE** to `src/metrics.rs` |

### Policy Tests (Special Category)
These tests scan the codebase for pattern violations - they are NOT typical unit tests:

| File | Purpose | Target Location |
|------|---------|-----------------|
| `click_policy_tests.rs` | Forbid `page.click(` and prefer selector clicks | Root `/tests/policy/` or `/tests/click_policy_tests.rs` |
| `page_manager_policy_tests.rs` | Ensure `PageManager` not re-exported publicly | Root `/tests/policy/` or `/tests/page_manager_policy_tests.rs` |
| `blockmedia_policy_tests.rs` | Restrict `block_heavy_resources_for_cookiebot(` usage | Root `/tests/policy/` or `/tests/blockmedia_policy_tests.rs` |

---

## Improved Restructuring Plan

### Step 1: Remove Redundant Test Files
```bash
# These files test code that already has comprehensive inline tests
rm src/tests/result_tests.rs
rm src/tests/task_registry_policy_tests.rs
```

### Step 2: Move `config_tests` and `metrics_tests` to Target Files

**2a. Append to `src/config.rs` (after line 1919):**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_validate_config_valid() {
        let config = Config {
            browser: BrowserConfig {
                connectors: vec![],
                connection_timeout_ms: 10000,
                max_discovery_retries: 3,
                discovery_retry_delay_ms: 5000,
                circuit_breaker: CircuitBreakerConfig {
                    enabled: true,
                    failure_threshold: 5,
                    success_threshold: 3,
                    half_open_time_ms: 30000,
                },
                profiles: vec![],
                roxybrowser: RoxybrowserConfig {
                    enabled: true,
                    api_url: "http://localhost".to_string(),
                    api_key: "key".to_string(),
                },
                user_agent: None,
                extra_http_headers: BTreeMap::new(),
                cursor_overlay_ms: 0,
                native_interaction: NativeInteractionConfig::default(),
                max_workers_per_session: 5,
            },
            orchestrator: OrchestratorConfig {
                max_global_concurrency: 5,
                task_timeout_ms: 60_000,
                group_timeout_ms: 120_000,
                worker_wait_timeout_ms: 10000,
                stuck_worker_threshold_ms: 60_000,
                task_stagger_delay_ms: 1000,
                max_retries: 2,
                retry_delay_ms: 500,
            },
            twitter_activity: TwitterActivityConfig::default(),
            tracing: TracingConfig::default(),
        };
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_invalid_concurrency() {
        let config = Config {
            browser: BrowserConfig {
                connectors: vec![],
                connection_timeout_ms: 10000,
                max_discovery_retries: 3,
                discovery_retry_delay_ms: 5000,
                circuit_breaker: CircuitBreakerConfig {
                    enabled: true,
                    failure_threshold: 5,
                    success_threshold: 3,
                    half_open_time_ms: 30000,
                },
                profiles: vec![],
                roxybrowser: RoxybrowserConfig {
                    enabled: true,
                    api_url: "http://localhost".to_string(),
                    api_key: "key".to_string(),
                },
                user_agent: None,
                extra_http_headers: BTreeMap::new(),
                cursor_overlay_ms: 0,
                native_interaction: NativeInteractionConfig::default(),
                max_workers_per_session: 5,
            },
            orchestrator: OrchestratorConfig {
                max_global_concurrency: 0,  // Invalid
                task_timeout_ms: 60_000,
                group_timeout_ms: 120_000,
                worker_wait_timeout_ms: 10000,
                stuck_worker_threshold_ms: 60_000,
                task_stagger_delay_ms: 1000,
                max_retries: 2,
                retry_delay_ms: 500,
            },
            twitter_activity: TwitterActivityConfig::default(),
            tracing: TracingConfig::default(),
        };
        assert!(validate_config(&config).is_err());
    }
}
```

**2b. Append to `src/metrics.rs` (after line 1138):**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::TaskStatus;

    #[test]
    fn test_metrics_collector_new() {
        let mc = MetricsCollector::new(100);
        let stats = mc.get_stats();
        assert_eq!(stats.total_tasks, 0);
        assert_eq!(stats.active_tasks, 0);
    }

    #[test]
    fn test_metrics_success_rate() {
        let mc = MetricsCollector::new(100);
        mc.task_started();
        mc.task_completed(TaskMetrics {
            task_name: "test".to_string(),
            status: TaskStatus::Success,
            duration_ms: 100,
            session_id: "s1".to_string(),
            attempt: 1,
            error_kind: None,
            last_error: None,
        });
        assert_eq!(mc.success_rate(), 100.0);
    }
}
```

### Step 3: Move Policy Tests to Root `/tests/`

Policy tests should be integration tests since they verify project-wide conventions:

```bash
# Create policy subdirectory (optional, for organization)
mkdir -p tests/policy

# Move policy tests
mv src/tests/click_policy_tests.rs tests/policy/
mv src/tests/page_manager_policy_tests.rs tests/policy/
mv src/tests/blockmedia_policy_tests.rs tests/policy/
```

Update imports in moved files - change `crate::` to `auto::` where necessary.

### Step 4: Clean Up `src/tests/mod.rs`
After moving tests, the mod.rs will be empty:
```bash
rm src/tests/mod.rs
```

### Step 5: Remove `src/tests/` Directory and `lib.rs` Reference
```bash
rmdir src/tests  # After all files are moved/deleted
```

In `src/lib.rs`, remove line 32:
```diff
- pub mod tests;
```

---

## Final Directory Structure

```
auto-rust/
├── tests/                          # Integration & policy tests
│   ├── policy/
│   │   ├── click_policy_tests.rs
│   │   ├── page_manager_policy_tests.rs
│   │   └── blockmedia_policy_tests.rs
│   ├── chaos_failure_classification.rs
│   ├── cli_parsing_tests.rs
│   ├── graceful_shutdown_integration.rs
│   ├── launcher_tests.rs
│   ├── soak_test.rs
│   ├── task_api_behavior.rs
│   ├── task_registration_tests.rs
│   └── twitteractivity_integration.rs
├── src/
│   ├── lib.rs                     # No more `pub mod tests;`
│   ├── config.rs                  # + config_tests moved here
│   ├── metrics.rs                 # + metrics_tests moved here
│   ├── result.rs                  # Already has 50+ inline tests
│   ├── validation/
│   │   └── task_registry.rs      # Already has inline tests
│   └── ... (other modules with inline #[cfg(test)])
```

---

## Test Count Summary

| Location | Before | After | Change |
|----------|---------|--------|--------|
| `src/result.rs` inline | 50+ | 50+ | No change (delete redundant `src/tests/result_tests.rs`) |
| `src/validation/task_registry.rs` | 8 | 8 | No change (delete redundant `src/tests/task_registry_policy_tests.rs`) |
| `src/config.rs` inline | 0 | 2 | +2 from `mod.rs` |
| `src/metrics.rs` inline | 0 | 2 | +2 from `mod.rs` |
| `tests/policy/` | 0 | 4 | +4 policy tests moved |
| **Total** | **~84** | **~84** | **No net loss** |

---

## Validation Steps
1. **Compile**: `cargo check` (must pass)
2. **Test**: `cargo test` (all tests must pass)
3. **Verify no redundant tests**: `cargo test 2>&1 | grep "test result"` should show same count
4. **Clean up**: Confirm `src/tests/` directory is removed

---

## Key Improvements Over Previous Plan
1. **Identifies redundant tests** - `result_tests.rs` and `task_registry_policy_tests.rs` are unnecessary
2. **Preserves all test coverage** - No tests are lost, only reorganized
3. **Correctly categorizes policy tests** - They belong in `/tests/` as integration tests
4. **Provides exact code to move** - Copy-paste ready Rust code blocks
5. **Includes test count validation** - Ensures no regressions
