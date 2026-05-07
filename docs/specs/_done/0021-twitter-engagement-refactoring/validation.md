# Validation Checklist

## Pre-Implementation Checks
- [ ] Run `cargo test` - establish baseline (ALL tests must pass)
- [ ] Run `cargo check` - verify clean compilation
- [ ] Count lines: `process_candidate` = 762 lines (lines 95-857)
- [ ] Count total file: `twitteractivity_engagement.rs` = 1,325 lines

## During Implementation (After Each Extraction)

### After Extracting `engage_replies` (Step 1)
- [ ] `cargo check` passes
- [ ] `process_candidate` reduced by ~120 lines
- [ ] Depth-first engagement still works (if integration tests available)
- [ ] Function signature correct: takes api, persona, task_config, counters, actions_this_scan

### After Extracting `execute_engagement_action` (Step 2)
- [ ] `cargo check` passes
- [ ] `process_candidate` reduced by ~350 lines
- [ ] All action types work: like, retweet, quote, reply, follow, bookmark
- [ ] Return value (bool) correctly indicates success

### After Extracting `modulate_persona_by_sentiment` (Step 3)
- [ ] `cargo check` passes
- [ ] `process_candidate` reduced by ~50 lines
- [ ] Sentiment analysis still works
- [ ] Persona modulation applies correctly

### After Simplifying Action Selection (Step 4)
- [ ] `cargo check` passes
- [ ] `process_candidate` now ~200-300 lines
- [ ] Action selection logic preserved

## Post-Implementation Verification

### Line Count Verification
- [ ] `process_candidate` reduced from 762 to ~200-300 lines
- [ ] `twitteractivity_engagement.rs` total ~1,400 lines (slight increase due to function signatures)
- [ ] No new files created

### Functional Verification
- [ ] Run `cargo test` - ALL tests pass
- [ ] Run `cargo test --lib twitteractivity_engagement` - engagement tests pass
- [ ] Run `.\check.ps1` - FULL CI GATE PASSES

### Code Quality
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt` applied
- [ ] Each helper function has single responsibility
- [ ] Function signatures are clean (no 10-parameter functions)

## Behavioral Verification

### process_candidate Flow
- [ ] Sentiment analysis runs correctly
- [ ] Smart decision check works
- [ ] Action selection respects limits
- [ ] Thread diving works when needed
- [ ] Each action type executes correctly (like/retweet/quote/reply/follow/bookmark)
- [ ] Depth-first reply engagement works
- [ ] Pause between actions works
- [ ] Counter increments correctly

# CI Commands

```bash
# Full validation gate (MUST PASS before commit)
cd "C:\My Script\auto-rust"
.\check.ps1

# Individual checks
cd "C:\My Script\auto-rust"
cargo check
cargo test --lib twitteractivity_engagement
cargo clippy -- -D warnings
cargo fmt --all -- --check

# Line count verification (process_candidate should be ~200-300 lines)
Select-String -Path "src/utils/twitter/twitteractivity_engagement.rs" -Pattern "^pub async fn process_candidate" | Select-Object LineNumber
# Then manually check where the function ends (look for next "pub fn" or "#[cfg(test)]")
```

# Quality Rules

1. **No logic changes**: Refactoring ONLY - behavior must be identical
2. **Keep in same file**: Do NOT create new files or directories
3. **Preserve tests**: All existing tests must pass without modification
4. **Document new functions**: Add rustdoc for extracted helper functions
5. **Incremental verification**: Run `cargo check` after EACH extraction
6. **No new dependencies**: Use existing types and functions


