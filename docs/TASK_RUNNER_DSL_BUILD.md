# Task Runner DSL Build Plan

> Companion to: `../TASK_RUNNER_PROPOSAL.md`

This document describes the **future DSL build** once the registry foundation is complete.
It should be read as a separate implementation plan, not as part of the current preparation work.

---

## 1. Purpose

The long-term goal is to let simple automation be expressed in a small, readable task format while keeping Rust tasks for complex logic.

The DSL should be introduced only after the registry foundation is stable.

### DSL goals

- make simple linear tasks easier to author
- preserve the same task-running workflow
- keep Rust tasks for advanced logic
- reuse the same task registry, validation, and policy model
- avoid changing the execution architecture a second time

---

## 2. Proposed DSL Shape

The DSL should stay small and line-based.

### Example

```task
name: strawberry_search
duration: 10-120s

navigate https://google.com
click "textarea[name='q']"
type "textarea[name='q']" "strawberry" --humanize
wait_for_load 5000
screenshot
end
```

### Design rules

- one action per line
- comments start with `#`
- quoted strings for text with spaces
- minimal flags only
- predictable parsing
- easy error messages with line numbers

---

## 3. DSL Scope

### Phase 2A — Grammar and Parser

Define the smallest useful grammar.

#### Deliverables

- header parsing
- action parsing
- quoted string handling
- comment handling
- error reporting with line numbers

#### Acceptance criteria

- the parser rejects malformed input clearly
- valid tasks parse into a stable AST

---

### Phase 2B — Executor Bridge

Map DSL actions to the existing `TaskContext` API.

#### Deliverables

- executor layer
- action-to-API mapping
- timeout handling per action or per task
- cancellation support

#### Acceptance criteria

- every supported action calls the real task API
- action failures are reported clearly
- long-running actions cannot block indefinitely

---

### Phase 2C — Policy and Validation

Connect DSL tasks to the existing safety model.

#### Deliverables

- DSL task policy metadata
- permission checks
- duration limits
- validation before execution

#### Acceptance criteria

- DSL tasks obey the same safety boundaries as Rust tasks
- unsupported actions are rejected

---

### Phase 2D — Tooling and Inspection

Add usability features once execution is stable.

#### Deliverables

- dry-run mode
- task linting
- task listing
- better error previews
- example tasks

#### Acceptance criteria

- users can validate a DSL task before running it
- diagnostics are easy to understand

---

## 4. Action Set for the First DSL Version

Keep the first version intentionally small.

### Recommended actions

- `navigate`
- `click`
- `type`
- `pause`
- `wait_for_load`
- `wait_for_visible`
- `scroll_to`
- `screenshot`
- `end`

### Deferred actions

- loops
- conditionals
- variables
- includes
- custom plugins
- macros
- task composition

These belong in later proposals, not in the first DSL release.

---

## 5. File and Module Plan

A future DSL implementation would likely need:

- `src/task/dsl/mod.rs`
- `src/task/dsl/ast.rs`
- `src/task/dsl/parser.rs`
- `src/task/dsl/executor.rs`
- `src/task/dsl/error.rs`

The exact structure can change, but the split should stay clear:

- parse the file
- validate the AST
- execute through `TaskContext`
- reuse the registry for task metadata and policy

---

## 6. DSL Safety Model

The DSL must not bypass the project’s existing task safety rules.

### Required safeguards

- task duration limits
- policy checks
- explicit permission model
- cancellation-aware execution
- deterministic error reporting
- tests for invalid selectors / invalid actions / timeout behavior

### Important rule

If the DSL cannot express a task safely and clearly, the task should stay in Rust.

---

## 7. Testing Plan

### Parser tests

- valid task parses successfully
- invalid header fails clearly
- malformed quotes fail clearly
- unknown action fails clearly

### Executor tests

- each action maps to the correct `TaskContext` call
- timeout behavior is enforced
- cancellation works

### Policy tests

- permissions are checked before use
- unsupported actions are rejected
- task duration bounds are enforced

### Integration tests

- DSL task can be parsed and executed in a real browser session
- DSL task and Rust task still coexist cleanly

---

## 8. Definition of Done

The DSL build is complete when:

- a small line-based task can run end-to-end
- registry, policy, and validation are reused
- Rust tasks still work unchanged
- dry-run and diagnostics are available
- the DSL remains intentionally small and easy to maintain

---

## 9. Relationship to the Preparation Plan

This document depends on the preparation work.

Do **not** start DSL build until the following are true:

- registry foundation exists
- validation uses the registry
- policy lookup is registry-aware
- task inspection works
- conflict rules are defined

Only then does it make sense to add a DSL parser and executor.
