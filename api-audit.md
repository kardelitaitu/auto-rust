# API Audit Checklist

Scope: `src/runtime/task_context.rs`

Goal: review every public `TaskContext` API one by one for correctness, reliability, scalability, and ease of use.

## Constructors and Core State
- [x] `new()` (last arg: `Option<CancellationToken>` for cooperative pause cancel)
- [x] `new_with_metrics()` (orchestrator passes `Some(cancel_token)`)
- [x] `session_id()`
- [x] `clipboard()`
- [x] `behavior_profile()`
- [x] `behavior_runtime()`
- [x] `native_interaction()`
- [x] `increment_run_counter()`
- [x] `metrics()`

## Navigation and Session Control
- [x] `navigate()`
- [x] `check_permission()`
- [x] `check_page_connected()`
- [x] `wait_for_load()`
- [x] `wait_for_any_visible_selector()`
- [x] `url()`
- [x] `title()`
- [x] `viewport()`

## Screenshots and Page Capture
- [x] `screenshot()`
- [x] `screenshot_with_quality()`

## Cookies, Storage, and Session Portability
- [x] `export_cookies()`
- [x] `export_cookies_for_domain()`
- [x] `export_session_cookies()`
- [x] `has_cookie()`
- [x] `import_cookies()`
- [x] `export_session()`
- [x] `import_session()`
- [x] `export_browser()`
- [x] `import_browser()`
- [x] `export_local_storage()`
- [x] `import_local_storage()`
- [x] `validate_session_data()`
- [x] `set_user_agent()`
- [x] `set_extra_http_headers()`
- [x] `apply_browser_context()`

## Clipboard and Data Files
- [x] `read_clipboard()`
- [x] `write_clipboard()`
- [x] `clear_clipboard()`
- [x] `has_clipboard_content()`
- [x] `append_clipboard()`
- [x] `read_data_file()`
- [x] `write_data_file()`
- [x] `list_data_files()`
- [x] `data_file_exists()`
- [x] `delete_data_file()`
- [x] `append_data_file()`
- [x] `data_file_metadata()`

## Network and Remote Data
- [x] `http_get()`
- [x] `download_file()`

## DOM Inspection and Element State
- [x] `get_computed_style()`
- [x] `get_element_rect()`
- [x] `get_scroll_position()`
- [x] `count_elements()`
- [x] `is_in_viewport()`
- [x] `exists()`
- [x] `visible()`
- [x] `text()`
- [x] `html()`
- [x] `attr()`
- [x] `value()`
- [x] `wait_for()`
- [x] `wait_for_visible()`

## Focus, Hover, and Pointer Movement
- [x] `focus()`
- [x] `hover()`
- [x] `move_mouse_to()`
- [x] `move_mouse_fast()`
- [x] `randomcursor()`
- [x] `sync_cursor_overlay()`

## Clicking and Dragging
- [x] `click_at()`
- [x] `click()`
- [x] `click_and_wait()`
- [x] `double_click()`
- [x] `middle_click()`
- [x] `left_click()`
- [x] `nativeclick()`
- [x] `nativecursor()`
- [x] `nativecursor_query()`
- [x] `nativecursor_selector()`
- [x] `left_click_fast()`
- [x] `right_click_at()`
- [x] `right_click_fast()`
- [x] `right_click()`
- [x] `drag()`

## Keyboard and Text Input
- [ ] `press()`
- [ ] `press_with_modifiers()`
- [ ] `keyboard()`
- [ ] `type_into()`
- [ ] `type_text()`
- [ ] `select_all()`
- [ ] `clear()`

## Scrolling and Reading
- [ ] `random_scroll()`
- [ ] `scroll_to()`
- [ ] `scroll_read()`
- [ ] `scrollread()`
- [ ] `scroll_read_to()`
- [ ] `scroll_back()`
- [ ] `scroll_into_view()`
- [ ] `scroll_to_top()`
- [ ] `scroll_to_bottom()`

## Pause and Timing
- [x] `pause()`
- [x] `pause_with_variance()`
- [x] `pause_human()`

### Pause and Timing — audit notes

Implementation (`src/runtime/task_context.rs`); optional `CancellationToken` from orchestrator (`new` / `new_with_metrics` last arg):

| API | Behavior | Delegates to |
|-----|----------|--------------|
| `pause(base_ms)` | Fixed **20%** spread, **uniform** random delay (clamped 10ms–30s). Ends early if cancel token set and cancelled. | `timing::uniform_pause_with_cancel(..., 20)` |
| `pause_with_variance(base_ms, variance_pct)` | Same **uniform** model as `pause`, custom spread `variance_pct`. Cancel-aware. | `timing::uniform_pause_with_cancel` |
| `pause_human(base_ms, variance_pct)` | **Gaussian** delay (human-like). Cancel-aware. | `timing::human_pause_with_cancel` |

Resolved (2026-04):

- **Uniform vs Gaussian**: `pause` / `pause_with_variance` are uniform; `pause_human` is Gaussian. Twitter humanized helpers use `pause_human` where Gaussian was intended.
- **Shutdown**: orchestrator passes `Some(cancel_token)` into `TaskContext::new_with_metrics`; pause family uses `timing::sleep_interruptible`.
- **Edge cases**: `base_ms == 0` still yields clamped minimum ~10ms via helpers in `src/utils/timing.rs`.

### Pause and Timing — improvement strategy (status)

- **Done:** (1) rustdoc + AGENTS + authoring guide table alignment; **Path B** (uniform `pause_with_variance`, new `pause_human`); (3) cancel-aware pauses + orchestrator wiring; (5) `sleep_interruptible` unit test in `timing.rs`.
- **Deferred:** (4) max-pause cap / per-run pause metrics; integration test “cancel during long pause” (optional follow-up).

## Review Notes
- [ ] Confirm each API has a single responsibility.
- [ ] Confirm errors are clear and actionable.
- [ ] Confirm defaults are safe for automation tasks.
- [ ] Confirm public names are short and consistent.
- [ ] Confirm browser-task behavior stays thin and reusable.
