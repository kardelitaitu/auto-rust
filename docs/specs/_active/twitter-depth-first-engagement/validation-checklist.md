# Validation Checklist

- [ ] `cargo check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Verify `EngagementCounters` correctly track likes on replies.
- [ ] Verify `process_candidate` doesn't exit early if replies are found.
- [ ] Verify that global limits (e.g. `max_likes`) are respected even when engaging with multiple items per thread.
- [ ] Ensure no recursive dives (bot should never "dive" into a reply of a reply).
