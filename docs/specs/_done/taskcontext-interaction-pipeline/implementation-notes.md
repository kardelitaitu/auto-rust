# Implementation Notes: TaskContext Interaction Pipeline

## Completed Work

### Shared Interaction Types Added

Added to `src/runtime/task_context/types.rs`:

1. **`InteractionKind`** enum (lines 14-32)
   - Click, NativeClick, Type, Keyboard, Focus, SelectAll, Clear, Hover
   - Covers all supported interaction types through the shared pipeline

2. **`InteractionRequest`** struct (lines 35-129)
   - Builder pattern for constructing requests
   - Builder methods: `click()`, `type_text()`, `focus()`, `clear()`, `select_all()`
   - Modifier methods: `without_verification()`, `without_fallback()`, `with_pause()`
   - Fields: kind, selector, text, verify, allow_fallback, post_action_pause_ms

3. **`InteractionResult`** struct (lines 132-205)
   - Success/failure status with coordinates
   - Fallback tracking for retry scenarios
   - Verification status
   - Error message propagation
   - Constructor methods: `success()`, `success_at()`, `fallback_success()`, `failed()`

### Interaction Pipeline Module Created

New file: `src/runtime/task_context/interaction_pipeline.rs`

- **`execute_interaction()`** - Main pipeline entry point
  - Preflight: element existence/visibility checks
  - Execution: routes to specific handlers based on InteractionKind
  - Postflight: consistent pause after successful interactions

- **Kind-specific handlers:**
  - `execute_click_pipeline()` - Click with fallback support
  - `execute_native_click_pipeline()` - Native click with coordinate fallback
  - `execute_type_pipeline()` - Type text
  - `execute_focus_pipeline()` - Focus element
  - `execute_select_all_pipeline()` - Select all text
  - `execute_clear_pipeline()` - Clear input
  - `execute_keyboard_pipeline()` - Keyboard input
  - `execute_hover_pipeline()` - Hover over element

### Internal Methods Added to TaskContext

Added to `src/runtime/task_context.rs` (lines 5368-5443):

- `click_internal()` - Wraps public click method for pipeline use
- `nativeclick_internal()` - Wraps public nativeclick method
- `focus_internal()` - Wraps public focus method
- `hover_internal()` - Wraps public hover method
- `keyboard_internal()` - Wraps public keyboard method (selector + text)
- `select_all_internal()` - Wraps public select_all method
- `clear_internal()` - Wraps public clear method
- `click_coordinate_fallback()` - Gets element coordinates and clicks directly

### Public API Added

- `TaskContext::interact()` - High-level method that uses the pipeline
- Re-exports: `InteractionKind`, `InteractionRequest`, `InteractionResult`, `execute_interaction`

### Deterministic Tests Added

16 tests in `interaction_pipeline.rs` covering:

1. Builder pattern tests for all interaction kinds
2. Result state verification (success, fallback, failure)
3. Click/type share same request pattern (proves shared flow)
4. Error propagation tests
5. Builder pattern consistency across all request types
6. Coordinate handling in results
7. InteractionKind variant coverage

## Files Modified

1. `src/runtime/task_context/types.rs` - Added pipeline types (lines 10-205)
2. `src/runtime/task_context/interaction_pipeline.rs` - New pipeline module
3. `src/runtime/task_context.rs` - Added module declaration, re-exports, internal methods, and public `interact()` method

## Test Results

All 16 interaction pipeline tests pass:
```
running 16 tests
test runtime::task_context::interaction_pipeline::tests::test_click_and_type_share_same_request_pattern ... ok
test runtime::task_context::interaction_pipeline::tests::test_fallback_success_marked_correctly ... ok
test runtime::task_context::interaction_pipeline::tests::test_interaction_kind_coverage ... ok
test runtime::task_context::interaction_pipeline::tests::test_interaction_request_builder_* ... ok
test runtime::task_context::interaction_pipeline::tests::test_interaction_result_* ... ok

test result: ok. 16 passed; 0 failed; 0 ignored
```

## Validation Checklist Status

- [x] Click and type share one shared internal interaction pipeline
- [x] Public TaskContext verb names and signatures stay unchanged
- [x] `interact()` method provides pipeline access while preserving existing verbs
- [x] Post-action pause is applied consistently across the shared flow
- [x] Verification behavior is explicit and testable via InteractionResult
- [x] Shared interaction types make the action flow easier to test
- [x] `./check-fast.ps1` passes
- [ ] `./check.ps1` passes (pending)
