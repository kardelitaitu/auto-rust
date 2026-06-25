# testing-rust

Expert skill for **writing tests** in the auto-rust Rust codebase. Covers all 3 test patterns — unit tests, proptests, and integration tests — with concrete patterns drawn from the actual codebase.

## When to use

- User says "add tests for this function"
- User wants to add proptest coverage for edge cases
- User needs to know which test type to use and how
- User is debugging a failing test and needs to understand the test infrastructure
- User needs to add `#[ignore]` for browser-backed tests

---

## 1. The 3 test patterns

This codebase uses 3 distinct test patterns, each with a different purpose:

| Pattern | Location | Runs without browser? | Best for |
|---|---|---|---|
| **Inline unit tests** | `#[cfg(test)] mod tests { ... }` inside each source file | ✅ Yes | Testing individual functions, parsing, data manipulation, helpers |
| **Proptests** | `proptest! { ... }` inside `mod proptests` or inline in `mod tests` | ✅ Yes | Property-based testing of edge cases, round-trips, invariants |
| **Integration tests** | `tests/*.rs` (separate crate) | ❌ No (browser needed) | Cross-module flows, DSL execution, session lifecycle, shutdown handling |
| **Lib tests** | `src/tests/*.rs` (internal to the crate) | ✅ Yes | Policy validation, task routing, internal invariants |

---

## 2. Inline unit tests (`#[cfg(test)]`)

### Where they live

Every source file in `src/` that has testable logic has a `#[cfg(test)]` block at the bottom:

```rust
// src/utils/keyboard.rs

// ... production code ...

#[cfg(test)]
mod tests {
    use super::*;     // Import everything from the parent module
    use serde_json::json;  // For building test payloads (when needed)

    #[test]
    fn test_descriptive_name() {
        // Arrange
        let input = "test";

        // Act
        let result = normalize_modifier(input);

        // Assert
        assert_eq!(result, "Test");
    }
}
```

### Naming conventions

