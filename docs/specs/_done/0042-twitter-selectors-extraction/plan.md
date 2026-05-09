# Plan

## Step 1: Create JS Asset Directory

- Create `src/utils/twitter/js/`.
- This directory will hold the extracted JavaScript files.

## Step 2: Extract Scripts

- For each function returning a raw JS string (e.g., `selector_feed_visible`, `selector_all_tweets`, `js_identify_engagement_candidates`), create a corresponding `.js` file in the new directory.
- Move the JavaScript content into the new files. Remove the outer `r#"` and `"#` Rust string delimiters.
- Ensure the JS syntax is valid standalone JavaScript (they are mostly IIFEs).

## Step 3: Update Rust Module

- Update the functions in `twitteractivity_selectors.rs` to load the scripts at compile time.
- Example:
  ```rust
  pub fn selector_feed_visible() -> &'static str {
      include_str!("js/selector_feed_visible.js")
  }
  ```
- For functions that format strings (like `selector_element_center`), retain the `format!` macro but use `include_str!` for the template base. Note: The `{{` and `}}` used for escaping in Rust's `format!` macro must be converted to `{` and `}` in the pure `.js` file, and you will need to handle interpolation carefully (perhaps by string replacement instead of `format!` to avoid bracket escaping hell in the JS files).

## Step 4: Verification

- Run `cargo test --lib utils::twitter::twitteractivity_selectors::tests` to ensure the JS strings still generate correctly.
- Ensure `cargo clippy` and `./check-fast.ps1` pass without errors.

# Internal API Outline

- `include_str!("js/<filename>.js")` will replace raw string literals.
- No public API signatures in `twitteractivity_selectors.rs` will change.

# Decisions

- Use `include_str!` instead of reading at runtime: This preserves the zero-overhead cost of the current implementation and ensures the scripts are bundled into the binary.
- For functions taking arguments (e.g., `selector_element_center`), we will replace the `format!` macro with `replace("{SELECTOR}", ...)` to keep the `.js` files clean of Rust macro escaping syntax (`{{`).
