# Add Average Recovery Time to FailurePattern

## Baseline
The `FailurePattern` struct in `src/adaptive/self_healing/history.rs` (lines 37-42) captures failure signatures, frequency, and impact, but lacks a field for average recovery time. The `FailureRecord` struct already has a `recovery_time: Duration` field, and `FailureHistory` maintains `mttr` (mean time to recovery) globally. Adding a per-pattern average recovery time would allow identifying which specific failure patterns take longest to recover from.

## Implementation Steps
1. Open `src/adaptive/self_healing/history.rs`.
2. Add a `pub average_recovery_time: Duration` field to the `FailurePattern` struct, after the `impact` field.
3. Update the test `failure_pattern_construction` to include the new field and assert its value.
4. Update the test `failure_pattern_empty_signature` to include the new field (set to `Duration::ZERO` or similar default).
5. Update the test `failure_pattern_is_cloneable` to include the new field in both the original and clone assertions.
6. Compile and run the test suite to confirm all tests pass.

## API Changes
- `FailurePattern` gains one public field: `average_recovery_time: Duration`.
- This is an additive change. Existing code that constructs `FailurePattern` without the field will fail to compile until updated.

## Validation
- Run `cargo test -p <crate-name> history` (or `cargo test` for the whole project).
- Confirm all existing tests pass and the updated tests include the new field.
- Verify `cargo check` produces no warnings.

## Design Decisions and Risks
- **Trade-off**: The field is added as a plain `Duration` rather than `Option<Duration>`. This means patterns must always have a value, even when no records exist yet (use `Duration::ZERO`). An alternative would be `Option<Duration>` to distinguish "no data" from "zero recovery time", but that adds complexity.
- **Risk**: This is a compile-time breaking change for any external code constructing `FailurePattern` directly. Since the struct is `pub`, downstream code will need updating.
- **Scope**: The change does not include logic for computing the average from matching records. That is left for a separate improvement to keep this change minimal and mechanical.

Confidence: High