# Plan

## Step 1: Fix Pauses and Imports

- Replace all explicit internal capability imports with `use crate::prelude::*;`.
- Replace `human_pause(api, ...)` and `api.pause(...)` with `api.pause_human(...)`.

## Step 2: Refactor Scrolling

- Remove raw JS `window.scrollBy(...)`.
- Implement `crate::capabilities::scroll::human_scroll` or `random_scroll` within the main dive loop.
- Apply realistic backtracking using the scroll capability module rather than raw JS.

## Step 3: Robust End-of-Thread Detection

- Rewrite `check_end_of_thread`.
- Remove the `articles.length > 1` check which breaks on virtualized lists.
- Implement a check that verifies if the scroll position actually changed after a scroll attempt, combining it with the `showMoreThread` button check.

## Step 4: Accurate Metric Tracking

- Change the `tweets_read` counter.
- Track unique tweet identifiers by maintaining a `HashSet<String>` in Rust.
- During each scroll loop iteration, execute a lightweight JS snippet to extract the `data-item-id`, `data-tweet-id`, or permalink IDs of all visible `article[data-testid="tweet"]` elements.
- Insert the extracted IDs into the `HashSet`. The final `tweets_read` metric will be the length of this set.

## Step 5: Verification

- Run `cargo clippy` and `cargo test`.
- Ensure the task still completes within its duration budget.
- Verify `check_fast.ps1` passes.

# Internal API Outline

- `api.pause_human(duration_ms: u64)` -> `Result<()>`
- `scroll::human_scroll(page: &Page, direction: &str, amount: i32)` -> `Result<()>`
- `api.get_scroll_position()` -> `Result<(f64, f64)>` (for delta checking)

# Decisions

- Use scroll delta for end-of-thread: Chosen because virtualized lists remove DOM nodes, making `querySelectorAll` unreliable.
- Track unique IDs for read count: Chosen because the bot might need multiple small scrolls to pass a single long tweet.
