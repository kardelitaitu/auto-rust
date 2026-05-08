# Self-Healing Modularization

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary

The `src/adaptive/self_healing.rs` file represents a severe violation of the Single Responsibility Principle. At over 900 lines of code, it defines 34 distinct structs and enums that encompass an entire subsystem, including health monitoring, complex recovery strategies, failure history tracking, and active recovery state management. This spec proposes modularizing this monolithic file into a cohesive `self_healing/` directory with narrowly scoped domain modules (`health.rs`, `strategy.rs`, `history.rs`, `state.rs`, and `system.rs`).

## Scope

- In scope:
  - Creating a `src/adaptive/self_healing/` directory structure.
  - Moving health-related types (e.g., `SystemHealth`, `HealthMonitor`) to `health.rs`.
  - Moving strategy types (e.g., `RecoveryStrategies`, `ConnectionRecovery`, `ResourceScaling`) to `strategy.rs`.
  - Moving history types (e.g., `FailureHistory`, `FailureRecord`) to `history.rs`.
  - Moving state types (e.g., `RecoveryState`, `ActiveRecovery`) to `state.rs`.
  - Keeping the primary `SelfHealingSystem` orchestration in `system.rs`.
  - Setting up `src/adaptive/self_healing/mod.rs` to re-export the public API.
  - Updating all internal and external imports to reflect the new structure.
- Out of scope:
  - Changing the actual recovery logic or algorithms.
  - Introducing new health checks or recovery strategies.
  - Modifying other files in the `adaptive` module beyond import path updates.

## Files

- `spec.yaml`
- `plan.md`
- `validation.md`
- `notes.md`

## Rules

- Keep the spec short.
- Run `spec-lint.ps1` before handoff.
- Use `.\check-fast.ps1` while iterating.
- Use the archive helper `.\spec-archive.ps1` to move to `_done/`.

## Next Step

Wait for the implementer agent to extract the modules.
