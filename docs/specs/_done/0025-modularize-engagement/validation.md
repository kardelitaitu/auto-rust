## Acceptance Criteria

- [ ] `scoring.rs` exists with `handle_engagement_decision()` + `modulate_persona_by_sentiment()` (≤150 lines)
- [ ] `dispatch.rs` exists with `dispatch_action()` extracted from process_candidate() match block (≤250 lines)
- [ ] `mod.rs` exists with `process_candidate()` calling `dispatch::dispatch_action()` + `engage_replies()` + re-exports (≤350 lines)
- [ ] `tests.rs` exists with all 5 test modules: integration, decision, statistical, property, gap (≤600 lines)
- [ ] Original `twitteractivity_engagement.rs` deleted
- [ ] All dependent files updated with new `crate::utils::twitter::engagement` import path
- [ ] No compiler warnings from `cargo check --lib`
- [ ] No behavior changes — all existing tests pass unchanged
- [ ] No clippy warnings — `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `spec-lint.ps1` passes with the new spec in `_active/`

## Test Commands

- `cargo check --lib`
- `cargo test --lib engagement` (or the module path equivalent)
- `powershell -File check.ps1`
- `powershell -File spec-lint.ps1`

## Visual Inspection

- Confirm `src/utils/twitter/engagement/` directory contains exactly 4 files: `mod.rs`, `scoring.rs`, `dispatch.rs`, `tests.rs`
- Confirm `src/utils/twitter/twitteractivity_engagement.rs` no longer exists
- Confirm no remaining `use ... twitteractivity_engagement` imports in dependent files
- Confirm `dispatch_action()` signature matches the documented contract and is called from `process_candidate()`
