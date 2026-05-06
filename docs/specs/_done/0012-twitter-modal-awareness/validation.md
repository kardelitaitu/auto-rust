# Validation Checklist

- [ ] `cargo check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo nextest run --lib utils::twitter::twitteractivity_selectors` (Verify JS generation)
- [ ] `cargo nextest run --lib utils::twitter::twitteractivity_interact` (Verify context logic)
- [ ] Verify `is_on_tweet_page` handles both URL and Modal cases correctly.
- [ ] Verify `js_root_tweet_button_center` finds buttons inside a modal when one exists.

# CI Commands

Run these from the repo root:

```powershell
.\check-fast.ps1
```

Targeted test for selectors:
```powershell
cargo test --lib utils::twitter::twitteractivity_selectors::tests
```

# Quality Rules

- The JS injected into the page must be robust against missing elements (use optional chaining or explicit checks).
- Performance: The visibility and presence checks in the DOM must be efficient to avoid lagging the browser.
- No regression: Ensure the home feed navigation still works correctly and doesn't accidentally think it's on a tweet page when just a small unrelated popup appears.

