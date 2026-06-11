## Acceptance Criteria

- [ ] `TweetActionTracker::last_action` is `HashMap<TweetId, ...>` — no `String` keys for tweet IDs anywhere in the tracker
- [ ] `record_action()` accepts `TweetId`, not `String` — all call sites use `.clone()` not `.to_string()`
- [ ] `can_perform_action()` accepts `&TweetId`, not `&str`
- [ ] `TweetContext.tweet_id` is `TweetId`, not `String` — all strategy test code updated
- [ ] `engagement/mod.rs` line 197 extracts `tweet_id` as `TweetId` (fails on missing/malformed, not "unknown")
- [ ] `dispatch_action()` signature uses `&TweetId` for tweet_id parameter
- [ ] `validate_tweet_page()`, `action_allowed_by_limits()`, `selected_candidate_actions()` use `&TweetId`
- [ ] `status_id_from_url()` delegates to `StatusUrl::tweet_id()`
- [ ] No raw `String` or `&str` used for tweet IDs or status URLs in twitter subsystem (outside of `TweetId`/`StatusUrl` internals)
- [ ] All test code uses `TweetId::from_unchecked("id")` for literal tweet IDs

## Test Commands

- `cargo check --lib` — no compilation errors
- `cargo test -p auto-rust` — all 3,400+ tests pass, 0 failures
- `cargo clippy --all-targets -- -D warnings` — no warnings
- `cargo fmt --all -- --check` — no formatting issues
- `pwsh -File check.ps1` — all 5 steps pass (spec-lint, build, format, clippy, nextest)

## Visual Inspection

- `git diff --stat` shows changes only in the 15 files listed in spec.yaml
- No behavioral changes in any function body — only type signature updates and `.to_string()` → `.clone()` replacements
- No new `unwrap()` or `expect()` calls except the JSON extraction in engagement/mod.rs (which replaces a silent `unwrap_or("unknown")`)
- `status_id_from_url()` in twitteractivity_dive.rs is either removed or delegates to `StatusUrl`
- `tweet_url: String` → `tweet_url: StatusUrl` in task files — navigation calls unchanged since `StatusUrl: Deref<Target=str>`
