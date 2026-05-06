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
