last audited 26-06-26 by antigravity

## Acceptance Criteria

1. **Tweet Centering**: Feed-level actions (like, retweet, follow, bookmark) scroll the target tweet element to the center of the viewport before acting.
2. **Dynamic Position Resolution**: Action coordinates are resolved immediately post-scroll to avoid layout drift.
3. **Retweet Failsafe**: `retweet_at_position` falls back to `RETWEET_CONFIRM_SELECTOR` direct click if coordinate-based popup confirmation fails.
4. **Test Coverage**: Unit tests are added or updated to cover the new selector and action logic.
5. **CI Health**: All checks (SpecLint, Clippy, Format, tests) pass cleanly via `.\check.ps1`.

## Verification Steps

### Automated Verification
Run the compiler, clippy, formatting, and unit tests:
```powershell
.\check-fast.ps1
```

Run full integration and unit test suite:
```powershell
.\check.ps1
```

### Manual Verification
Verify that the `js_scroll_and_get_tweet_button` script resolves correctly by adding a unit test verifying replacements:
- Test that `js_scroll_and_get_tweet_button()` returns a string containing `{TWEET_ID}` and `{BUTTON_NAME}` before substitution.
- Test that replacement works.
