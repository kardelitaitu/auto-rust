# Plan — Replace ambiguous `Result<bool>` with typed outcome enums

## Baseline

32 functions across the Twitter subsystem return `Result<bool>` where `false` is ambiguous:

| Function | File | What `false` means |
|---|---|---|
| `like_tweet(api) -> Result<bool>` | `twitteractivity_interact.rs:176` | Button not found? Already liked? Error swallowed? |
| `retweet_tweet(api) -> Result<bool>` | `twitteractivity_interact.rs:254` | No retweet button? Already retweeted? |
| `reply_to_tweet(api, text) -> Result<bool>` | `twitteractivity_interact.rs:526` | Composer not found? Post failed? |
| `follow_from_tweet(api) -> Result<bool>` | `twitteractivity_interact.rs:566` | Button not found? Already following? |
| `bookmark_tweet(api) -> Result<bool>` | `twitteractivity_interact.rs:657` | Already bookmarked? Button absent? |
| `quote_tweet(api, commentary) -> Result<bool>` | `twitteractivity_llm_execute.rs:23` | Quote button missing? Post failed? |
| `post_reply(api) -> Result<bool>` | `twitterreply.rs:216` | Button not found? Twitter rejected? |
| `post_quote(api) -> Result<bool>` | `twitterretweet.rs:250` / `twitterquote.rs:249` | Same ambiguity |
| `robust_follow(api, username) -> Result<bool>` | `twitterfollow.rs:84` | Exhausted retries? Already following? No button? |

**What should stay `bool`:** Predicate functions like `is_already_following()`, `check_soft_error()`, `is_on_home_feed()`, `is_on_tweet_page()`, `check_end_of_thread()` — these answer a yes/no question and `bool` is correct.

## Design: Outcome Enums

### `EngagementOutcome` (for like/retweet/reply/bookmark/quote)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngagementOutcome {
    /// Action completed successfully
    Completed,
    /// Action was already performed (e.g., already liked)
    AlreadyDone,
    /// Required UI element not found (button, composer, etc.)
    ElementNotFound,
    /// Action failed after attempt (network, timing, etc.)
    Failed,
}
```

### `FollowOutcome` (for follow_from_tweet, robust_follow)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowOutcome {
    /// Successfully followed
    Followed,
    /// Already following this user
    AlreadyFollowing,
    /// Follow button not visible/found
    ButtonNotFound,
    /// Follow attempted but failed (retries exhausted, verification failed)
    Failed,
}
```

### `PostOutcome` (for post_reply, post_quote)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostOutcome {
    /// Post confirmed successful
    Posted,
    /// Composer or post button not found
    ComposerNotFound,
    /// Post attempted but failed
    Failed,
}
```

These enums live in `src/utils/twitter/twitteractivity_types.rs` alongside `TweetId` and `StatusUrl`.

## Implementation Steps

### Phase 1: Define enums

1. Add `EngagementOutcome`, `FollowOutcome`, `PostOutcome` to `twitteractivity_types.rs` with `#[derive(Debug, Clone, PartialEq, Eq)]` and doc comments.
2. Add `impl` blocks with convenience methods if useful (e.g., `is_completed() -> bool` for backward compat).

### Phase 2: Update integration test helpers

3. Update `src/tests/twitter_helpers.rs` — if any test helpers assert on `bool` outcomes, update to match on enum variants.

### Phase 3: Update public API functions (6 files)

4. **`twitteractivity_interact.rs`** — `like_tweet()` → `Result<EngagementOutcome>`, `retweet_tweet()` → `Result<EngagementOutcome>`, `reply_to_tweet()` → `Result<EngagementOutcome>`, `follow_from_tweet()` → `Result<FollowOutcome>`, `bookmark_tweet()` → `Result<EngagementOutcome>`. Internal helpers (`click_reply_button`, `send_reply`, `click_retweet_button`) update signatures. Update test code.

5. **`twitteractivity_actions.rs`** — `like_at_position()` → `Result<EngagementOutcome>`.

6. **`twitteractivity_llm_execute.rs`** — `quote_tweet()` → `Result<EngagementOutcome>`.

### Phase 4: Update task files (4 files)

7. **`twitterfollow.rs`** — `robust_follow()` → `Result<FollowOutcome>`. Update `run_inner()` call site to match on outcome variants. Update all tests.

8. **`twitterreply.rs`** — `post_reply()` → `Result<PostOutcome>`, `post_reply_with_retry()` → `Result<PostOutcome>`. Update caller.

9. **`twitterretweet.rs`** — `post_quote()` / `post_quote_with_retry()` → `Result<PostOutcome>`. Update caller.

10. **`twitterquote.rs`** — `post_quote()` / `post_quote_with_retry()` → `Result<PostOutcome>`. Update caller.

### Phase 5: Update engagement dispatch

11. **`twitteractivity_engagement` / dispatch module** — If `dispatch_action()` checks `bool` returns, update to match on enum variants.

### Phase 6: Finalize

12. `cargo check --tests`
13. `cargo test -p auto-rust -- twitter`
14. `cargo test -p auto-rust` (full suite)
15. `cargo fmt --check && cargo clippy -- -D warnings`

## API Changes

- **`like_tweet()`**: `Result<bool>` → `Result<EngagementOutcome>`
- **`retweet_tweet()`**: `Result<bool>` → `Result<EngagementOutcome>`
- **`reply_to_tweet()`**: `Result<bool>` → `Result<EngagementOutcome>`
- **`follow_from_tweet()`**: `Result<bool>` → `Result<FollowOutcome>`
- **`quote_tweet()`**: `Result<bool>` → `Result<EngagementOutcome>`
- **`bookmark_tweet()`**: `Result<bool>` → `Result<EngagementOutcome>`
- **`robust_follow()`**: `Result<bool>` → `Result<FollowOutcome>`
- **`post_reply()` / `post_quote()`**: `Result<bool>` → `Result<PostOutcome>`
- All are internal to the Twitter subsystem — no public library API changes

## Validation

- `cargo check --tests` — must compile clean
- `cargo test -p auto-rust -- twitter` — all twitter tests pass with updated assertions
- `cargo test -p auto-rust` — full test suite passes

## Design Decisions and Risks

**Why three enums instead of one?** `EngagementOutcome` covers like/retweet/reply/quote/bookmark (same semantics). `FollowOutcome` has different states (AlreadyFollowing vs AlreadyDone). `PostOutcome` is narrower and avoids ElementNotFound ambiguity (post operations always have the composer visible). Separate enums prevent accidental mixing and keep match arms exhaustive.

**Risk: engagement dispatch pipeline.** The `dispatch_action` function in the engagement module may pattern-match on `bool` returns. Must update in lockstep with the function signatures.

**Risk: retry logic.** `robust_follow` and `post_*_with_retry` have retry loops that check `bool`. Must update to check enum variants (e.g., `FollowOutcome::Followed` vs `FollowOutcome::Failed`).

**Confidence: Medium-High.** Type-safe, compiler-enforced, but touches 10+ files with non-trivial control flow. Retry loops need careful review.
