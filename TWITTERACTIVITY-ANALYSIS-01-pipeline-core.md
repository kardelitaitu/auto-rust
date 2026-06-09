# Twitteractivity Analysis — Group 1: Pipeline Core

**Files:** `actions.rs` (99 lines), `constants.rs` (19 lines), `types.rs` (366 lines)
**Audited:** 09 June 2026

---

## twitteractivity_actions.rs — CLEAN

| Function | Lines | Verdict | Notes |
|----------|-------|---------|-------|
| `extract_tweet_text` | 17-24 | OK | Falls back `text` → `full_text`; returns `String::new()` on missing |
| `extract_tweet_button_position` | 28-38 | OK | Safe `and_then` chain, returns `Option<(f64, f64)>` |
| `like_at_position` | 41-63 | OK | Coordinate click + hover + pauses + JS verification; `?` propagates errors |
| `generate_reply_text` | 67-81 | OK | Modulo template selection; empty-guard before indexing |
| `generate_quote_text` | 85-99 | OK | Same pattern as `generate_reply_text` |

**No bugs found.** All functions use safe patterns (no unwrap/expect in production paths).

---

## twitteractivity_constants.rs — CLEAN

| Constant | Value | Notes |
|----------|-------|-------|
| `DEFAULT_TWITTERACTIVITY_DURATION_MS` | 300_000 (5 min) | — |
| `MIN_CANDIDATE_SCAN_INTERVAL_MS` | 2500 | — |
| `MIN_ACTION_CHAIN_DELAY_MS` | 3000 | — |
| `MAX_CONSECUTIVE_SCROLL_FAILURES` | 3 | Now config-driven per note |
| `MAX_CONSECUTIVE_EMPTY_SCANS` | 3 | Now config-driven per note |

**No bugs found.**

---

## twitteractivity_types.rs — CLEAN

### TweetId — CLEAN
- Validates non-empty on construction.
- Traits: Display, AsRef, Deref, PartialEq, Eq, Hash, From<String>, From<&str>, FromStr.
- `from_unchecked()` for bypass.

### StatusUrl — CLEAN
- Validates non-empty on construction.
- `tweet_id()`: parses `/status/` segment with query/fragment/trailing-slash stripping.
- Same trait surface as `TweetId`.

### Minor observations
- Error type is `Result<Self, String>` rather than `anyhow::Error` — callers using `anyhow::Result` need `.map_err(|e| anyhow::anyhow!(e))?`. Style choice, not a bug.
- 16 unit tests cover all paths including edge cases.

**No bugs found.**

---

## Group Summary

| Metric | Count |
|--------|-------|
| Total lines | ~484 |
| Bugs found | 0 |
| Unsafe blocks | 0 |
| Production unwrap/expect | 0 |
| Unit tests | 16 (all in types.rs) |
