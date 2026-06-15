# Plan — Modularize TaskContext mod.rs

## Baseline

`src/runtime/task_context/mod.rs` is 3,265 lines — the single largest file in the entire codebase. It already has 12 submodules extracted (click_learning, clipboard, cookies, data_files, http, interaction, interaction_pipeline, page_nav, query, session_io, style, types — totaling 3,257 lines). The remaining 3,265 lines in mod.rs contain:

| Concern | Lines (approx) | Description |
|---------|---------------|-------------|
| **Tests** | 113-1612 (1,500) | 80+ unit tests across 12 categories (click learning, retry behavior, browser management, permissions, data structures, policy integration, error handling, validation) |
| **Click pipeline** | ~400 | `click()` with adaptive timing/retry/fallback, `nativeclick()`, click variants (double, middle, right, left), private helpers (`execute_primary_click_attempt`, `fallback_click_with_adaptation`, `record_click_learning`) |
| **Pointer + Keyboard** | ~300 | Mouse movement (`hover`, `focus`, `move_mouse_to`, `randomcursor`, `drag`, `nativecursor`), typing (`r#type`, `type_text`, `press`), aliases (`keyboard`, `type_into`) |
| **Struct + Constructors** | 1614-1750 (136) | `TaskContext` struct, `new()`, `new_with_metrics()`, accessors |
| **Navigation + Query + Scroll + Pause** | ~930 | Thin delegation methods: `navigate`, `wait_for_load`, `exists`, `visible`, `text`, `html`, `attr`, `url`, `viewport`, `scroll_*`, `pause*`, `post_interaction_pause*`, `is_in_viewport`, `verify_selector_hit`, `set_user_agent`, `apply_browser_context`, etc. |

The test module alone accounts for 46% of the file. Extracting it yields the biggest single win.

## Implementation Steps

1. Create `src/runtime/task_context/tests.rs` — move the entire `#[cfg(test)] mod tests { ... }` block (lines 113-1612) verbatim. Update `use super::*` and any `super::` references.

2. Create `src/runtime/task_context/click.rs` — extract these methods from `impl TaskContext`:
   - `click()` + its two private helpers (`execute_primary_click_attempt`, `fallback_click_with_adaptation`)
   - `click_at()`, `click_and_wait()`
   - Click variants: `double_click()`, `middle_click()`, `left_click()`, `left_click_fast()`, `right_click()`, `right_click_at()`, `right_click_fast()`
   - `nativeclick()`
   - `record_click_learning()`
   
   Declare in `mod.rs`: `mod click;` and `use click::*;` (or explicit imports).

3. Create `src/runtime/task_context/pointer.rs` — extract these methods:
   - Mouse: `hover()`, `focus()`, `move_mouse_to()`, `move_mouse_fast()`, `randomcursor()`, `sync_cursor_overlay()`, `drag()`
   - Native cursor: `nativecursor()`, `nativecursor_query()`, `nativecursor_selector()`
   - Keyboard: `r#type()`, `keyboard()`, `type_into()`, `type_text()`, `press()`, `press_with_modifiers()`

4. Update `mod.rs`:
   - Add `pub mod tests;` (or `#[cfg(test)] mod tests;` if private)
   - Add `mod click;` and `mod pointer;`
   - Remove the extracted method bodies
   - Delegate from `impl TaskContext` to the submodule functions where needed, OR move the `impl` block into each submodule file with a separate `impl TaskContext` block

5. **Key design decision**: Rust allows multiple `impl TaskContext` blocks across files. Each submodule can have its own `impl TaskContext { ... }` block. The private methods (`verify_selector_hit`, `focus_internal`, `click_internal`, `nativeclick_internal`, `execute_nativecursor`, `is_in_viewport`, `post_interaction_pause*`, `url`, `viewport`) must be made `pub(crate)` so they're accessible from `click.rs` and `pointer.rs`.

6. No changes to any files outside `src/runtime/task_context/`.

## API Changes

- **Internal**: Private helper methods become `pub(crate)`: `verify_selector_hit`, `focus_internal`, `click_internal`, `nativeclick_internal`, `execute_nativecursor`, `is_in_viewport`, `post_interaction_pause`, `post_interaction_pause_with_budget`
- **Public**: No change. All existing `pub` methods retain their signatures and visibility.
- Module structure: `crate::runtime::task_context` gains `tests`, `click`, `pointer` submodules (alongside existing 12).

## Validation

- `cargo check --lib` — no compilation errors
- `cargo test --lib task_context` — all existing tests pass
- `powershell -File check.ps1` — full quality gate passes
- Manual: confirm `mod.rs` is ≤1,100 lines after extractions

## Design Decisions and Risks

**Why separate click from pointer/keyboard?** The click pipeline (~400 lines) is the most complex code in TaskContext — it has adaptive timing profiles, learning engine integration, multi-attempt retry loops, fallback strategies, and strict verification. Isolating it makes this complexity reviewable and testable independently. Pointer and keyboard methods are simpler thin wrappers over `capabilities::*` with post-interaction pauses.

**Why keep navigation/query/scroll/pause in mod.rs?** These are 40+ thin delegation methods (typically 2-5 lines each) that call `capabilities::*` directly. Extracting them to a separate file would create ~200 lines of boilerplate without meaningful organizational benefit. If mod.rs remains large after tests+click+pointer extraction, these can be further split in a follow-up spec.

**Risk: Multiple impl blocks.** Rust allows multiple `impl TaskContext` blocks, but they must be in the same crate. Since `click.rs` and `pointer.rs` are submodules of `task_context`, this is fine. The `pub(crate)` visibility change on private helpers is the main risk — any crate-internal callers of these methods (if any exist) must be verified.

**Confidence: High.** Twelve prior modularization specs (0014, 0016-0025) have followed this pattern. The click extraction is the riskiest due to complex state dependencies, but the plan delegates via pub(crate) methods rather than restructuring internals.
