# Validation Checklist

- [ ] `cargo check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Verify `engage_replies` handles `dry_run_actions` correctly.
- [ ] Verify `process_candidate` processes multiple actions per tweet (e.g. logs show "Liked" and "Replied" on the same tweet ID).
- [ ] Verify that a `like` action executed *after* a thread dive uses the correct method (`like_tweet(api)`).
- [ ] `.\check-fast.ps1` passes.
- [ ] `.\spec-lint.ps1` passes.

# CI Commands

```powershell
.\check-fast.ps1
.\spec-lint.ps1
```

# Quality Rules

- Ensure the bot pauses appropriately between multiple actions on the same tweet.
- Verify that `actions_this_scan` budget checks are enforced *during* the multi-action loop, breaking early if the budget is exhausted mid-tweet.
