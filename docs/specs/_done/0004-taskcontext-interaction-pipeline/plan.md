# Plan

## Step 1: Baseline and contract

- Confirm the current click and type method flow in TaskContext.
- Confirm the existing task API tests that already cover click, type, focus, select_all, clear, and click_and_wait.
- Lock the public API stability rule before any code changes.

## Step 2: Shared types

- Add the smallest set of shared interaction request/result types.
- Keep the types focused on action kind, selector, verification, and fallback data.
- Avoid moving unrelated TaskContext state into the new types.

## Step 3: Shared pipeline

- Add a new internal interaction pipeline module.
- Route click and type through the shared preflight and postflight stages.
- Reuse the pipeline where focus, select_all, and clear can share the same guard rails.

## Step 4: Reliability tests

- Add tests that prove click and type use the same shared flow.
- Add regression tests for selector verification and fallback handling.
- Keep browser-backed tests narrow and deterministic.

## Step 5: Verification

- Run `.
check-fast.ps1` during implementation.
- Run `.
check.ps1` before handoff.
- Move the spec to `_done/` only after the full gate passes.

# Internal API Outline

## Ownership Boundaries

- `src/runtime/task_context.rs`
  - Keep the public TaskContext verbs thin.
  - Delegate interaction orchestration to a shared internal pipeline.

- `src/runtime/task_context/interaction_pipeline.rs`
  - Own the shared preflight, execution, verification, fallback, and postflight flow.
  - Keep the action kinds small and explicit.

- `src/runtime/task_context/types.rs`
  - Own shared request and result types for interaction flow.
  - Add only the data that the pipeline needs to report or verify outcomes.

- `src/runtime/task_context/interaction.rs`
  - Keep low-level wrappers and narrow helpers that the pipeline can reuse.

- `src/capabilities/mouse.rs`
  - Keep mouse execution primitives focused on pointer movement and clicking.

- `src/capabilities/keyboard.rs`
  - Keep typing primitives focused on text entry and key handling.

## Suggested Type Shape

- `InteractionKind` enum for the small set of supported actions.
- `InteractionRequest` for selector, text, fallback, and verification settings.
- `InteractionResult` for the resolved outcome, verification status, and fallback usage.
- `InteractionPipeline` as the orchestration entry point for TaskContext verbs.

## API Shape Rules

- Preserve the current public TaskContext method names and signatures.
- Prefer one shared flow per action family instead of large per-method branching.
- Keep the pipeline explicit enough that tests can target a single stage.

# Decisions

- Keep the public TaskContext API stable and move only internal orchestration behind the pipeline.
- Use shared interaction types to carry action-specific verification and fallback data.
- Keep click and type as separate verbs, but let them share the same preflight and postflight stages.
- Prefer deterministic tests over timing-heavy interaction checks.

