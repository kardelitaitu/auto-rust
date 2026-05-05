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
