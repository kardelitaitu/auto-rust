# Plan

## Step 1: Extract Validation Logic

- Create `src/utils/twitter/twitteractivity_llm_validation.rs`.
- Move `validate_reply`, `truncate_to_word_boundary`, `remove_mentions`, `remove_hashtags`, `remove_emojis`, `check_banned_words`, and the `BANNED_WORDS` constant into this file.
- Move the associated `#[cfg(test)]` block containing validation tests.
- Expose `validate_reply` publicly.

## Step 2: Extract Execution Logic

- Create `src/utils/twitter/twitteractivity_llm_execute.rs`.
- Move the `quote_tweet` function, which handles the complex DOM interaction involving finding the button, focusing the composer, typing the text, and clicking post.
- Bring over necessary constants like `QUOTE_CLICK_PAUSE_SHORT_MS` and `COMPOSER_WAIT_MS`.

## Step 3: Update Main Module

- Update `twitteractivity_llm.rs` to declare `pub mod twitteractivity_llm_validation;` and `pub mod twitteractivity_llm_execute;`.
- Add `pub use` statements so that external consumers of `twitteractivity_llm::validate_reply` and `twitteractivity_llm::quote_tweet` do not break.
- Ensure `twitteractivity_llm.rs` now only contains `generate_reply`, `generate_quote_commentary`, and `extract_tweet_context`.

## Step 4: Verification

- Run `cargo test` to ensure all moved unit tests still pass.
- Run `cargo clippy` and `./check-fast.ps1` to confirm the module restructuring is clean.

# Internal API Outline

- `twitteractivity_llm_validation::validate_reply(text: &str) -> Result<String>`
- `twitteractivity_llm_execute::quote_tweet(api: &TaskContext, commentary: &str) -> Result<bool>`

# Decisions

- Use sub-modules within the same directory: This aligns with the existing project structure (`twitteractivity_*.rs`) rather than creating a nested folder, keeping the `src/utils/twitter/` structure flat but modular.
