# Plan — Adopt StatusUrl/TweetId in task files and URL helpers

## Baseline

After spec 0027, `TweetId` and `StatusUrl` are used throughout the Twitter engagement pipeline (state, decision, engagement modules). However, the 7 task files still pass raw `String` for tweet URLs:

| File | Current state |
|---|---|
| `twitterlike.rs` | `extract_url_from_payload() -> Result<String>`, `tweet_url.is_empty()` check, `api.navigate(&tweet_url, …)` |
| `twitterretweet.rs` | `extract_url_from_payload() -> Result<String>`, `api.navigate(&tweet_url, …)` |
| `twitterreply.rs` | `extract_url_from_payload() -> Result<String>`, `api.navigate(&tweet_url, …)` |
| `twitterquote.rs` | `extract_url_from_payload() -> Result<String>`, `api.navigate(&tweet_url, …)` |
| `twitterdive.rs` | `extract_url_from_payload() -> Result<String>`, `api.navigate(&tweet_url, …)`. Also `extract_visible_tweet_ids() -> Result<Vec<String>>` |
| `twitterfollow.rs` | `extract_url_from_payload() -> Result<String>`, `is_tweet_url(&str) -> bool`, `extract_username_from_tweet_url(&str) -> Option<String>`, `tweet_to_profile_flow(api, tweet_url: &str)` |
| `twitterintent.rs` | `extract_param(url, "tweet_id")` already wrapped in `TweetId::from_unchecked()` (Phase 6). No further URL changes needed. |

`StatusUrl` implements `Deref<Target = str>`, so `api.navigate(&status_url, timeout)` works with zero overhead — no allocation, no `.as_str()` call needed.

## Implementation Steps

### Phase 1: Core URL type change (5 files)

1. **`twitterlike.rs`** — Change `extract_url_from_payload() -> Result<StatusUrl>`. Replace `tweet_url.is_empty()` with pattern matching on `StatusUrl::new()`. Update all tests to compare `status_url.as_str()`.

2. **`twitterretweet.rs`** — Change `extract_url_from_payload() -> Result<StatusUrl>`. Update tests.

3. **`twitterreply.rs`** — Change `extract_url_from_payload() -> Result<StatusUrl>`. Update tests.

4. **`twitterquote.rs`** — Change `extract_url_from_payload() -> Result<StatusUrl>`. Update tests.

5. **`twitterdive.rs`** — Change `extract_url_from_payload() -> Result<StatusUrl>`. Change `extract_visible_tweet_ids() -> Result<Vec<TweetId>>`. Update tests.

### Phase 2: twitterfollow.rs deduplication

6. **`twitterfollow.rs`** — Replace `is_tweet_url()` body with `StatusUrl::new(url).is_ok() && StatusUrl::from_unchecked(url).tweet_id().is_some()`. Replace `extract_username_from_tweet_url()` with `StatusUrl` path parsing. Change `tweet_to_profile_flow()` signature from `tweet_url: &str` to `tweet_url: &StatusUrl`. Update `extract_url_from_payload()` to return `Result<StatusUrl>`. Update all tests.

### Phase 3: Finalize

7. Run `cargo check --tests`
8. Run `cargo test -p auto-rust -- task`
9. Run `cargo fmt --check && cargo clippy -- -D warnings`
10. Run `cargo test -p auto-rust` (full suite)

## API Changes

- **`extract_url_from_payload()` return type**: `Result<String>` → `Result<StatusUrl>` in all 7 task files (internal change, task files are not library API)
- **`extract_visible_tweet_ids()`**: `Result<Vec<String>>` → `Result<Vec<TweetId>>` (internal to `twitterdive.rs`)
- **`is_tweet_url()` / `extract_username_from_tweet_url()`**: removed or delegated to `StatusUrl` (private to `twitterfollow.rs`)
- No public API changes

## Validation

- `cargo check --tests` — must compile clean
- `cargo test -p auto-rust -- task` — all task tests pass
- `cargo test -p auto-rust` — full test suite passes (minus pre-existing flaky tests)
- `cargo fmt --check && cargo clippy -- -D warnings` — clean

## Design Decisions and Risks

**Why now?** This is the natural continuation of spec 0027. The StatusUrl newtype already exists with full `Deref`, `Display`, `Hash`, `Eq` implementations. Adopting it in task files eliminates the last remaining `String`-typed tweet URLs in the codebase.

**Why not change `api.navigate()` signature?** Not needed. `StatusUrl: Deref<Target = str>` means `&status_url` coerces to `&str` automatically, so no call-site changes are required.

**Risk: `tweet_url.is_empty()` checks.** `twitterlike.rs` uses `tweet_url.is_empty()` to decide between feed mode and direct navigation. After the change, use `status_url.as_str().is_empty()` or restructure the flow to extract the URL first, then branch. Low risk.

**Risk: Test string comparisons.** Tests like `assert!(result.contains("x.com"))` need to use `.as_str()`. Mechanical change, low risk.

**Confidence: High.** Mechanical type migration with strong compiler enforcement. All code paths are well-tested.
