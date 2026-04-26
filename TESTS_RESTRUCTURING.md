# TESTS_RESTRUCTURING.md

## Objective
Clean up `src/tests/` directory by eliminating redundant tests and moving policy/unit tests to their appropriate locations, while maintaining Rust testing conventions and preserving test coverage.

---

## Current State Analysis

### Key Discovery: Test Coverage Audit

| Source File | Status | Action |
|-------------|--------|--------|
| `src/result.rs` (lines 258-953) | **Has 50+ inline tests** | **DELETE** `src/tests/result_tests.rs` (3 redundant tests - all covered) |
| `src/validation/task_registry.rs` (lines 120-213+) | **Has 8 inline tests** | **MOVE** `src/tests/task_registry_policy_tests.rs` to `tests/` (tests `twitterreply`/`demoqa` not covered in source) |
| `src/config.rs` (lines 578-900+) | **Has 25+ tests for struct defaults** | **MOVE** `config_tests` from `src/tests/mod.rs` (tests `validate_config()` not covered) |
| `src/metrics.rs` (line 707+) | **HAS existing `#[cfg(test)] mod tests` with 4+ tests** | **MERGE** `metrics_tests` from `src/tests/mod.rs` into existing mod (add only missing tests) |
| `src/tests/mod.rs` | Contains 5 module refs (`click_policy_tests`, `page_manager_policy_tests`, `blockmedia_policy_tests`, `result_tests`, `task_registry_policy_tests`) + inline `config_tests`/`metrics_tests` mods | **DELETE** after removing all mod refs and inline test mods |

### Policy Tests (Special Category)
These tests scan the codebase for pattern violations - they are NOT typical unit tests:

| File | Purpose | Target Location |
|------|---------|-----------------|
| `click_policy_tests.rs` | Forbid `page.click(` and prefer selector clicks | `tests/policy/click_policy_tests.rs` |
| `page_manager_policy_tests.rs` | Ensure `PageManager` not re-exported publicly | `tests/policy/page_manager_policy_tests.rs` |
| `blockmedia_policy_tests.rs` | Restrict `block_heavy_resources_for_cookiebot(` usage | `tests/policy/blockmedia_policy_tests.rs` |

---

## Improved Restructuring Plan

### Step 1: Remove Redundant Test File
**CRITICAL**: `src/tests/mod.rs` contains `mod result_tests;` - remove this line first to avoid dangling module error:
1. Edit `src/tests/mod.rs` - delete the line `mod result_tests;`
2. Delete the redundant test file:
```bash
# Windows PowerShell
Remove-Item -Path "C:\My Script\auto-rust\src\tests\result_tests.rs"
# Unix
rm src/tests/result_tests.rs
```

**Note**: All shell commands in this document are written for Unix-like systems. Windows (PowerShell) users must use equivalent commands:
- `rm <file>` → `Remove-Item -Path "C:\My Script\auto-rust\<file>"`
- `mv <src> <dest>` → `Move-Item -Path "C:\My Script\auto-rust\<src>" -Destination "C:\My Script\auto-rust\<dest>"`
- `mkdir -p <dir>` → `New-Item -ItemType Directory -Path "C:\My Script\auto-rust\<dir>" -Force`
- `rmdir <dir>` → `Remove-Item -Path "C:\My Script\auto-rust\<dir>" -Recurse`

### Step 2: Add `validate_config` Tests to `src/config.rs`'s Existing Test Module

**Note**: `src/config.rs` already has an existing `#[cfg(test)] mod tests` (lines 578-900+) with 25+ tests for struct defaults, but does NOT test the `validate_config()` function. Add the following test functions to the existing `tests` mod (inside the `#[cfg(test)]` block):

Add the following test functions inside the existing `#[cfg(test)] mod tests` block in `src/config.rs` (the existing mod already includes `use super::*;` and `use std::collections::BTreeMap;`):

```rust
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
```

### Step 3: Merge `metrics_tests` into Existing `src/metrics.rs` Test Module

**Important**: `src/metrics.rs` already has a `#[cfg(test)] mod tests` starting at line 707 with 4+ tests (including `test_metrics_collector_new`). Do NOT create a duplicate module.

**Action**: Add only the missing test(s) to the existing `tests` mod in `src/metrics.rs` (before the closing `}` of the module):

```rust
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
```

**Note**: The existing `test_metrics_collector_new` in `src/metrics.rs` already covers the same functionality as the one in `src/tests/mod.rs`'s `metrics_tests`. No need to add duplicate.

### Step 4: Move Policy Tests to `tests/` (Flat Structure)

