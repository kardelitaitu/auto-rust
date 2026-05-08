# DSL Task Guide

Use this guide when you change task parsing, validation, execution, or DSL task authoring.

## What It Covers

- Task definition loading
- YAML and TOML parsing
- Action execution
- Variables and condition handling
- Control flow like `if`, `loop`, `foreach`, `while`, `retry`, and `parallel`
- Validation before execution

## Core Rule

Keep DSL behavior predictable.

- Parse once.
- Validate before execution.
- Execute with the smallest safe scope.
- Prefer shared helpers over task-specific ad hoc logic.

## Recommended Reading Order

1. [docs/TASKS/overview.md](overview.md)
2. [docs/API_REFERENCE.md](../API_REFERENCE.md)
3. `src/task/dsl/parser.rs`
4. `src/task/dsl/executor.rs`
5. `src/task/dsl/control_flow.rs`
6. `src/task/dsl/evaluator.rs`

## When to Use This Doc

- Adding a new DSL action
- Changing how task payloads are resolved
- Editing validation rules
- Touching control flow or retry behavior
- Modifying task execution reports or debug flow

## Common Risks

- Parser and executor drift
- Validation accepting an action the executor cannot run
- Scope leaks across nested task calls
- Confusing defaults for control flow or retries

## Best Practice

Update the docs and tests together when DSL behavior changes.

