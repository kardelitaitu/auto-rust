# Validation Checklist

- [ ] `cargo check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo nextest run --lib utils::twitter::twitteractivity_selectors` (Verify JS generation)
- [ ] `cargo nextest run --lib utils::twitter::twitteractivity_interact` (Verify context logic)
- [ ] Verify `is_on_tweet_page` handles both URL and Modal cases correctly.
- [ ] Verify `js_root_tweet_button_center` finds buttons inside a modal when one exists.
