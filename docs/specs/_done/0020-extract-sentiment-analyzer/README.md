# 0020-extract-sentiment-analyzer

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Extract the 1,800-line `src/utils/twitter/sentiment/analyzer.rs` into focused submodules: types, strategies, analyzer core, and helpers. The sentiment directory already has a `strategies/` subdirectory, but the main analyzer file still holds 10 data types, 4 strategy structs, the analyzer itself, and 5 helper functions — all mixed together.

Also fixes 2 TODO stubs for `author_prestige()` and `tweet_recency()` extraction functions.

## Scope

- `src/utils/twitter/sentiment/` directory only
- Extraction targets: `types.rs`, `analyzer_core.rs`, `helpers.rs`
- Existing `strategies/` directory extended with extracted strategy structs
- Existing `utils.rs` and `mod.rs` unchanged

## Next Steps

1. Review spec package
2. Implement extraction preserving strategy trait coherence
3. Fix 2 TODO stubs
4. Verify: cargo check, cargo test --lib sentiment, cargo clippy
