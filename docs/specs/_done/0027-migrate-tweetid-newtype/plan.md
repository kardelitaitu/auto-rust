# Plan — Migrate String tweet IDs/status URLs to TweetId and StatusUrl

## Baseline

`TweetId` and `StatusUrl` already exist in `src/utils/twitter/twitteractivity_types.rs` with full implementations (`Display`, `Hash`, `Eq`, `From`, `FromStr`, `Deref`, `Clone`, 20+ tests). They are **not used anywhere** — all production code passes raw `String` and `&str` for tweet IDs and status URLs.

**Problems this fixes (from TODO.md Layer 1):**
- `String` everywhere means mixups are silent — tweet IDs can be swapped with usernames, action names, or URLs at compile time
- `HashMap<String, ...>` in `TweetActionTracker` allocates on every lookup
- `record_action(tweet_id.to_string())` allocates a new `String` on every action (2-3 per candidate tweet)
- `tweet.get("id").unwrap_or("unknown")` in `engagement/mod.rs` silently swallows missing IDs
- `status_url` is `Option<&str>` with no validation it's actually a status URL

**Audit summary (15 files, ~80 call sites):**

```
TweetActionTracker (HashMap<String,...>)  ← core, start here
  ↑ SessionState.record_action()
    ↑ engagement/mod.rs (2x record_action), dispatch.rs (2x), scoring.rs
      ↑ All task files (twitterlike, retweet, reply, quote, dive, follow, intent)

TweetContext.tweet_id: String            ← decision engine types
  ↑ 4 decision strategies + all their test code

twitterdive::status_id_from_url()        ← duplicated StatusUrl logic
twitterfollow::is_tweet_url()            ← should delegate to StatusUrl
twitterintent::extract_param(tweet_id)   ← URL parsing
engagement/mod.rs: status_url extraction ← Option<&str>, no type safety
```

## Implementation Steps

### Phase 1: Core tracker (central choke point)

1. Open `src/utils/twitter/state/tracking.rs`
2. Change `HashMap<String, (&'static str, Instant)>` → `HashMap<TweetId, (&'static str, Instant)>`
3. Change `record_action(tweet_id: String, ...)` → `record_action(tweet_id: TweetId, ...)`
4. Change `can_perform_action(tweet_id: &str)` → `can_perform_action(tweet_id: &TweetId)`
5. Add `use super::super::twitteractivity_types::TweetId;` import
6. Update all internal test code in tracking.rs to use `TweetId::from_unchecked("tweet_1")`

### Phase 2: SessionState pass-through

7. Open `src/utils/twitter/state/session.rs`
8. Change `record_action(&self, tweet_id: &str, ...)` → `record_action(&self, tweet_id: &TweetId, ...)`
9. Update delegation: `self.action_tracker.record_action(tweet_id.clone(), action_type)` (clone is cheap)
10. Update all test code in session.rs

### Phase 3: Engagement files (highest volume)

11. Open `src/utils/twitter/engagement/mod.rs`
12. **Critical fix:** Replace lines 197-200:
    ```rust
    // OLD: silently defaults to "unknown"
    let tweet_id = tweet.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    // NEW: fail fast
    let tweet_id = TweetId::new(
        tweet.get("id").and_then(|v| v.as_str()).ok_or_else(|| anyhow::anyhow!("tweet missing 'id' field"))?
    )?;
    ```
13. Replace `action_tracker.record_action(tweet_id.to_string(), "dive")` → `action_tracker.record_action(tweet_id.clone(), "dive")` (lines 260, 313)
14. Change `tweet_id` type in `dispatch_action()` call from `&str` to `&TweetId`

15. Open `src/utils/twitter/engagement/dispatch.rs`
16. Change `dispatch_action(tweet_id: &str)` → `dispatch_action(tweet_id: &TweetId)` in signature
17. Change `validate_tweet_page` call to pass `tweet_id.as_ref()` or update validate_tweet_page's signature
18. Replace `action_tracker.record_action(tweet_id.to_string(), ...)` → `action_tracker.record_action(tweet_id.clone(), ...)`

19. Open `src/utils/twitter/engagement/scoring.rs`
20. Extract `tweet_id` as `TweetId` from JSON (same pattern as mod.rs line 197)

### Phase 4: Helper functions

21. Open `src/utils/twitter/twitteractivity_helpers.rs`
22. Change `validate_tweet_page(tweet_id: &str)` → `validate_tweet_page(tweet_id: &TweetId)`
23. Change `action_allowed_by_limits(tweet_id: &str)` → `action_allowed_by_limits(tweet_id: &TweetId)`
24. Change `selected_candidate_actions(tweet_id: &str)` → `selected_candidate_actions(tweet_id: &TweetId)`

