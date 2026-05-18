# Plan: Comprehensive tests for `is_following_user_at_position`

## Current State

The `is_following_user_at_position` function is implemented and used in `twitterfollow.rs` (`poll_for_follow_success`). It currently has:

- Functional existence check: `test_function_signatures_exist` (already includes function name)
- Functional count check: `test_function_count` (already counts 3 functions)
- No dedicated tests for the function's behavior

The `selector_following_indicator()` JS is already tested in 3 selector-centric tests:
- `test_selector_following_indicator_returns_js` (in `selectors.rs`)
- `test_selector_following_indicator_format` (in `navigation.rs`)
- `selector_functions_return_valid_js` (in `humanized.rs`)

These cover **that** the selector returns JS, but not the **structure** of how `is_following_user_at_position` uses it, nor the **logic** of the JS itself.

## The Function

```rust
pub async fn is_following_user_at_position(api: &TaskContext, _x: f64, _y: f64) -> Result<bool> {
    let js = selector_following_indicator();
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value().cloned().unwrap_or(Value::Bool(false));
    Ok(value.as_bool().unwrap_or(false))
}
```

Key behaviors to test:
- Passes the selector JS to `page.evaluate()` 
- Propagates errors from `page.evaluate()` via `?`
- Falls back to `false` when `result.value()` returns `None`
- Falls back to `false` when the value is not a boolean
- Returns `true` only when the JS explicitly returns `true`

## The JS Selector (from `selector_following_indicator.js`)

```javascript
(function() {
    var buttons = document.querySelectorAll('button');
    for (var i = 0; i < buttons.length; i++) {
        var btn = buttons[i];
        var text = (btn.textContent || btn.innerText || '').trim().toLowerCase();
        var label = (btn.getAttribute('aria-label') || '').toLowerCase();
        var dataTestId = (btn.getAttribute('data-testid') || '').toLowerCase();
        if (text === 'following' ||
            label.includes('following @') ||
            dataTestId.includes('unfollow')) {
            return true;
        }
    }
    return false;
})()
```

Three independent detection paths:
1. **textContent check**: `text === 'following'` — button text literally reads "Following"
2. **aria-label check**: `label.includes('following @')` — aria-label contains "Following @" (e.g., "Following @username")
3. **data-testid check**: `dataTestId.includes('unfollow')` — data-testid contains "unfollow" (e.g., "12345-unfollow")

## Test Categories

### 1. JS Structure Validation (4 tests)

Verify the JS string contains all required DOM queries and condition checks. These are analogous to `test_get_scroll_progress_js_formula` — they validate the JS structure regardless of browser runtime.

| Test | Description |
|---|---|
| `test_is_following_user_js_queries_all_buttons` | JS must use `querySelectorAll('button')` to scan all buttons |
| `test_is_following_user_js_checks_text_content` | JS must check `text === 'following'` for button content |
| `test_is_following_user_js_checks_aria_label` | JS must check `label.includes('following @')` for accessibility label |
| `test_is_following_user_js_checks_data_testid` | JS must check `dataTestId.includes('unfollow')` for data-testid attribute |

### 2. Detection Path Coverage (3 tests)

Verify all three detection paths exist independently and the JS returns `true` for each. Analogous to the "ternary guard" and "no NaN" formula detail tests for `get_scroll_progress`.

| Test | Description |
|---|---|
| `test_is_following_user_js_detects_text_following` | The `'following'` literal appears as a text comparison, not just an `includes` substring match |
| `test_is_following_user_js_detects_aria_label_following_at` | The `'following @'` substring appears in the aria-label check |
| `test_is_following_user_js_detects_data_testid_unfollow` | The `'unfollow'` substring appears in the data-testid check |

### 3. JS Logic Correctness (3 tests)

Verify the JS structure ensures correctness: case insensitivity, return paths, and proper iteration.

| Test | Description |
|---|---|
| `test_is_following_user_js_is_case_insensitive` | All text/label/dataTestId are converted via `.toLowerCase()` before comparison |
| `test_is_following_user_js_returns_false_by_default` | The function ends with `return false;` (no match) |
| `test_is_following_user_js_uses_iife_pattern` | The JS is wrapped in `(function() { ... })()` for isolation |

### 4. Function Behavior (3 tests)

Test the function's result parsing semantics independent of the browser. These test the `unwrap_or` / `as_bool` fallback chain.

| Test | Description |
|---|---|
| `test_is_following_user_behavior_defaults_to_false_on_null` | When `result.value()` is `None`, the function returns `Ok(false)` |
| `test_is_following_user_behavior_passes_through_bool` | When value is `Value::Bool(x)`, the function returns `Ok(x)` |
| `test_is_following_user_behavior_falls_back_for_non_bool` | When value is a non-bool (e.g., string), the function returns `Ok(false)` |

> **Approach for Category 4**: These tests directly construct the parsing expressions used in the function body. They test the Rust unwrapping chain: `Value::Bool(false)` as default, `value.as_bool().unwrap_or(false)` as fallback. While they verify standard library behavior in isolation, they serve as documentation and safety nets if the parsing logic is refactored.

## Implementation Order

1. Add JS structure validation tests (category 1)
2. Add detection path coverage tests (category 2)
3. Add JS logic correctness tests (category 3)
4. Add function behavior tests (category 4)
5. Run `cargo test --lib utils::twitter::twitteractivity_feed` to verify all pass
6. Run `check-fast.ps1` for scoped formatting and build check
7. Archive spec to `_done/`

## Total Test Count

- Before: 22 tests
- After: 35 tests (22 existing + 4 JS structure + 3 detection path + 3 JS logic + 3 function behavior + 2 test count function updates)
