## Validation Checklist

- [ ] `run()` rejects invalid payloads with a readable validation error.
- [ ] `run()` stops at the configured timeout and does not leak past the task duration.
- [ ] `TaskConfig::from_payload()` keeps duration, candidate count, thread depth, action limits, and feature flags stable.
- [ ] Persona weights still merge payload overrides with behavior profile input.
- [ ] Phase 1 navigation still calls the expected helper entry point.
- [ ] Feed scanning still honors the scan interval and empty-scan stop rules.
- [ ] Candidate processing still stops on limit exhaustion or break conditions.
- [ ] `build_summary_lines()` still emits the same summary keys and remaining-limit keys.
- [ ] `tests/twitteractivity_integration.rs` stays green.
- [ ] `./check-fast.ps1` passes.
- [ ] `./check.ps1` passes.
- [ ] `spec-lint.ps1` passes for this package.
