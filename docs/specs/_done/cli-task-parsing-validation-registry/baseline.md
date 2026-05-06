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
