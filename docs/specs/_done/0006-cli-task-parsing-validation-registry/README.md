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

# Baseline

## Current State

- `src/cli.rs` contains task-group parsing, browser filter parsing, and task-group formatting in one module.
- `src/main.rs` owns startup mode selection and command dispatch for list, validate, dry-run, and watch flows.
- `src/task/mod.rs` holds task-name normalization and built-in task lookup.
- `src/task/registry.rs` owns task metadata, list output, and registry-backed validation behavior.
- `src/validation/task.rs` and `src/validation/task_registry.rs` already validate task names and payloads, but the CLI path still pulls those concerns through a split surface.

## Known Gaps

- Parser behavior is still embedded in `src/cli.rs` instead of a narrower parser module.
- CLI help for a specific task does not exist yet.
- Validation and registry semantics are spread across multiple layers, which makes the CLI path harder to extend.
- Some of the current behavior is well tested, but the ownership boundary is still not obvious.

## Why This Matters

This is the smallest user-facing improvement with a real ergonomics win. It makes CLI behavior easier to extend, easier to test, and easier for users to discover without changing how tasks execute.