- All test functions start with `test_` (not `should_`, not `it_`)
- Use snake_case, descriptive: `test_extract_url_from_payload_empty`
- Group related tests conceptually (they'll run alphabetically)

### Common assertion patterns from the codebase

```rust
// Simple equality
assert_eq!(result, expected);

// Range check
assert!((48_000..=72_000).contains(&duration_ms));

// Boolean
assert!(result.contains("x.com"));
assert!(!result.is_empty());

// Result checking
assert!(result.is_ok());
assert!(result.is_err());

// Error message check
assert!(result.unwrap_err().to_string().contains("not found"));

// Contains
assert!(debug_str.contains("PressOptions"));

// Approximate float equality
let diff = (left - right).abs();
assert!(diff < 0.001, "Expected {} ≈ {}, diff = {}", left, right, diff);
```

### Test helper macros (from `tests/common/mod.rs`)

```rust
// Assert Result is Ok and unwrap
assert_ok!(function_returning_result());

// Assert Result is Err
assert_err!(function_that_should_fail());

// Assert string contains substring
assert_contains!(haystack, "needle");

// Assert approximate float equality
assert_approx_eq!(left, right, tolerance);
```

### What to test

For every non-trivial function, test:
1. **Happy path** — normal input produces expected output
2. **Edge cases** — empty input, min/max values, boundary conditions
3. **Error cases** — invalid input returns expected error
4. **Round-trips** — serialize → deserialize → compare (for serde types)

**Specific codebase patterns:**

```rust
// Test duration bounds (every task module has one)
#[test]
fn task_duration_stays_within_bounds() {
    let duration_ms = task_duration_ms();
    assert!((24_000..=36_000).contains(&duration_ms));
}

// Test payload extraction (every Twitter task)
#[test]
fn extract_url_from_payload_url() {
    let payload = json!({"url": "https://x.com/user/status/123"});
    let result = extract_url_from_payload(&payload).unwrap();
    assert!(result.as_str().contains("x.com"));
}

// Test empty payload fallback
#[test]
fn extract_url_from_payload_empty() {
    let payload = json!({});
    let result = extract_url_from_payload(&payload).unwrap();
    assert_eq!(result.as_str(), "");
}
```

### Struct and Option tests

```rust
#[test]
fn test_press_options_defaults() {
    let options = PressOptions::default();
    assert!(options.modifiers.is_empty());
    assert_eq!(options.delay, 0);
}

#[test]
fn test_press_options_custom() {
    let options = PressOptions {
        modifiers: vec!["Control".to_string()],
        delay: 100,
        ..Default::default()
    };
    assert_eq!(options.modifiers.len(), 1);
}

#[test]
fn test_press_options_clone() {
    let options = PressOptions::default();
    let cloned = options.clone();
    assert_eq!(cloned.delay, options.delay);
}
```

---

## 3. Property-based tests (`proptest!`)

### When to use proptests

Use proptests when:
- A function has an **invariant** that should hold for all inputs
- A function is **self-inverse** (applying twice returns original)
- A function should **not panic** for any input
- You want to test **round-trip serialization**

### Proptest location

Proptests live inside `mod tests` in the source file, typically in a nested `mod proptests`:

```rust
#[cfg(test)]
mod tests {
    // ... regular unit tests ...

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn proptest_descriptive_name(
                // strategies for inputs
                param in proptest::char::range('a', 'z'),
            ) {
                // invariant to test
                prop_assert!(some_invariant(param));
            }
        }
    }
}
```

### Pattern 1: Self-inverse (from `src/utils/keyboard.rs`)

```rust
/// For mapped characters, get_similar_char is self-inverse (applying it twice
/// returns the original character).
fn is_self_inverse(ch: char) -> bool {
    matches!(
        ch.to_ascii_lowercase(),
        'a' | 's' | 'd' | 'f' | 'e' | 'r' | 'w' | 'q'
    ) && !ch.eq_ignore_ascii_case(&'i')
}

proptest! {
    #[test]
    fn proptest_get_similar_char_self_inverse(
        ch in proptest::char::range('a', 'z'),
    ) {
        prop_assume!(is_self_inverse(ch));
        let roundtrip = get_similar_char(get_similar_char(ch));
        prop_assert_eq!(roundtrip, ch);
    }
}
```

### Pattern 2: Identity for unmapped inputs (from `src/utils/keyboard.rs`)

```rust
proptest! {
    #[test]
    fn proptest_get_similar_char_unmapped_identity(
        ch in proptest::char::range(' ', '~'),
    ) {
        let mapped = get_similar_char(ch);
        prop_assume!(mapped == ch);
        let second = get_similar_char(mapped);
        prop_assert_eq!(second, ch);
    }
}
```

### Pattern 3: No-panic sanity check

```rust
proptest! {
    #[test]
    fn proptest_get_similar_char_no_panic(
        ch in any::<char>(),
    ) {
        let _result = get_similar_char(ch);
        prop_assert!(true);
    }
}
```

### Pattern 4: YAML round-trip (from `src/task/dsl/parser.rs`)

```rust
proptest! {
    #[test]
    fn test_yaml_round_trip(task_def in arb_task_definition()) {
        let yaml = serde_yaml::to_string(&task_def).unwrap();
        let parsed: TaskDefinition = serde_yaml::from_str(&yaml).unwrap();
        prop_assert_eq!(task_def.name, parsed.name);
        prop_assert_eq!(task_def.description, parsed.description);
    }
}
```

### Pattern 5: Fuzz validation (from `src/task/dsl/parser.rs`)

```rust
/// Fuzz: validate_task_definition must never panic.
#[test]
fn fuzz_validate_task_definition(
    task_def in arb_task_definition(),
) {
    let _result = validate_task_definition(&task_def);
}
```

### Key proptest strategies used in this codebase

| Strategy | Example | Used in |
|---|---|---|
| `proptest::char::range('a', 'z')` | Lowercase ASCII | `keyboard.rs` |
| `proptest::char::range(' ', '~')` | Printable ASCII | `keyboard.rs` |
| `any::<char>()` | Any valid char | `keyboard.rs` |
| `any::<u64>()` | Any 64-bit unsigned | `twitteractivity_limits.rs` |
| Custom strategies via `prop_compose!` | Complex nested types | `parser.rs` |
| `vec(any::<String>(), 0..10)` | String vectors | `parser.rs` |

### Rules for writing good proptests

1. **Always use `prop_assume!` to filter inputs** — don't test invariants that don't apply
2. **Always use `prop_assert!` / `prop_assert_eq!`** — not regular `assert!` (proptest needs the `prop_` variants to report the failing input)
3. **Name tests with `proptest_` prefix** — to distinguish from regular unit tests
4. **Keep strategies tight** — don't generate huge inputs (use `0..100` not `any::<usize>`)
5. **Add a doc comment** explaining the invariant being tested

---

## 4. Integration tests (`tests/`)

### When to use integration tests

Integration tests go in `tests/*.rs` when:
- The test needs a **real browser** via CDP
- The test spans **multiple modules** (orchestrator + session + task)
- The test involves **DSL task execution** end-to-end
- The test validates **shutdown, cancellation, or lifecycle** behavior

### Structure

Each integration test file is a standalone crate. They import from the `auto` crate:

```rust
// tests/session_management_tests.rs
use auto::session::Session;
// No `mod` declarations needed — tests/ is a separate crate
```

### Browser-backed tests (`#[ignore]` by default)

Tests that need a real browser are marked `#[ignore]` and require `TASK_API_TEST_WS`:

```rust
/// This test requires a browser with remote debugging enabled.
/// Set TASK_API_TEST_WS=ws://localhost:9222 to run.
#[ignore]
#[tokio::test]
async fn test_browser_interaction() {
    let cdp_url = std::env::var("TASK_API_TEST_WS")
        .expect("TASK_API_TEST_WS not set");
    // ... connect and test ...
}
```

To run them:
```powershell
# Start a browser with CDP, then:
$env:TASK_API_TEST_WS="ws://localhost:9222"
cargo test --test <name> -- --ignored

# Or use the integration test script:
.\scripts\run-integration-tests.ps1
.\scripts\run-integration-tests.ps1 -TestFilter query
.\scripts\run-integration-tests.ps1 -IncludeOrchestrator
```

### Mock-based integration tests (no browser needed)

Some tests use `MockPageContext` from `tests/common/mod.rs` to simulate browser behavior:

```rust
use auto::tests::test_utils::*;

#[tokio::test]
async fn test_with_mock() {
    let mock = MockPageContext::new()
        .with_local_storage("example.com", build_local_storage_data());
    assert!(mock.export_local_storage_json("example.com").is_ok());
}
```

`MockPageContext` provides:
- `with_local_storage()` / `with_session_storage()` — simulate storage
- `with_cookie()` — add cookies
- `record_js()` / `get_last_js()` — track JavaScript execution
- `count_elements()` / `is_in_viewport()` — mock DOM queries
- `export_local_storage_json()` — simulate storage export

### Test helper types (from `tests/common/mod.rs`)

```rust
// Temporary directory that auto-cleans on drop
let temp = TempTestDir::new();
let path = temp.create_file("test.txt", "content");
let json_path = temp.create_json_file("data.json", serde_json::json!({"k": "v"}));

// Mock HTTP responses
let resp = MockHttpResponse::json(200, serde_json::json!({"ok": true}));
let not_found = MockHttpResponse::not_found();
let error = MockHttpResponse::server_error();

// Build test data
let cookie = build_mock_cookie("session", "abc", "example.com");
let storage = build_local_storage_data();  // 3 key/value pairs
```

---

## 5. How to run tests

### Quick reference

```powershell
# Fastest: check only (no test execution)
cargo check

# Run all lib tests (4,128+ tests, no browser needed)
cargo test --lib

# Run a specific module's tests (fast feedback)
cargo test --lib <module_name>
# Examples:
cargo test --lib keyboard
cargo test --lib policy
cargo test --lib twitterlike

# Run a specific test function
cargo test --lib test_extract_url_from_payload_url

# Run with output visible
cargo test --lib -- --nocapture

# Run proptests (they're just regular tests)
cargo test --lib proptest_get_similar_char

# Run integration tests that don't need a browser
cargo test --test dsl_integration
cargo test --test dsl_translation

# Run browser-backed integration tests (requires CDP browser)
$env:TASK_API_TEST_WS="ws://localhost:9222"
cargo test --test session_management -- --ignored

# Full CI pipeline
.\check-fast.ps1     # Fast scoped check
.\check.ps1          # Full CI pipeline (check + fmt + clippy + nextest)

# Run with nextest (used in CI)
cargo nextest run --all-features --lib

# Profile a specific test
Measure-Command { cargo test --lib twitterlike -- 2>&1 > $null }
```

### Test output levels

```powershell
# Default: show failures only
cargo test --lib

# Show all output (use for debugging flaky tests)
cargo test --lib -- --nocapture

# Show test names as they run
cargo test --lib -- --show-output

# Run only ignored tests
cargo test --lib -- --ignored

# Run tests including ignored
cargo test --lib -- --include-ignored
```

### Filtering for proptests

Proptests have built-in shrinkers — when a proptest fails, it will report the minimal failing input. To reproduce:

```powershell
# Seed-based reproduction (proptest prints the seed on failure)
cargo test --lib proptest_test_name -- --proptest-seed=123456789

# Run more proptest cases to catch flaky failures
PROPTEST_CASES=10000 cargo test --lib proptest_test_name
```

---

## 6. Async tests

### Sync tests (`#[test]`)

Most tests are sync — use `#[test]` for:
- Pure function tests
- Helper function tests
- Duration/variance tests
- Selector/locator tests

### Async tests (`#[tokio::test]`)

Use `#[tokio::test]` when the function under test is `async`:

```rust
#[tokio::test]
async fn test_async_operation() {
    // The tokio runtime is provided automatically
    let result = some_async_function().await;
    assert!(result.is_ok());
}
```

Async tests are less common in the test suite (most browser-dependent logic is tested via mocks or integration tests). Use them sparingly — prefer to extract sync helper functions that are testable with `#[test]`.

### Pattern: run_with_timeout for async tests

```rust
// From tests/common/mod.rs
pub async fn run_with_timeout<F, T>(future: F, timeout_ms: u64) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), future)
        .await
        .expect("Test timed out")
}

// Usage
#[tokio::test]
async fn test_with_timeout() {
    run_with_timeout(async {
        // test body
    }, 5000).await;
}
```

---

## 7. When to add `#[ignore]`

Add `#[ignore]` when the test **cannot run without external infrastructure**:

| Scenario | Example | Alternative |
|---|---|---|
| Needs a real browser | `#[ignore] #[tokio::test]` | Extract sync helpers, test those without browser |
| Needs network access | `#[ignore]` | Mock the HTTP layer |
| Needs Twitter API | `#[ignore]` | Test the logic layer with mock data |
| Slow (>10s) | `#[ignore]` or mark with `#[cfg(not(debug_assertions))]` | Consider if test is still valuable |

**Never add `#[ignore]` to pure unit tests** — unit tests should always run because they have zero external dependencies.

---

## 8. Test-driven checklist for new code

When writing tests for new code:

- [ ] **Happy path** — one `#[test]` for normal successful operation
- [ ] **Edge cases** — empty input, max values, boundary conditions
- [ ] **Error cases** — invalid input produces a clear error
- [ ] **Invariants** — consider a proptest for round-trip or self-inverse properties
- [ ] **Duration bounds** — if adding a `DEFAULT_*_DURATION_MS` constant, add a `task_duration_stays_within_bounds()` test
- [ ] **Payload extraction** — if adding parameters, add `test_extract_*_from_payload_*()` tests that cover each parameter + empty payload
- [ ] **No browser dependency** — keep tests in `#[cfg(test)]` inline modules, not `tests/` integration tests, unless they genuinely need a browser
- [ ] **Run cargo test --lib <module>** — confirm all tests pass before pushing

---

## 9. Common test failures and debugging

### Test passes locally but fails in CI
- **Check for timing dependencies** — use `run_with_timeout()` for async operations
- **Check for environment variables** — set `TASK_API_TEST_WS` in CI
- **Check for proptest seed** — CI may run with different seed

### Proptest failure
```
Test failed: assertion failed: ...; minimal failing input: ...
```
- Read the "minimal failing input" — proptest shrinks to the smallest failing case
- If it's a real bug, fix the code. If it's a false positive, tighten the strategy with `prop_assume!`

### "Test ignored" when you expected it to run
- Remove `#[ignore]` or run with `-- --ignored` or `-- --include-ignored`

### Async test hangs
- Add a timeout: `tokio::time::timeout(Duration::from_secs(5), test_future).await`
- Check if the test is waiting on a channel or event that never fires

---

## Common pitfalls

1. **Using `assert!` instead of `prop_assert!` in proptests** — proptest won't report the failing input. Always use `prop_assert!`, `prop_assert_eq!`, etc.
2. **Forgetting `prop_assume!`** — without filtering, proptests will test invariants that don't hold for all inputs
3. **Adding `#[ignore]` to unit tests** — unit tests should always run. Only browser-backed integration tests should be ignored.
4. **Not using `super::*`** — the test module doesn't have access to parent module items without it
5. **Testing implementation, not behavior** — test the public API, not private internals (unless the private function is the one being tested)
6. **Missing serde_json import** — `use serde_json::json;` is needed for `json!({...})` macro
7. **No duration test for new task modules** — every task with a `DEFAULT_*_DURATION_MS` constant must have `task_duration_stays_within_bounds()`
8. **Writing integration tests for things that could be unit tests** — prefer inline `#[cfg(test)]` tests. Only use `tests/` when you need a real browser or cross-module orchestration.

> last audited 26-06-26 by docs-auditor
