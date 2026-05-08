# Plan

## Step 1: Create Strategies Directory

- Create `src/utils/twitter/sentiment/strategies/` folder.
- Add `mod.rs` to expose the strategies internally.

## Step 2: Migrate Implementations

- Move the contents of `twitteractivity_sentiment_emoji.rs` into `sentiment/strategies/emoji.rs`.
- Move the contents of `twitteractivity_sentiment_domains.rs` into `sentiment/strategies/domain.rs`.
- Move the contents of `twitteractivity_sentiment_llm.rs` into `sentiment/strategies/llm.rs`.
- Move any remaining context/negation constants from `twitteractivity_sentiment_context.rs` that aren't already duplicated in `analyzer.rs`.

## Step 3: Update Analyzer

- Modify `src/utils/twitter/sentiment/analyzer.rs` to import from the new `strategies/` modules instead of the old root modules.
- Ensure the `SentimentStrategy` trait implementations for `EmojiStrategy`, `DomainStrategy`, etc., point to the newly relocated functions.

## Step 4: Delete Zombie Files

- Delete `src/utils/twitter/twitteractivity_sentiment.rs`
- Delete `src/utils/twitter/twitteractivity_sentiment_context.rs`
- Delete `src/utils/twitter/twitteractivity_sentiment_domains.rs`
- Delete `src/utils/twitter/twitteractivity_sentiment_emoji.rs`
- Delete `src/utils/twitter/twitteractivity_sentiment_enhanced.rs`
- Delete `src/utils/twitter/twitteractivity_sentiment_llm.rs`

## Step 5: Clean Up Module Exports

- Edit `src/utils/twitter/mod.rs`.
- Remove all `pub mod twitteractivity_sentiment_*;` declarations.
- Remove all `pub use twitteractivity_sentiment_*::*;` statements.

## Step 6: Verification

- Run `cargo check` and `cargo test` to ensure the compilation succeeds and all sentiment tests still pass.
- Verify line counts and file structure match the expected consolidated state.

# Internal API Outline

- `src/utils/twitter/sentiment/strategies/emoji.rs`
- `src/utils/twitter/sentiment/strategies/domain.rs`
- `src/utils/twitter/sentiment/strategies/llm.rs`

# Decisions

- Enforce standard Rust module hierarchy: The strategy pattern demands that implementations live within or beneath the module defining the trait (`sentiment/`), preventing pollution of the parent `twitter/` namespace.
