# Plan

## Step 1: Baseline and contract

- Confirm the current CLI parser, startup modes, and task validation flow.
- Re-read the tests already present in `src/cli.rs`, `src/main.rs`, `src/task/registry.rs`, and `src/validation/*`.
- Lock the promise that current task-group syntax does not change.

## Step 2: Parser boundary

- Move parser-focused helpers into a dedicated `src/cli/parser.rs` module.
- Keep task-group parsing and task-group formatting behavior stable.
- Keep browser filter normalization stable.

## Step 3: Registry-backed validation and help

- Route CLI validation through the task registry and task validation modules.
- Add `--help-task` handling with task-specific payload guidance.
- Keep the output deterministic and easy to test.

## Step 4: Tests and regression checks

- Add or update unit tests for parser extraction and `--help-task` behavior.
- Keep existing list, dry-run, validate, and watch behavior tests passing.
- Add only narrow smoke tests if they are needed for command dispatch.

## Step 5: Verification

- Run `.
check-fast.ps1` during implementation.
- Run `.
check.ps1` before handoff.
- Move the spec to `_done/` only after the full gate passes.

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

# Decisions

- Keep task-group parsing syntax unchanged.
- Move parser helpers behind a dedicated CLI parser module.
- Use the registry as the source of truth for `--help-task` output.
- Keep startup-mode dispatch in `main.rs`.
- Prefer deterministic task help output over verbose dynamic text.

