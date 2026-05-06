# Twitter Modal Awareness

Status: `done`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Twitter frequently opens tweet threads in an overlay modal (`div[role="dialog"]`) while keeping the user on the home feed URL (`x.com/home`). The current automation fails to recognize this as being on a "tweet page" and often targets background tweets instead of the one in the modal. This initiative makes the automation modal-aware by enhancing context validation and prioritizing modal selectors.

## Scope

- **In scope**:
  - Updating `is_on_tweet_page` to detect visibility of `div[role="dialog"]` or `div[data-testid="tweetDetail"]`.
  - Updating `js_root_tweet_button_center` to prioritize elements within an active modal.
  - Ensuring engagement loops (Like, Retweet, Reply) work correctly within modals.
- **Out of scope**:
  - Fixing issues with the home feed scanning itself (only the engagement/dive part).
  - General UI layout changes to the dashboard.

## Files

- `spec.yaml`
- `baseline.md`
- `internal-api-outline.md`
- `plan.md`
- `validation-checklist.md`
- `ci-commands.md`
- `decisions.md`
- `quality-rules.md`
- `implementation-notes.md`

## Rules

- Keep the spec short.
- Run `spec-lint.ps1` before handoff.
- Use `.\check-fast.ps1` while iterating.
- Use the archive helper `.\spec-archive.ps1` to move to `_done/`.

## Next Step

Implement the enhanced context validation in `twitteractivity_interact.rs`.

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

