# Validation Checklist

- Missing `duration_ms` resolves from the task config, not a hardcoded `300_000` fallback.
- `scroll_count` changes the runtime scroll budget in a visible way.
- Wrong-type numeric payload values return validation errors.
- Docs and examples for TwitterActivity match the runtime contract.
- Regression tests cover default, explicit, and malformed payload cases.
- `.\check-fast.ps1` still works during iteration.
- `.\check.ps1` still passes before handoff.
- `spec-lint.ps1` stays green for the package.

# CI Commands

## Focused

- `.\check-fast.ps1 -Paths src\task\twitteractivity.rs src\utils\twitter\twitteractivity_state.rs src\utils\twitter\twitteractivity_feed.rs tests\twitteractivity_integration.rs docs\TASKS\twitteractivity.md README.md config\default.toml`
- `cargo test --test twitteractivity_integration`
- `cargo test twitteractivity`

## Full

- `.\check.ps1`

# Quality Rules

- The task entry point must not contain hidden fallback logic that contradicts the contract.
- Payload validation must be explicit; wrong types are errors, not silent defaults.
- The runtime and docs should move together, not drift independently.
- Changes should stay inside the TwitterActivity task boundary unless a shared helper is clearly required.
- Tests should prove the contract, not just the happy path.

