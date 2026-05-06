# Plan

## Step 1: Selector for Thread Replies

Modify `src/utils/twitter/twitteractivity_selectors.rs`.
- Implement `js_identify_thread_replies`.
- This JS should:
  - Find all `article[data-testid="tweet"]` elements.
  - Skip the first one (the root tweet).
  - Extract text and button coordinates (Like button only) for the rest.
  - Return a list of `ReplyCandidate` objects.

## Step 2: Update Engagement Sub-Loop

Modify `src/utils/twitter/twitteractivity_engagement.rs`.
- In `process_candidate`, after the `if success` block for the root action:
  - If `did_dive` is true:
    - Call `identify_thread_replies(api)`.
    - For each reply candidate (limited to 2-3):
      - Run `decide_engagement` (Smart Decision).
      - If score > 30 (Medium/High quality):
        - Perform `like_at_position`.
        - Increment counters and `actions_taken`.
        - Pause for human-like reading (1-2s).

## Step 3: Refine Navigation

Modify `process_candidate`'s "Navigate back to home" logic.
- Ensure the "back to home" wait time is slightly longer if we engaged with multiple items, simulating a deep read.

## Step 4: Verification

- Run `cargo test` to ensure existing engagement flows still work.
- Verify through logs that "Sub-engagement with reply" entries appear.
- Ensure global limits are still enforced (e.g., if we like 3 replies, the global like counter must reflect this).

## Rollback

- Revert the `if did_dive` sub-loop in `process_candidate`.

# Internal API Outline

## Contract

After a successful `dive_into_thread`, the automation must check for high-quality replies before leaving.
It must identify replies that meet a "Like" threshold and apply the action without leaving the page.

## Inputs

- `max_replies_per_thread`: A new configuration or local limit (default: 2-3).
- `api: &TaskContext`: For DOM interaction.

## Outputs

- Updated `actions_taken` count.
- Updated `EngagementCounters` (likes).

## State Changes

- None (besides counter increments).

## Error Paths

- Thread view fails to load (already handled).
- No engageable replies found (proceed to exit).
- Engagement limit hit during sub-loop (break and exit).

## Invariants

- Must NOT perform automated replies to replies (safety constraint: Likes only for now).
- Must NOT stay in a thread for more than 2-3 scrolls to avoid getting stuck.

# Decisions

## Decision Log

- **Likes Only for Replies**: Decided to restrict reply engagement to "Like" only. Automated replies to replies are much higher risk for bot detection and negative community sentiment if the template doesn't fit the context perfectly.
- **Top-Level Only**: We will only scan the top-level replies visible on the initial dive (plus a small scroll). Navigating deeper into nested threads is technically complex and increases the risk of the bot getting lost.

## Open Questions

- Should we make `max_replies_per_thread` a user-configurable setting in `default.toml`? (Starting with a hardcoded range of 1-3 for now).

