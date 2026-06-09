## Acceptance Criteria
1. executor.rs non-test code reduced from 864 lines to ≤500 lines
2. executor.rs total file reduced from 4736 lines to ≤1000 lines
3. Action handlers extracted into 4 focused submodules under `actions/`:
   - `browser.rs`: navigate, click, type, hover, select, scroll_to, right_click, double_click, clear
   - `wait.rs`: wait, wait_for
   - `inspection.rs`: extract
   - `media.rs`: screenshot
4. Each extracted submodule compiles independently via `cargo check` (no cross-module dead_code warnings)
5. `cargo test` passes all 3466+ tests with no regressions
6. `cargo clippy --all-targets --all-features` shows zero new warnings
7. Each new submodule has a `//!` module-level doc comment
8. `dsl/mod.rs` re-exports all public types previously accessible from `dsl::executor`
9. Test functions for extracted handlers moved inline to each action submodule's `#[cfg(test)] mod tests`

## Test Commands
- `cargo check`
- `cargo test -p auto-rust`
- `cargo clippy --all-targets --all-features`
- `cargo fmt --check`
- `rg "fn " src/task/dsl/executor.rs | wc -l` (should be ≤80 after extraction, down from 198)

## Visual Inspection
- `src/task/dsl/actions/` directory exists with `mod.rs`, `browser.rs`, `wait.rs`, `inspection.rs`, `media.rs`
- `src/task/dsl/executor.rs` contains: struct def, constructors, `execute()`, `execute_action()` dispatch, `execute_call()`, cache methods, debug/profiling methods, and dispatch-level tests only
- Each action module has `//! <category> actions for DSL executor` doc comment
- No duplicate imports between executor.rs and new modules
- Test functions for each handler live in the same file as the handler
