# Internal API Outline

## Ownership Boundaries

- `src/runtime/shutdown.rs`
  - Own shutdown signal wiring and shutdown subscription behavior.
  - Keep the shutdown manager the single place that translates external shutdown events into runtime control flow.

- `src/runtime/execution.rs`
  - Own active group execution, cancellation token wiring, and shutdown waiting behavior.
  - Stop new work when shutdown has begun, then wait for active work to unwind.

- `src/orchestrator.rs`
  - Own session state transitions around work start, cleanup, and failure classification.
  - Keep cancellation handling explicit.

- `src/session/mod.rs`
  - Own session state machine invariants and release semantics.
  - Never leave a session stuck in `Busy` after a canceled attempt.

- `src/utils/navigation.rs`
  - Accept cancellation-aware waits where long selector waits must be interruptible.

## API Shape Rules

- Do not add a new public surface unless a test proves the existing surface cannot express the shutdown contract.
- Prefer small internal helpers over broad refactors.
- Prefer explicit cancellation state over string matching or inferred failure classification.
