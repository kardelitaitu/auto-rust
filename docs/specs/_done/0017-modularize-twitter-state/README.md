# Modularize Twitter Activity State

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Extract 8 type definitions from the 1226-line `src/utils/twitter/twitteractivity_state.rs` into focused submodules under `src/utils/twitter/state/`. This is the second-largest file in the codebase (after executor.rs) and the last remaining monolith in the Twitter utility module.

## Scope

Extract into submodules without behavioral changes:
- `types.rs` — `TaskValidationError`, `SentimentTemplates`, `CandidateContext`, `CandidateResult`
- `session.rs` — `SessionState`, `RateLimitBackoff`
- `tracking.rs` — `TweetActionTracker`
- `config.rs` — `TaskConfig` + `from_payload()`

## Next Steps

1. Implementer reads `baseline.md` and `plan.md`
2. Extract each type group into its target submodule
3. Verify `cargo check && cargo test --lib`
4. Run `cargo clippy --all-targets --all-features`
5. Archive spec to `_done/`
