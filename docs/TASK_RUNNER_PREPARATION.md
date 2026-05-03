# Task Runner Preparation Plan

> Companion to: `../TASK_RUNNER_PROPOSAL.md`

This document prepares the codebase for the future task-runner evolution **without implementing DSL yet**.
It focuses on the registry foundation, task discovery, validation, and policy alignment that must exist before any new task format is added.

---

## 1. Purpose

The current task system works, but task identity is spread across multiple modules:

- task dispatch lives in `src/task/mod.rs`
- task validation lives in `src/validation/task_registry.rs`
- policy lookup lives in `src/task/policy.rs`
- CLI parsing lives in `src/cli.rs`
- execution wiring lives in `src/main.rs`

The preparation phase makes those pieces speak the same language.

### Outcome we want

- one authoritative task registry
- task descriptors instead of raw string lookups
- validation and execution using the same resolver
- clearer task listing and diagnostics
- a safe base for future DSL work

---

## 2. What This Phase Is Not

This phase must **not** introduce:

- DSL parsing
- DSL execution
- task format migration
- plugins or marketplace support
- loops, variables, or conditionals
- breaking changes to current CLI usage

The foundation must stay Rust-task compatible.

---

## 3. Preparation Goals

### Functional goals

1. Preserve every current Rust task.
2. Remove duplicated task-name logic.
3. Make task discovery explicit.
4. Make policy lookup registry-aware.
5. Add a clean inspection path such as `--list-tasks`.
6. Make validation and execution agree on task identity.

### Technical goals

1. Introduce a `TaskDescriptor` model.
2. Introduce a `TaskSource` model.
3. Stop relying on hardcoded relative-path checks.
4. Make future discovery roots configurable.
5. Keep the codebase easy to extend without changing the dispatcher again.

### Registry model

The registry should describe each task as a small piece of metadata instead of a raw string. At minimum, the registry needs to answer:

- what the task name is
- where the task comes from
- which policy applies
- whether the task is built in, configured, or unknown

A practical first-pass shape is:

- `TaskDescriptor { name, source, policy_name }`
- `TaskSource::BuiltInRust` for compiled-in tasks
- `TaskSource::ConfiguredPath(PathBuf)` for future external task roots
- `TaskSource::Unknown` for clean error reporting

Built-in tasks should remain pathless in the descriptor; paths only matter for configured sources.

---

## 4. Recommended Phase Breakdown

### Phase 1A — Registry Types

Create the core registry types and use them to describe built-in tasks.

#### Deliverables

- `TaskDescriptor`
- `TaskSource`
- registry lookup API
- built-in task metadata mapping

#### Acceptance criteria

- every built-in task can be described by the registry
- built-in execution behavior is unchanged
- tests still pass

---

### Phase 1B — Validation Alignment

Route validation through the registry instead of separate ad hoc checks.

#### Deliverables

- registry-backed task name validation
- registry-backed task existence checks
- clearer error messages for unknown tasks
- deterministic handling of unknown vs known tasks

#### Acceptance criteria

- validation and execution resolve the same task identity
- no current task command breaks
- relative-path assumptions are removed where possible

---

### Phase 1C — Policy Alignment

Make policy lookup come from the same task metadata used elsewhere.

#### Deliverables

- registry-aware policy lookup hooks
- built-in policy mapping preserved
- no duplicate task-name matching in multiple places

#### Acceptance criteria

- policies resolve the same way everywhere
- tests prove policy/task mapping stays correct

---

### Phase 1D — Inspection and Listing

Add a non-invasive way to inspect registered tasks.

#### Deliverables

- `--list-tasks`
- task name
- source
- path, if applicable
- policy name
- known/unknown state

#### Example

```bash
cargo run --list-tasks
```

```text
✓ cookiebot       BuiltInRust                     policy=cookiebot
✓ pageview        BuiltInRust                     policy=pageview
```

#### Acceptance criteria

- listing does not change execution behavior
- output is stable and testable

---

## 5. File-Level Start Plan

### `src/task/mod.rs`

- keep current dispatch for now
- add registry integration for task metadata and listing
- avoid changing execution flow in the first pass

### `src/validation/task_registry.rs`

- replace hardcoded checks with registry lookups
- remove direct dependence on `Path::new("src/task")`
- keep validation semantics compatible

### `src/task/policy.rs`

- keep current policies
- add registry-aware hooks for policy resolution
- preserve current timeout behavior

### `src/main.rs`

- consume registry-aware validation results
- keep the run loop behavior intact
- surface clearer diagnostics when possible

### `src/cli.rs`

- add inspection flags later in the phase
- keep positional task parsing unchanged

---

## 6. Testing Plan

### Must-have tests

- registry resolves known built-in tasks
- unknown tasks are reported consistently
- validation and execution agree
- policy lookup is consistent with registration
- task listing output is correct

### Current coverage note

- `TaskDiscoveryConfig` defaults are covered both directly and through `load_config()`
- `.env` overrides for `task_discovery` are covered through the config load path
- task list ordering is covered so `--list-tasks` stays stable
- startup flag precedence is covered through `select_startup_mode`

### Regression checks

- `cargo check`
- existing task unit tests
- validation tests
- task registry tests

### Suggested test naming

- `task_registry_tests`
- `task_validation_registry_tests`
- `task_policy_registry_tests`
- `task_listing_tests`

---

## 7. Definition of Done

This preparation document is complete when:

- the registry is the source of truth for built-in task metadata
- validation and policy lookup use the same task identity model
- existing task commands still work
- the codebase can clearly describe tasks without changing execution behavior

---

## 8. Suggested Work Order

1. Add registry types.
2. Populate built-in task descriptors.
3. Route validation through the registry.
4. Route policy lookup through the registry.
5. Add task listing.
6. Add config hooks for future discovery roots.
7. Write docs and tests.

---

## 9. Relationship to the DSL Goal

This document is intentionally **DSL-free**.

The long-term DSL goal stays in the main proposal, but this preparation phase is about making the foundation solid enough that a DSL proposal can be added later without reworking the task system again.
