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
