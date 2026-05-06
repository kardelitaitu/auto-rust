# Baseline

## Current State

- `TODO.md` still lists `src/task/*.rs` as the next medium-priority coverage candidate.
- The specific files called out are `twitteractivity.rs`, `twitterfollow.rs`, and `twitterintent.rs`.
- `twitteractivity.rs` has a small helper surface around payload fixtures and entry-point selection.
- `twitterfollow.rs` contains locator candidate ordering plus URL and username parsing helpers.
- `twitterintent.rs` contains intent parsing, payload URL extraction, and query-parameter helpers.

## Known Gaps

- The three target files still have uneven coverage, with some pure helper branches not directly exercised.
- The broad `src/task/*.rs` area is too large for one pass, so this spec should stay on the three named modules.
- Browser-backed tests can mask the real gap if the work drifts away from helper coverage.

## Why This Matters

These modules sit on the task execution path and carry most of the parsing and branch-control logic for Twitter-oriented task behavior. The safest improvement is to add deterministic tests around the pure helpers first, then only add narrow integration checks if a behavior path cannot be covered otherwise.
