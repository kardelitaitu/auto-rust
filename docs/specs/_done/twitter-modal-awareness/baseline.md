# Baseline

## Current Behavior

1.  **Context Validation**: `is_on_tweet_page` strictly checks `url.contains("/status/")`. When Twitter opens a thread in a modal on the home page, the URL remains `x.com/home`, causing this check to return `false`.
2.  **Selector Logic**: `js_root_tweet_button_center` attempts to match a status ID from the URL to find the correct `article[data-testid="tweet"]`. If no ID is found in the URL (common in modals), it defaults to `articles[0]`, which is usually the first tweet in the background feed.

## Why It Needs Work

- **Reliability Gap**: Automation fails to engage with threads opened via modals, skipping actions because it thinks the "dive" failed.
- **Accuracy Gap**: When it does try to engage, it may click a background tweet instead of the modal tweet, leading to unintended likes/retweets or complete failure to find buttons.

## Relevant Files

- `src/utils/twitter/twitteractivity_interact.rs`
- `src/utils/twitter/twitteractivity_selectors.rs`

## Known Failure Modes

- Engagement loop returns early with "dive failed" logs.
- Incorrect tweet engagement (interacting with background instead of foreground).

## Evidence

- Observer reports of "dive failed" even when thread is clearly visible on screen.
- Code analysis of `is_on_tweet_page` showing pure URL-based logic.
