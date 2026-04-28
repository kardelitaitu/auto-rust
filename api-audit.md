# API Audit Checklist

Scope: `src/runtime/task_context.rs`

Goal: review every public `TaskContext` API one by one for correctness, reliability, scalability, and ease of use.

## Constructors and Core State
- [x] `new()`
- [x] `new_with_metrics()`
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
- [ ] `http_get()`
- [ ] `download_file()`

## DOM Inspection and Element State
- [ ] `get_computed_style()`
- [ ] `get_element_rect()`
- [ ] `get_scroll_position()`
- [ ] `count_elements()`
- [ ] `is_in_viewport()`
- [ ] `exists()`
- [ ] `visible()`
- [ ] `text()`
- [ ] `html()`
- [ ] `attr()`
- [ ] `value()`
- [ ] `wait_for()`
- [ ] `wait_for_visible()`

## Focus, Hover, and Pointer Movement
- [ ] `focus()`
- [ ] `hover()`
- [ ] `move_mouse_to()`
- [ ] `move_mouse_fast()`
- [ ] `randomcursor()`
- [ ] `sync_cursor_overlay()`

## Clicking and Dragging
- [ ] `click_at()`
- [ ] `click()`
- [ ] `click_and_wait()`
- [ ] `double_click()`
- [ ] `middle_click()`
- [ ] `left_click()`
- [ ] `nativeclick()`
- [ ] `nativecursor()`
- [ ] `nativecursor_query()`
- [ ] `nativecursor_selector()`
- [ ] `left_click_fast()`
- [ ] `right_click_at()`
- [ ] `right_click_fast()`
- [ ] `right_click()`
- [ ] `drag()`

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

### Pause and Timing — audit notes

Implementation (`src/runtime/task_context.rs`):

| API | Behavior | Delegates to |
|-----|----------|--------------|
| `pause(base_ms)` | Fixed **20%** spread, **uniform** random delay in `[base*(1-0.2), base*(1+0.2)]` (clamped 10ms–30s). | `timing::uniform_pause(base_ms, 20)` |
| `pause_with_variance(base_ms, variance_pct)` | Custom `variance_pct` (0–100% as fraction of base for bounds), **Gaussian**-sampled delay (clamped same 10ms–30s). | `timing::human_pause(base_ms, variance_pct)` |

Findings:

- **Naming vs behavior**: `pause` is uniform ±20%; `pause_with_variance` is Gaussian, not uniform. Callers who expect “variance” to mean the same distribution as `pause` may be surprised. Consider documenting explicitly or aligning implementations (e.g. `pause_with_variance` → `uniform_pause` for parity, or rename / add `pause_gaussian`).
- **AGENTS.md alignment**: “`api.pause(base_ms)` uses uniform 20% deviation” matches code.
- **Shutdown**: both use `tokio::time::sleep` only; pauses are **not** tied to orchestrator shutdown/cancel (long `pause` can delay exit). Note for graceful-shutdown work if desired.
- **Edge cases**: `base_ms == 0` still yields clamped minimum ~10ms via `uniform_pause` / `human_pause` helpers in `src/utils/timing.rs`.

### Pause and Timing — improvement strategy

**Goal:** predictable semantics for task authors, minimal surprise, optional faster shutdown, docs that match behavior.

1. **Clarify contract (low risk, do first)**  
   - In `TaskContext` rustdoc: state explicitly **uniform vs Gaussian**, clamp range, and that `pause` is always 20% uniform.  
   - In `TASK_AUTHORING_GUIDE.md` / AGENTS: one short table mirroring the audit table so examples use the right API on purpose.

2. **Reduce naming/behavior mismatch (pick one path)**  
   - **Path A — document only:** keep signatures; add `pause_gaussian` alias only if you want discoverability without breaking callers.  
   - **Path B — align behavior:** change `pause_with_variance` to call `uniform_pause(base_ms, variance_pct)` so “variance” means the same family as `pause`; add **new** `pause_human` (or keep internal `human_pause` behind that name) for Gaussian. Requires grep for `pause_with_variance` usages and a changelog note.  
   - **Path C — deprecate:** mark `pause_with_variance` deprecated with migration message to the chosen split APIs; remove in a later major.

3. **Shutdown-aware waits (medium effort, high UX)**  
   - Thread a cancel token (or `watch` shutdown flag) into `TaskContext` where feasible; implement `pause` / variants with `tokio::select!` on `sleep` vs shutdown so long pauses truncate on graceful shutdown.  
   - Document: “pause may end early during shutdown” to avoid tasks relying on exact wall-clock duration for correctness.

4. **Caps and policy (optional)**  
   - Optional max single-pause cap (config) to prevent accidental `pause(600000)` from blocking sessions.  
   - Optional metrics: count total paused ms per run for tuning.

5. **Verification**  
   - Unit tests: distribution bounds (min/max clamp), `base_ms == 0`, and if Path B: that uniform vs Gaussian are covered by distinct tests.  
   - If shutdown-aware: integration test that shutdown completes within N ms while a long pause is pending.

**Recommended order:** (1) → choose **Path A or B** → (3) if graceful shutdown is a priority → (4)(5) as needed.

## Review Notes
- [ ] Confirm each API has a single responsibility.
- [ ] Confirm errors are clear and actionable.
- [ ] Confirm defaults are safe for automation tasks.
- [ ] Confirm public names are short and consistent.
- [ ] Confirm browser-task behavior stays thin and reusable.
