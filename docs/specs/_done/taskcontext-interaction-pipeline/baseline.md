# Baseline

## Current State

- `TaskContext` already exposes separate public verbs for click, nativeclick, focus, type, keyboard, select_all, clear, and click_and_wait.
- Click currently has its own timing, retry, fallback, and learning flow.
- Type currently has its own existence check, focus step, text entry, and verification step.
- `select_all` and `clear` already perform local validation and keyboard interaction.
- Existing integration tests cover many task API behaviors, but the interaction flow is still spread across several methods.

## Known Gaps

- Shared preflight and postflight logic are duplicated across action methods.
- Click and type do not yet share a single internal pipeline contract.
- Verification rules are implemented per method instead of being shaped by one shared result model.
- Reliability drift is possible when a change lands in one verb but not the others.

## Why This Matters

The public API is already stable, but the implementation is harder to reason about than it needs to be. A shared pipeline should make click and type behavior more reliable without changing task code.
