# Plan

## Step 1: Create Directory Structure

- Create `src/adaptive/self_healing/` directory.
- Create empty files: `mod.rs`, `health.rs`, `strategy.rs`, `history.rs`, `state.rs`, `system.rs`.

## Step 2: Extract Health Domain

- Move `HealthMonitor`, `SystemHealth`, `HealthCheckResult`, `HealthCheckType`, `HealthCheckStatus` to `health.rs`.
- Include any associated implementations and `#[cfg(test)]` blocks for these specific types.

## Step 3: Extract Strategy Domain

- Move `RecoveryStrategies`, `ConnectionRecovery`, `ResourceRecovery`, `ResourceScaling`, `ResourceCleanup`, `PerformanceRecovery`, `PerformanceTuning`, `ErrorRecovery`, `ErrorClassification`, `ErrorCategory`, `ErrorSeverity`, `ErrorProcedure`, `RecoveryStep`, `RecoveryConditions`, `RecoveryActionType`, `RecoveryResult` to `strategy.rs`.
- Include their implementations.

## Step 4: Extract History Domain

- Move `FailureHistory`, `FailureRecord`, `FailureType`, `FailurePattern`, `ImpactLevel` to `history.rs`.
- Include their implementations.

## Step 5: Extract State Domain

- Move `RecoveryState`, `RecoveryMode`, `ActiveRecovery`, `RecoveryType`, `RecoveryStatus`, `RecoveryProgress` to `state.rs`.
- Include their implementations.

## Step 6: Extract Core System

- Move `SelfHealingSystem` to `system.rs`.
- Update it to import from the newly created sibling modules.

## Step 7: Wire Up Modules

- In `src/adaptive/self_healing/mod.rs`, declare `pub mod health; pub mod strategy; pub mod history; pub mod state; pub mod system;`.
- Re-export the primary types so the external API remains largely unchanged: `pub use system::SelfHealingSystem; pub use health::HealthMonitor; ...`
- Delete `src/adaptive/self_healing.rs` and update `src/adaptive/mod.rs` to point to the new directory (`pub mod self_healing;`).

## Step 8: Verification

- Run `cargo check` and fix any import path issues in the rest of the codebase (e.g., in other `adaptive` modules or `orchestrator`).
- Run `cargo test` to ensure all functionality remains identical.

# Internal API Outline

- The entire `self_healing` module is internal to the application, so the main goal is just internal organization.
- `SelfHealingSystem` will remain the primary entry point for consumers.

# Decisions

- Module split by domain concept: Grouping types by their lifecycle purpose (health sensing, strategy definition, state tracking) creates strong cohesion and makes the system much easier to maintain.
