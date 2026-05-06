# CLI Task Parsing + Validation + Registry

Status: `done`

Owner: `spec-agent`
Implementer: `code-implementer`

## Summary

Make CLI task handling more self-contained by separating parser logic from command dispatch, tying validation to the task registry, and adding a task-specific help command so users can inspect expected payload shape without reading source.

## Scope

- In scope:
  - parser extraction for task groups and browser filter parsing
  - registry-backed validation during CLI parsing
  - a task-specific help command for payload guidance
  - keeping task group formatting and current CLI behavior stable
- Out of scope:
  - browser discovery/session assembly refactors
  - runtime shutdown work
  - task execution pipeline changes
  - public task API redesign

## Files

- `spec.yaml`
- `baseline.md`
- `internal-api-outline.md`
- `plan.md`
- `validation-checklist.md`
- `ci-commands.md`
- `decisions.md`
- `quality-rules.md`
- `implementation-notes.md`

## Completed

- Created `src/cli/parser.rs` module with dedicated task group and browser filter parsing
- Added `TaskValidationInfo` struct and `get_task_validation_info()` for registry-backed help
- Added `--help-task` CLI flag with task-specific payload guidance
- Updated startup modes to handle HelpTask variant
- All 2265 tests pass (added 22 new tests)