Policy tests should be integration tests since they verify project-wide conventions. **Do NOT use a subdirectory** - Rust's test harness does not discover integration tests in `tests/` subdirectories.

```bash
# Move policy tests directly to tests/ (they will be discovered as separate test crates)
mv src/tests/click_policy_tests.rs tests/click_policy_tests.rs
mv src/tests/page_manager_policy_tests.rs tests/page_manager_policy_tests.rs
mv src/tests/blockmedia_policy_tests.rs tests/blockmedia_policy_tests.rs
# Windows PowerShell
Move-Item -Path "C:\My Script\auto-rust\src\tests\click_policy_tests.rs" -Destination "C:\My Script\auto-rust\tests\click_policy_tests.rs"
Move-Item -Path "C:\My Script\auto-rust\src\tests\page_manager_policy_tests.rs" -Destination "C:\My Script\auto-rust\tests\page_manager_policy_tests.rs"
Move-Item -Path "C:\My Script\auto-rust\src\tests\blockmedia_policy_tests.rs" -Destination "C:\My Script\auto-rust\tests\blockmedia_policy_tests.rs"
```

**Important**: Update imports in moved files:
1. For `click_policy_tests.rs`: Change `use std::fs;` (already fine) - no `crate::` imports needed.
2. For `page_manager_policy_tests.rs` and `blockmedia_policy_tests.rs`: If they use `crate::`, change to `auto::` (e.g., `use auto::lib::...`).

**Note**: Also remove `mod click_policy_tests;`, `mod page_manager_policy_tests;`, `mod blockmedia_policy_tests;` lines from `src/tests/mod.rs` after moving.

### Step 5: Move `task_registry_policy_tests.rs` to `tests/`

This test checks that specific tasks (`twitterreply`, `demoqa`) are registered - this complements existing tests:

```bash
# Move to tests/
mv src/tests/task_registry_policy_tests.rs tests/task_registry_policy_tests.rs
# Windows PowerShell
Move-Item -Path "C:\My Script\auto-rust\src\tests\task_registry_policy_tests.rs" -Destination "C:\My Script\auto-rust\tests\task_registry_policy_tests.rs"
```

**CRITICAL**: Update imports in the moved file:
1. Change `use crate::{cli, task};` to `use auto::{cli, task};`
2. Change `task::TASK_NAMES` references to `auto::task::TASK_NAMES` (or import via `use auto::task::TASK_NAMES;`)

**Alternative**: Merge the test function into `tests/task_registration_tests.rs`:
```rust
#[test]
fn canonical_task_registry_includes_twitterreply_and_demoqa() {
    use auto::task::{is_known_task, TASK_NAMES};
    assert!(is_known_task("twitterreply"));
    assert!(is_known_task("demoqa"));
    assert!(TASK_NAMES.contains(&"twitterreply"));
    assert!(TASK_NAMES.contains(&"demoqa"));
}
```

**Note**: Remove `mod task_registry_policy_tests;` from `src/tests/mod.rs` after moving.

### Step 6: Clean Up `src/tests/`

After moving all tests and removing all module references from `src/tests/mod.rs`:
1. Remove all `mod ...;` lines (`click_policy_tests`, `page_manager_policy_tests`, `blockmedia_policy_tests`, `result_tests`, `task_registry_policy_tests`)
2. Remove the inline `#[cfg(test)] mod config_tests { ... }` and `#[cfg(test)] mod metrics_tests { ... }` blocks
3. Verify `mod.rs` is empty, then delete:
```bash
# Remove the now-empty mod.rs
rm src/tests/mod.rs
# Windows PowerShell
Remove-Item -Path "C:\My Script\auto-rust\src\tests\mod.rs"

# Remove the directory
rmdir src/tests
# Windows PowerShell
Remove-Item -Path "C:\My Script\auto-rust\src\tests" -Recurse
```

### Step 7: Remove `tests` Module Reference from `src/lib.rs`

In `src/lib.rs`, remove line 32:
```diff
- pub mod tests;
```

---

## Final Directory Structure

```
auto-rust/
├── tests/                          # Integration & policy tests
│   ├── click_policy_tests.rs      # Moved from src/tests/ (policy test)
│   ├── page_manager_policy_tests.rs # Moved from src/tests/ (policy test)
│   ├── blockmedia_policy_tests.rs  # Moved from src/tests/ (policy test)
│   ├── task_registry_policy_tests.rs  # Moved from src/tests/
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
│   ├── config.rs                  # + validate_config tests added to existing mod
│   ├── metrics.rs                 # + test_metrics_success_rate added to existing mod
│   ├── result.rs                  # Keeps 50+ inline tests (result_tests.rs deleted)
│   ├── validation/
│   │   └── task_registry.rs      # Keeps 8 inline tests
│   └── ... (other modules with inline #[cfg(test)])
```

