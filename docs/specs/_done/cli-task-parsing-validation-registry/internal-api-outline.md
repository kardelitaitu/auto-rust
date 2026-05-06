# Internal API Outline

## Ownership Boundaries

- `src/cli/parser.rs`
  - Own task-group parsing, browser filter parsing, and task-group formatting.
  - Keep this module free of task execution logic.

- `src/cli.rs`
  - Keep the public CLI surface small.
  - Re-export parser helpers if needed for compatibility.

- `src/main.rs`
  - Own startup mode dispatch and task-help command handling.
  - Keep output formatting deterministic.

- `src/task/mod.rs`
  - Keep task-name normalization and built-in task knowledge.

- `src/task/registry.rs`
  - Own registry metadata, list output, and task discovery semantics.

- `src/validation/task.rs`
  - Own task payload validation rules.

- `src/validation/task_registry.rs`
  - Own registry-backed validation and conflict handling.

## API Shape Rules

- Preserve current task-group syntax.
- Keep task name normalization consistent everywhere.
- Do not duplicate registry knowledge inside CLI parsing.
- If `--help-task` needs task metadata, source it from the registry rather than hard-coding it in the CLI.