### Phase 5: Decision engine types

25. Open `src/utils/twitter/decision/types.rs`
26. Change `TweetContext { pub tweet_id: String }` → `TweetContext { pub tweet_id: TweetId }`
27. Update all 4 strategy files (legacy, persona, llm, unified) and their test code to use `TweetId::from_unchecked("1")` instead of `"1".to_string()`

28. Open `src/utils/twitter/state/types.rs`
29. Change `CandidateContext` tweet_id field from `String` to `TweetId`

### Phase 6: Task files + URL types

30. Open `src/utils/twitter/twitteractivity_dive.rs`
31. Replace `status_id_from_url(status_url: &str) -> Option<&str>` with delegation to `StatusUrl`:
    ```rust
    fn status_id_from_url(url: &str) -> Option<&str> {
        StatusUrl::new(url).ok()?.tweet_id()
    }
    ```
    Or remove entirely and inline `StatusUrl::new(status_url)?.tweet_id()` at call sites.

32. Open `src/task/twitterfollow.rs`
33. Change `is_tweet_url(url: &str)`, `extract_username_from_tweet_url(url: &str)` to use `StatusUrl`
34. Change `tweet_to_profile_flow(tweet_url: &str)` → `tweet_to_profile_flow(tweet_url: &StatusUrl)`

35. Open remaining task files (`twitterlike.rs`, `twitterretweet.rs`, `twitterreply.rs`, `twitterquote.rs`, `twitterdive.rs`)
36. Change `tweet_url: String` → `tweet_url: StatusUrl` where applicable

37. Open `src/task/twitterintent.rs`
38. Change `extract_param(url, "tweet_id")` → return `TweetId` instead of `String`

### Phase 7: Test helpers

39. Open `tests/twitter_helpers.rs` (and any integration test files)
40. Replace all `"tweet_1".to_string()` with `TweetId::from_unchecked("tweet_1")`
41. Update `assert_tracker_allows(tweet_id: &str)` → `assert_tracker_allows(tweet_id: &TweetId)`
42. Update `assert_tracker_blocks(tweet_id: &str)` → `assert_tracker_blocks(tweet_id: &TweetId)`

### Phase 8: Finalize

43. Run `cargo check` — fix any remaining type errors
44. Run `cargo test -p auto-rust` — all 3,400+ tests must pass
45. Run `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings`
46. Run `check.ps1` — all 5 steps must pass

## API Changes

| Old | New | Impact |
|-----|-----|--------|
| `record_action(tweet_id: String, ...)` | `record_action(tweet_id: TweetId, ...)` | Callers use `.clone()` instead of `.to_string()` — cheaper, clearer |
| `can_perform_action(tweet_id: &str)` | `can_perform_action(tweet_id: &TweetId)` | Callers pass `&tweet_id` (already Deref to str) |
| `TweetContext { tweet_id: String }` | `TweetContext { tweet_id: TweetId }` | Test code uses `.into()` or `::from_unchecked()` |
| `dispatch_action(tweet_id: &str)` | `dispatch_action(tweet_id: &TweetId)` | Callers already have TweetId from JSON extraction |
| `tweet_url: String` in task files | `tweet_url: StatusUrl` | Navigation APIs unchanged (StatusUrl: Deref to str) |

## Design Decisions and Risks

**Why bottom-up (tracker first)?** `TweetActionTracker` is the central HashMap — changing its key type forces all upstream callers to provide `TweetId`. The compiler guides the migration — no manual hunting for call sites.

**Why `TweetId::new()` for JSON extraction but `from_unchecked()` for test literals?** JSON values can be missing or empty — must handle errors. Test literals like `"tweet_1"` are known-valid at compile time — `from_unchecked` avoids redundant error handling.

**Why clone instead of `to_string()`?** `TweetId` wraps a `String` — `clone()` is a cheap ref-count copy. `to_string()` would allocate a new `String` and re-validate. Use clone at call sites that need ownership.

**Risk: TweetId::new() in engagement/mod.rs line 197 changes error semantics.** Currently `unwrap_or("unknown")` silently proceeds with a garbage ID. The new code returns `Err` — the caller (`process_candidate`) already returns `Result<CandidateResult>`, so this just makes the error visible instead of hidden.

**Risk: ~80 call sites across 15 files.** Mechanical but high-touch. Mitigation: 3,400+ existing tests provide comprehensive regression coverage. The compiler catches all type mismatches — no silent breakage possible.

**Confidence: High.** TweetId/StatusUrl are already implemented and tested. The migration is purely mechanical — replace `String`/`&str` with typed wrappers, fix compiler errors, verify tests pass.
