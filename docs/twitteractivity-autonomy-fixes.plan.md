# Implementation Plan: Twitter Activity Autonomy Fixes

## Scope

P0 (must fix) + P1 (strongly recommended) from the audit doc. 9 files changed (1 new).

---

## P0.1 — Separate Dive from Action Selection (H3)

**Root cause**: A single `should_dive()` gate (20% pass rate) strips ALL non-like actions. 80% of candidates become like-only.

### Layer 1: `twitteractivity_helpers.rs` — Fix the gate function

Change `filter_detail_actions_for_gate()` to only strip reply+quote instead of all non-like actions:

```rust
// BEFORE:
if actions_to_do.iter().any(|&action| action != "like") && (!has_status_url || !dive_allowed) {
    actions_to_do.retain(|&action| action == "like");
}

// AFTER:
let detail_needed = actions_to_do.iter().any(|&a| a == "reply" || a == "quote");
if detail_needed && (!has_status_url || !dive_allowed) {
    actions_to_do.retain(|&action| action != "reply" && action != "quote");
}
```

### Layer 2: `engagement/mod.rs` — Fix `need_dive` / `needs_detail_view`

Two changes in `process_candidate()`:

```rust
// Line ~258: needs_detail_view
// BEFORE: .any(|&action| action != "like")
// AFTER:  .any(|&a| a == "reply" || a == "quote")

// Line ~330: need_dive
// BEFORE: .any(|&action| action != "like")
// AFTER:  .any(|&a| a == "reply" || a == "quote")
```

### Layer 3: `engagement/dispatch.rs` + `twitteractivity_actions.rs`

Add position-based paths for retweet, follow, bookmark when `!did_dive`:

In `dispatch.rs` — for each of retweet/follow/bookmark:
```rust
if !did_dive {
    if let Some(pos) = extract_tweet_button_position(tweet, "retweet") {
        match retweet_at_position(api, pos.0, pos.1).await { ... }
    } else {
        // fallback to validate_tweet_page (will fail gracefully)
    }
} else {
    // existing selector-based logic
}
```

In `twitteractivity_actions.rs` — add these following `like_at_position()` pattern:
- `retweet_at_position(api, x, y)` — click retweet button, then confirm
- `follow_at_position(api, x, y)` — click follow button
- `bookmark_at_position(api, x, y)` — click bookmark button

---

## P0.2 — Reset `next_candidate_scan` After Dive (H1)

**Root cause**: `next_candidate_scan` only reset inside `goto_home()` success block. Stale on failure.

**Fix in `engagement/mod.rs`**: Move timer reset after the `goto_home()` block, guarded by `did_dive`:

```rust
if did_dive {
    next_scroll = Instant::now() + scroll_interval;
    next_candidate_scan = Instant::now() + scroll_interval;
    info!("[dive] Resumed continuous scrolling after thread dive");
}
```

Also add `next_candidate_scan` reset in the dive-failure path (`dive_outcome.opened = false`).

---

## P0.3 — Inter-Session Persistence

**New file**: `src/utils/twitter/twitteractivity_persistence.rs`

- `TwitterPersistenceState` struct (serde Serialize/Deserialize)
- Fields: `last_session_end`, `daily_action_counts`, `last_rate_limit_timestamp`
- Path: `~/.config/auto-rust/twitter-state.json`
- Methods: `load()`, `save()`, `record_action()`, `record_session_end()`

**Config**: Add `persistence_enabled: bool` to `TwitterActivityConfig`, default `false`.

**Integration in `twitteractivity.rs`**: Load on init, save at session end.

---

## P1.1 — Fix `extract_thread_context` Hardcoded Fields (M9/H4)

**In `sentiment/helpers.rs`**: Parse `in_reply_to_status_id` (or `_str`), `is_quote`, `conversation_id` from tweet JSON instead of hardcoding `false/false/0`.

---

## Files Changed

| # | File | Change |
|---|------|--------|
| 1 | `twitteractivity_helpers.rs` | Gate only reply/quote, not all non-like |
| 2 | `engagement/mod.rs` | Fix `needs_detail_view`, `need_dive`, timer reset |
| 3 | `engagement/dispatch.rs` | Position-based dispatch for retweet/follow/bookmark |
| 4 | `twitteractivity_actions.rs` | Add `retweet_at_position`, `follow_at_position`, `bookmark_at_position` |
| 5 | `sentiment/helpers.rs` | Parse `in_reply_to_status_id`, `is_quote`, `conversation_id` |
| 6 | `config/types.rs` | Add `persistence_enabled` field |
| 7 | `twitteractivity.rs` | Load/save persistence on session boundaries |
| 8 | `twitteractivity_persistence.rs` | **NEW** — persistence module |
| 9 | `mod.rs` | Register new module |

## Verification

1. `cargo test -p auto -- twitteractivity` — all tests pass
2. Update test assertions in `engagement/tests.rs` for new gate behavior
3. Run simulation with `persistence_enabled=true`
