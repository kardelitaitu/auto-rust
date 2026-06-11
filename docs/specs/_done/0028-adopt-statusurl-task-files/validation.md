## Acceptance Criteria

- [x] `extract_url_from_payload()` returns `Result<StatusUrl>` in all 7 task files
- [x] `twitterfollow.rs` `is_tweet_url()` delegates to `StatusUrl` (uses `tweet_id().is_some()`)
- [x] `twitterfollow.rs` `extract_username_from_tweet_url()` parses from `StatusUrl` path
- [x] `twitterdive.rs` `extract_visible_tweet_ids()` returns `Vec<TweetId>` instead of `Vec<String>`
- [x] `twitterlike.rs` `tweet_url.is_empty()` check updated to work with `StatusUrl`
- [x] No raw `String` tweet URLs remain in any task file
- [x] All test assertions updated to use `.as_str()` for `StatusUrl` comparisons
- [x] `cargo check --tests` passes with 0 errors
- [x] `cargo test -p auto-rust -- task` passes with 0 failures (pre-existing libyml proptest excluded)
- [x] `cargo fmt --check` passes with 0 diffs
- [x] `cargo clippy --all-targets -- -D warnings` passes with 0 warnings

## Test Commands

```powershell
cargo check --tests
cargo test -p auto-rust -- task
cargo test -p auto-rust
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

## Visual Inspection

- Confirm `extract_url_from_payload` return type is `Result<StatusUrl>` (not `Result<String>`) in all 7 task files
- Confirm `is_tweet_url()` and `extract_username_from_tweet_url()` bodies use `StatusUrl` methods
- Confirm `extract_visible_tweet_ids()` returns `Vec<TweetId>`
- Confirm no `.to_string()` calls on tweet URLs that are immediately passed to `api.navigate()`
