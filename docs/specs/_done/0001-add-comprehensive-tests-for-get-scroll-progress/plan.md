# Plan: Comprehensive tests for `get_scroll_progress`

## Current State

The `get_scroll_progress` function was recently implemented. It currently has 1 test:
- `test_get_scroll_progress_js_formula` — validates the JS formula string contains expected API names

The `twitteractivity_feed` test module has 9 tests total. The function has no math-correctness tests, no edge-case tests, and no error-handling tests.

## Test Categories

### 1. JS Formula Validation (retain + enhance existing)

| Test | Description |
|---|---|
| `test_get_scroll_progress_js_formula` | **Retain as-is**: checks formula contains `window.scrollY`, `window.innerHeight`, `document.body.scrollHeight`, `? 1.0 :` |
| `test_get_scroll_progress_js_handles_division_by_zero` | New: formula must include guard for `scrollHeight - innerHeight === 0` (e.g., the `>=` comparison before division) |

### 2. Math Correctness (pure fn reimplementations)

These tests reimplement the JS formula logic in Rust to verify correctness across scroll positions without needing a browser.

| Test | Description |
|---|---|
| `test_get_scroll_progress_math_at_top` | At `scrollY=0, innerHeight=800, scrollHeight=2400`: progress = 0.0 |
| `test_get_scroll_progress_math_at_midpoint` | At `scrollY=800, innerHeight=800, scrollHeight=2400`: progress = 0.5 |
| `test_get_scroll_progress_math_at_bottom` | At `scrollY=1600, innerHeight=800, scrollHeight=2400`: progress = 1.0 |
| `test_get_scroll_progress_math_beyond_bottom` | At `scrollY=2000, innerHeight=800, scrollHeight=2400`: progress = 1.0 (ternary guard) |

### 3. Edge Cases

| Test | Description |
|---|---|
| `test_get_scroll_progress_math_zero_scrollable_area` | `scrollHeight === innerHeight` (800/800): division by zero avoided by `>=` guard, returns 1.0 |
| `test_get_scroll_progress_math_content_fits_viewport` | `scrollHeight < innerHeight` (500/800): 1.0 (no scrollable area) |
| `test_get_scroll_progress_math_negative_scroll_y` | `scrollY=-50, innerHeight=800, scrollHeight=2400`: progress ≈ 0.0 (ternary false branch) |
| `test_get_scroll_progress_clamp_lower_bound` | Verify `f64::clamp(0.0, 1.0)` clamps values below 0.0 to 0.0 |
| `test_get_scroll_progress_clamp_upper_bound` | Verify `f64::clamp(0.0, 1.0)` clamps values above 1.0 to 1.0 |

> **Note on clamp tests**: The JS ternary already guarantees [0.0, 1.0] in normal operation. These tests verify the defensive `clamp()` wrapper works correctly for edge-case JS behavior (floating-point drift, negative values from overscroll APIs like Safari's elastic scroll). They test `f64::clamp` with the exact bounds used in the function.

### 4. Error Handling (error message construction)

Since `get_scroll_progress` requires a real browser `TaskContext` to exercise the error path, these tests verify the error message construction logic that would be triggered when `page.evaluate()` returns null or a non-numeric value.

| Test | Description |
|---|---|
| `test_get_scroll_progress_error_message_format` | Verify the error message constant contains "Failed to parse scroll progress" — a single test since both null and non-numeric paths produce the identical error |

> **Approach**: The error comes from `.ok_or_else(|| anyhow::anyhow!("Failed to parse scroll progress from page.evaluate"))` when `value().and_then(|v| v.as_f64())` returns `None`. These tests verify the error message constant is correct by constructing an identical error and checking its display string. While this is a lightweight check, it protects against accidental error message drift during refactoring.

### 5. Implementation Details (formula correctness)

| Test | Description |
|---|---|
| `test_get_scroll_progress_formula_uses_ternary_guard` | Verify the formula checks `scrollY + innerHeight >= scrollHeight` before division |
| `test_get_scroll_progress_formula_no_nan_patterns` | Verify the formula string does not contain NaN-producing patterns (e.g., bare division without a guard) — checkable via string matching |

## Implementation Order

1. Add math correctness tests (category 2) — reimplement JS formula in Rust side-by-side
2. Add edge case + clamp tests (category 3)
3. Add error message tests (category 4)
4. Add implementation detail tests (category 5)
5. Run `cargo test --lib utils::twitter::twitteractivity_feed` to verify all pass
6. Run `check-fast.ps1` for scoped formatting and build check

## Total Test Count

- Before: 9 tests
- After: 20 tests (9 existing + 4 math + 5 edge/clamp + 1 error + 2 formula detail)
