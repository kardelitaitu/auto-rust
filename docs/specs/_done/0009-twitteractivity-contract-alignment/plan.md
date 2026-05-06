# Plan

## Step 1

Record the current TwitterActivity contract drift and decide the source of truth for duration and scroll count.

## Step 2

Tighten payload parsing so malformed numeric values fail validation instead of falling back silently.

## Step 3

Wire the scroll budget into the TwitterActivity runtime so the task actually honors `scroll_count`.

## Step 4

Add deterministic tests for default duration resolution, explicit overrides, malformed payloads, and scroll budget behavior.

## Step 5

Update the task docs and examples so the public contract matches the runtime behavior.

## Step 6

Run the focused checks first, then the full gate.

# Internal Boundary Outline

- `TaskConfig::from_payload`
  - owns payload parsing, defaults, and validation for TwitterActivity
  - should expose explicit runtime fields for duration and scroll budget
- `twitteractivity.rs::run_inner`
  - owns the task execution contract
  - should use `TaskConfig` values only and avoid hidden fallback logic
- `twitteractivity_feed::scroll_feed`
  - owns the lower-level feed scrolling primitive
  - should stay reusable and not absorb contract decisions
- `read_u64` / `read_u32`
  - should reject malformed numeric payloads instead of silently substituting defaults
- docs/config files
  - should mirror runtime behavior, not define it independently

# Decisions

| Choice | Pros | Cons |
|---|---|---|
| Keep the time-only loop and delete `scroll_count` docs/config | Smallest code change | Breaks the public contract and leaves docs wrong |
| Wire `scroll_count` into the runtime contract | Matches docs and makes behavior explicit | Slightly larger refactor |
| Keep permissive numeric parsing | Tolerates malformed inputs | Hides bad payloads and creates silent drift |
| Make parsing strict | Fails fast on invalid payloads | Can break callers that relied on fallback behavior |

## Decision

Use the documented payload contract as the target behavior: strict numeric validation, config-backed defaults, and an explicit scroll budget in the runtime.

## Notes

- Keep `twitteractivity_feed::scroll_feed` as the reusable primitive.
- Keep docs updates as the final sync step after runtime behavior is fixed.

