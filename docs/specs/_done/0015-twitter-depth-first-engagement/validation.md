# Validation Checklist

- [ ] `cargo check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Verify `EngagementCounters` correctly track likes on replies.
- [ ] Verify `process_candidate` doesn't exit early if replies are found.
- [ ] Verify that global limits (e.g. `max_likes`) are respected even when engaging with multiple items per thread.
- [ ] Ensure no recursive dives (bot should never "dive" into a reply of a reply).

# CI Commands

Run these from the repo root:

```powershell
.\check-fast.ps1
```

Targeted test for counters and limits:
```powershell
cargo test --lib utils::twitter::twitteractivity_limits::tests
```

# Quality Rules

- **Human-like Timing**: Ensure pauses between reply engagements are randomized and realistic (1-3s).
- **Limit Respect**: Sub-engagements MUST NOT bypass the session engagement limits.
- **Safety First**: If any error occurs during reply scanning, the bot should immediately proceed to `goto_home` to avoid getting stuck in a loop.

