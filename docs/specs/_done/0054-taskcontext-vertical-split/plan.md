# Plan

## Step 1: Create submodule files

For each domain, create a new file under `src/runtime/task_context/` containing:
- The module-level doc comment (extracted from the original file)
- All `use` imports needed by the moved methods
- The `impl TaskContext { ... }` block with the moved methods

### File: `src/runtime/task_context/navigation.rs`

Move from main file (lines ~1788–2020):
- `navigate()` — line 1788
- `check_permission()` — line 1841
- `check_page_connected()` — line 1889
- `screenshot()` — line 1930
- `screenshot_with_quality()` — line 1975

### File: `src/runtime/task_context/cookies.rs`

Move from main file (lines ~2035–2244, ~2835–2912):
- `export_cookies()` — line 2035
- `export_cookies_for_domain()` — line 2087
- `export_session_cookies()` — line 2148
- `has_cookie()` — line 2220
- `import_cookies()` — line 2835

### File: `src/runtime/task_context/clipboard.rs`

Move from main file (lines ~2247–2403):
- `read_clipboard()` — line 2247
- `write_clipboard()` — line 2261
- `clear_clipboard()` — line 2298
- `has_clipboard_content()` — line 2336
- `append_clipboard()` — line 2383

### File: `src/runtime/task_context/data_files.rs`

Move from main file (lines ~2404–2834):
- `read_data_file()` — line 2404
- `write_data_file()` — line 2419
- `list_data_files()` — line 2476
- `data_file_exists()` — line 2547
- `delete_data_file()` — line 2592
- `append_data_file()` — line 2639
- `read_json_data()` — line 2714
- `write_json_data()` — line 2762
- `data_file_metadata()` — line 2808

### File: `src/runtime/task_context/http.rs`

Move from main file (lines ~2913–3100):
- `http_get()` — line 2913
- `http_post_json()` — line 2991
- `download_file()` — line 3073

### File: `src/runtime/task_context/session_io.rs`

Move from main file (lines ~3136–3573, ~3652–3942, ~3943–4018, ~4019–4094):
- `export_session()` — line 3136
- `import_session()` — line 3574
- `export_browser()` — line 3652
- `import_browser()` — line 3820
- `export_local_storage()` — line 3943
- `import_local_storage()` — line 4019

### File: `src/runtime/task_context/style.rs`

Move from main file (lines ~3235–3529):
- `get_computed_style()` — line 3235
- `get_element_rect()` — line 3312
- `get_scroll_position()` — line 3391
- `count_elements()` — line 3459
- `is_in_viewport()` — line 3530

## Step 2: Register submodules in task_context.rs

Add `pub mod` declarations for each new file at the top of `task_context.rs` (alongside existing ones):

```rust
pub mod click_learning;
pub mod clipboard;      // new
pub mod cookies;        // new
pub mod data_files;     // new
pub mod http;           // new
pub mod interaction;
pub mod interaction_pipeline;
pub mod navigation;     // new
pub mod query;
pub mod session_io;     // new
pub mod style;          // new
pub mod types;
```

## Step 3: Remove moved methods from task_context.rs

After creating each submodule file, delete the corresponding method bodies from the original `impl TaskContext { ... }` block in `task_context.rs`.

## Step 4: Verify

1. `cargo check` — ensure compilation succeeds
2. `cargo nextest run --all-features --lib` — ensure all 2,099 tests pass
3. `cargo clippy --all-targets --all-features -- -D warnings` — no new warnings
4. `cargo fmt --all -- --check` — formatting consistent

## Step 5: Update implementation-notes.md

Document exact line counts before/after.
