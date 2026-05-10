## Validation

- `run()` rejects invalid payloads with a readable validation error.
- `run()` stops at the configured timeout and does not leak past the task duration.
- `TaskConfig::from_payload()` keeps duration, candidate count, thread depth, action limits, and feature flags stable.
- Persona weights still merge payload overrides with behavior profile input.
- `build_summary_lines()` still emits the same summary keys and remaining-limit keys.
- `tests/twitteractivity_integration.rs` stays green.
- `./check-fast.ps1`, `./check.ps1`, and `spec-lint.ps1` pass for this package.
