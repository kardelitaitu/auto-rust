# TwitterActivity Contract Alignment

Status: `done`

Owner: `spec-agent`
Implementer: `archived-spec-agent`

## Summary

Align the TwitterActivity task with its documented payload contract. Replace the hardcoded duration fallback, make scroll count a real runtime input, reject malformed numeric payloads, and keep the task docs/config examples in sync with runtime behavior.

## Scope

- In scope:
  - `TaskConfig` payload defaults and validation
  - TwitterActivity run-loop scroll budget behavior
  - strict numeric payload parsing
  - regression tests for defaults, malformed payloads, and scroll behavior
  - docs/config example sync for the task contract
- Out of scope:
  - sentiment, persona, or decision-engine changes
  - browser discovery or shutdown refactors
  - CLI parsing or task-registry redesign
  - new Twitter/X task verbs

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

## Next Step

Hand this package to the implementer after the contract and test order are approved.

# Baseline

- `src/task/twitteractivity.rs` uses a time-only loop and never reads `feed_scroll_count` from the task config.
- `TaskConfig::from_payload()` in `src/utils/twitter/twitteractivity_state.rs` hardcodes `duration_ms` to `300_000` when payload input is missing.
- `read_u64()` and `read_u32()` in `src/utils/twitter/twitteractivity_state.rs` silently fall back to defaults when a payload field has the wrong type.
- `src/utils/twitter/twitteractivity_feed.rs` already has a `scroll_feed(api, scroll_count, use_native_scroll)` helper that can serve as the lower-level scrolling primitive.
- `docs/TASKS/twitteractivity.md`, `config/default.toml`, and `README.md` already describe `duration_ms` and `scroll_count`, so the public contract exists but the runtime does not fully honor it.
- The current gap is contract drift, not missing task wiring in unrelated modules.

