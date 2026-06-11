## Acceptance Criteria

- [ ] `tests.rs` exists with all ~80 unit tests from the original `#[cfg(test)] mod tests` block (≤1,550 lines)
- [ ] `click.rs` exists with `impl TaskContext` containing click pipeline methods: `click()`, `execute_primary_click_attempt()`, `fallback_click_with_adaptation()`, `click_at()`, `click_and_wait()`, `double_click()`, `middle_click()`, `left_click()`, `left_click_fast()`, `right_click()`, `right_click_at()`, `right_click_fast()`, `nativeclick()`, `record_click_learning()` (≤450 lines)
- [ ] `pointer.rs` exists with `impl TaskContext` containing pointer+keyboard methods: `hover()`, `focus()`, `move_mouse_to()`, `move_mouse_fast()`, `randomcursor()`, `sync_cursor_overlay()`, `drag()`, `nativecursor()`, `nativecursor_query()`, `nativecursor_selector()`, `r#type()`, `keyboard()`, `type_into()`, `type_text()`, `press()`, `press_with_modifiers()` (≤350 lines)
- [ ] `mod.rs` ≤1,100 lines after extractions (struct, constructors, accessors, navigation, query, scroll, pause, thin delegations)
- [ ] Private helpers made `pub(crate)` as needed: `verify_selector_hit`, `focus_internal`, `click_internal`, `nativeclick_internal`, `execute_nativecursor`, `is_in_viewport`, `post_interaction_pause`, `post_interaction_pause_with_budget`
- [ ] No behavioral changes — all existing tests pass with identical results
- [ ] No clippy warnings from `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo check --lib` compiles without errors
- [ ] `spec-lint.ps1` passes

## Test Commands

- `cargo check --lib`
- `cargo test --lib task_context`
- `powershell -File check.ps1`
- `powershell -File spec-lint.ps1`

## Visual Inspection

- Confirm `src/runtime/task_context/` now has 15 files (12 existing + tests.rs + click.rs + pointer.rs)
- Confirm `mod.rs` is ≤1,100 lines
- Confirm each extracted file has the expected methods in a dedicated `impl TaskContext` block
- Confirm no methods were duplicated or lost during extraction
