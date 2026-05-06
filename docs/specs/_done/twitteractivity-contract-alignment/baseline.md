# Baseline

- `src/task/twitteractivity.rs` uses a time-only loop and never reads `feed_scroll_count` from the task config.
- `TaskConfig::from_payload()` in `src/utils/twitter/twitteractivity_state.rs` hardcodes `duration_ms` to `300_000` when payload input is missing.
- `read_u64()` and `read_u32()` in `src/utils/twitter/twitteractivity_state.rs` silently fall back to defaults when a payload field has the wrong type.
- `src/utils/twitter/twitteractivity_feed.rs` already has a `scroll_feed(api, scroll_count, use_native_scroll)` helper that can serve as the lower-level scrolling primitive.
- `docs/TASKS/twitteractivity.md`, `config/default.toml`, and `README.md` already describe `duration_ms` and `scroll_count`, so the public contract exists but the runtime does not fully honor it.
- The current gap is contract drift, not missing task wiring in unrelated modules.
