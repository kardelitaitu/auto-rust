last audited 16-06-26 by opencode

# Enhance Twitter Activity Logging and Robustness

## Baseline
- Payload validation errors in `src/task/twitteractivity.rs` bubble up but are not logged locally at validation time.
- Phase 1 navigation or login check failures bubble up with `?` but are not logged locally.
- In `scan_and_process_candidates`, if `process_candidate` fails for a single candidate, the entire loop returns `Err(e)` and aborts the task instead of catching the error, logging it, and proceeding with other candidates.
- In `dispatch_action` within `src/utils/twitter/engagement/dispatch.rs`, when individual engagement actions (like, retweet, quote, follow, reply, bookmark) return unsuccessful outcomes (such as `ElementNotFound` or `Failed`), the functions return `false` without warning logs.

## Implementation Steps
1. Modify `src/task/twitteractivity.rs`:
   - In `run()`, log an `error!` if payload validation fails.
   - In `run_inner()`, log an `error!` if `phase1_navigation()` fails.
   - In `scan_and_process_candidates()`, catch errors from `process_candidate(ctx, ...)` using a `match` expression, log them with `error!`, and `continue` processing the remaining candidates.
2. Modify `src/utils/twitter/engagement/dispatch.rs`:
   - Add helper functions `log_engagement_failure(outcome: &EngagementOutcome, action: &str, tweet_id: &TweetId)` and `log_follow_failure(outcome: &FollowOutcome, tweet_id: &TweetId)` to log clear warnings when actions fail or are not found (excluding expected skip cases like `AlreadyDone`/`AlreadyFollowing` which should log at `info!` level).
   - In `dispatch_action()`, where `engagement_success` or `follow_success` returns `false` (meaning the outcome was not success), call the corresponding log helper.
3. Modify `src/utils/twitter/engagement/mod.rs`:
   - In the dispatch loop inside `process_candidate()`, match on the result of `dispatch_action()`. If it returns `Err(e)`, log it with `error!`.
4. Modify `src/utils/twitter/twitteractivity_navigation.rs`:
   - In `navigate_and_read()`, log `error!` if navigation to the entry point or home fails before bubbling the error.

## API Changes
No API changes.

## Validation
- Run unit/integration tests: `cargo test`
- Run fast checks: `powershell -ExecutionPolicy Bypass -File .\check-fast.ps1`
- Run full codebase check: `powershell -ExecutionPolicy Bypass -File .\check.ps1`

## Design Decisions and Risks
- **Logging Level Selection:** For expected actions that are skipped (e.g., `AlreadyDone`/`AlreadyFollowing`), we log at `info!` level since they are not execution errors. For actual failures (e.g., `ElementNotFound`/`Failed`), we log at `warn!` level.
- **Robustness in Loop:** Skipping a single candidate failure instead of aborting the task ensures the automation is resilient to weird DOM structures of individual tweets.
- **Confidence Level:** High.