---

## Test Count Estimate

| Location | Before | After | Change |
|----------|---------|--------|--------|
| `src/result.rs` inline | 50+ | 50+ | -3 (delete `src/tests/result_tests.rs`) |
| `src/validation/task_registry.rs` | 8 | 8 | No change |
| `src/config.rs` inline | 42 | 44 | +2 (add `validate_config` tests to existing mod) |
| `src/metrics.rs` inline | 4+ | 5+ | +1 (add `test_metrics_success_rate` to existing mod) |
| `tests/` (existing) | ~60 | ~60 | No change |
| `tests/click_policy_tests.rs` | 0 | 2 | +2 policy tests moved |
| `tests/page_manager_policy_tests.rs` | 0 | 1 | +1 policy test moved |
| `tests/blockmedia_policy_tests.rs` | 0 | 1 | +1 policy test moved |
| `tests/task_registry_policy_tests.rs` | 0 | 1 | +1 moved |
| **Estimated Total** | **~115** | **~117** | **+2** (net: -3 deleted +2 config +1 metrics +4 integration) |

*Corrected assumption: `src/tests/mod.rs` contains 5 module references + inline config_tests/metrics_tests mods. All mod refs must be removed before deletion. Note: `src/metrics.rs` already has a `tests` mod with 4+ tests; only add missing `test_metrics_success_rate`.*

**Note**: Run `cargo test 2>&1 | grep "test result"` to get the actual count before and after.

---

## Validation Steps
1. **Compile**: `cargo check` (must pass)
2. **Test**: `cargo test` (all tests must pass)
3. **Verify affected tests**: Run `cargo test --lib` (unit tests) and `cargo test --test <policy_test_name>` for each moved integration test to ensure they are discovered and pass
4. **Compare counts**: Before restructuring, count tests in affected modules:
   ```bash
   grep -c '#[test]' src/result.rs src/config.rs src/metrics.rs src/tests/mod.rs
   ```
   After restructuring, ensure no net loss (excluding redundant result_tests.rs)
5. **Clean up**: Confirm `src/tests/` directory is removed

**Note**: There is a pre-existing test failure (1 failure out of 1650+ tests) unrelated to this restructuring. Investigate separately.

---

## Key Improvements In This Revision
1. **Fixed incorrect claim**: `config.rs` HAS 25+ tests (not "NO test module")
2. **Corrected redundancy analysis**: `result_tests.rs` IS redundant; `task_registry_policy_tests.rs` is NOT
3. **Added specific test functions**: Verified `validate_config()` exists at config.rs:1368
4. **Verified MetricsCollector methods**: `get_stats()` (line 290), `success_rate()` (line 319) exist
5. **Fixed struct field names**: Verified `circuit_breaker` field name is correct (resolved previous misnaming)
6. **Policy tests properly categorized**: Moved to `tests/` subdirectory (flat structure for discovery)
7. **Fixed duplicate module issue**: Tests added to existing `mod tests` in `config.rs` instead of creating duplicate
8. **Added Windows compatibility**: Noted PowerShell equivalents for Unix shell commands
9. **Corrected test count arithmetic**: Net change is +2 (not +1), accounting for deleted `result_tests.rs`
10. **Fixed dangling mod reference**: Added step to remove `mod result_tests;` from `mod.rs` before deletion
11. **Fixed policy test discovery**: Moved policy tests directly to `tests/` (not subdirectory) to ensure Rust's test harness discovers them
12. **Fixed mod.rs cleanup**: Added step to remove all 5 module references + inline test mods before deleting `mod.rs`
13. **Verified TASK_NAMES publicity**: Confirmed `pub const TASK_NAMES` in `src/task/mod.rs`, accessible via `auto::task::TASK_NAMES`
14. **Corrected mod.rs description**: Updated to reflect actual contents (5 mod refs + inline test mods)
15. **Fixed metrics test module**: Corrected claim that `src/metrics.rs` has NO test module - it actually has existing `tests` mod with 4+ tests
16. **Prevented duplicate tests**: Only add missing `test_metrics_success_rate` to existing metrics test mod, avoid duplicating `test_metrics_collector_new`
17. **Fixed imports for moved tests**: Specified changing `use crate::` to `use auto::` for policy and task_registry_policy_tests when moving to `tests/`
