## Validation

- Post-dive next_candidate_scan is reset to prevent duplicate re-scan.
- Main loop sleep respects session deadline.
- Non-like actions are not gated behind should_dive().
- PersonaStrategy multiplier is consistent across all engagement levels.
- actions_taken is removed from call chain and CandidateResult.
- Cookie banner uses only standard CSS selectors.
- select_entry_point is seeded from TaskConfig.seed.
- 300s dive pause replaced with actual dive duration.
- Regex in llm_validation compiled via Lazy static.
- All existing tests pass.
- `spec-lint.ps1`, `./check-fast.ps1`, and `./check.ps1` pass.
