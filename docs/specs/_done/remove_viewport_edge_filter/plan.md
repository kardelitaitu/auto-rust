last audited 16-06-26 by opencode

# Remove Viewport Edge Filter from Candidate Selection

## Baseline
- `filter_candidates` in `src/utils/twitter/twitteractivity_feed.rs` filters out candidate tweets if their `y` coordinate is at or beyond 90% of the viewport height (`y >= viewport_height * 0.9`).
- Multiple unit tests verify this behavior and fail if it is removed.

## Implementation Steps
1. Modify `src/utils/twitter/twitteractivity_feed.rs`:
   - Remove the `if y >= (viewport_height * 0.9) { continue; }` block from `filter_candidates`.
   - Remove or update tests that assert the 90% viewport exclusion:
     - Remove `filter_as_y_approaches_viewport_edge`
     - Remove `filter_y_at_exact_viewport_threshold_is_excluded`
     - Remove `filter_y_just_below_threshold_is_included`
     - Update `filter_mixed_valid_and_invalid_tweets` to either remove the out-of-viewport test case or expect it to be included.

## API Changes
No API changes.

## Validation
- Run unit/library tests: `cargo test --lib`
- Run checks: `powershell -ExecutionPolicy Bypass -File .\check-fast.ps1`
- Run verification suite: `powershell -ExecutionPolicy Bypass -File .\check.ps1`
