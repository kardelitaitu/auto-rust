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
