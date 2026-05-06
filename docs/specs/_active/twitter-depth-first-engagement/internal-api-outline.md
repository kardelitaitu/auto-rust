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
