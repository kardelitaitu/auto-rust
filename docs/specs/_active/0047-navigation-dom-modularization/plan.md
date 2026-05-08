# Plan

## Step 1: Create DOM Module

- Create `src/utils/dom.rs`.
- Move the DOM query functions from `src/utils/navigation.rs` into `src/utils/dom.rs`. This includes:
  - `focus`, `focus_at_point`
  - `selector_exists`, `selector_is_visible`
  - `selector_text`, `selector_html`, `selector_attr`, `selector_value`
  - `selector_action_point`
  - `wait_for_selector`, `wait_for_visible_selector`
  - `wait_for_any_visible_selector`
  - The underlying helper functions like `css_selector_exists`, `query_ax_nodes`, `ax_locator_action_point`, etc.

## Step 2: Clean Navigation Module

- Keep `goto`, `goto_with_trampoline`, `goto_light`, `goto_raw` in `navigation.rs`.
- Keep `go_back`, `set_user_agent`, `set_extra_http_headers` in `navigation.rs`.
- Keep `page_url`, `page_title`, `wait_for_load`, `wait_for_page_settle` in `navigation.rs`.

## Step 3: Update Module Exports

- Edit `src/utils/mod.rs` to declare `pub mod dom;`.
- Check if `src/prelude.rs` or `src/internal/mod.rs` re-exports `navigation` and update them to also re-export `dom` if necessary so downstream consumers (like `TaskContext`) don't break.
- If `TaskContext` relies on these functions directly in `src/runtime/task_context.rs`, update its `use` imports.

## Step 4: Verification

- Run `cargo check` to fix all compilation errors stemming from the moved functions.
- Run `cargo test --lib utils::navigation` and the new `utils::dom` tests to ensure the inline tests were moved correctly and still pass.

# Internal API Outline

- `src/utils/dom.rs` will house `pub async fn selector_exists(...)`, `pub async fn wait_for_selector(...)`, etc.
- `src/utils/navigation.rs` will retain `pub async fn goto(...)`, `pub async fn wait_for_load(...)`, etc.

# Decisions

- Split over Sub-modules: Unlike `self_healing`, `navigation.rs` and the new `dom.rs` are broad enough utility categories that they deserve to be top-level siblings in the `utils` directory, rather than nesting `dom` under `navigation` (which would imply DOM querying is a subset of navigation).
