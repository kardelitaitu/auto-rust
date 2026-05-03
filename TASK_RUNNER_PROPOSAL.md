# Task Runner Proposal: Registry-First Task System

**Status:** Draft  
**Date:** 2026-05-03  
**Author:** AI Assistant  
**Target Version:** v0.0.5

---

## 1. Executive Summary

The current task system is reliable, but it is still organized around a **hardcoded Rust-task registry**. That works for the existing codebase, but it makes task discovery, validation, policy lookup, and future task-source support harder than it should be.

This proposal **does not introduce DSL execution yet**. Instead, it establishes a **single task registry abstraction** that becomes the source of truth for:

- task discovery
- task metadata
- task validation
- policy lookup
- task listing / inspection
- future support for additional task sources

The goal is to make the task system **solid first**, so any later hybrid format work can be added without reworking the foundation.

### Goals

- Preserve all existing Rust task behavior
- Remove duplicated task-name logic across modules
- Make task discovery and validation use the same resolver
- Make task policy resolution explicit and testable
- Add registry-aware inspection commands like `--list-tasks`
- Prepare the codebase for future task-source expansion without committing to DSL now

### Long-term goal

The eventual end state is still a small, human-readable DSL for simple linear tasks. This proposal does **not** implement that DSL yet, but the registry foundation should make it possible later.

Example future task:

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

### Non-goals

- No DSL parser or DSL executor in this proposal
- No plugin marketplace
- No task marketplace/registry service
- No loops, variables, or conditionals
- No breaking changes to existing task invocation syntax

---

## 2. Why This Proposal Is Needed

The current task model works, but it is split across several places:

- `src/task/mod.rs` contains the dispatch match
- `src/validation/task_registry.rs` checks task name and file existence
- `src/task/policy.rs` owns policy lookup
- `src/main.rs` wires validation into execution
- `src/cli.rs` parses task groups and payloads

That means the codebase does **not** yet have a single authoritative model for a task.

A future hybrid system cannot be safe if task identity, task location, task policy, and task validation are all maintained independently. The foundation should come first.

---

## 3. Current State in the Codebase

This section summarizes the concrete findings from the repo.

### 3.1 Task dispatch is hardcoded

`src/task/mod.rs` resolves tasks using a `match` over task names. That is fine for today, but it means new task sources require code changes in the dispatcher.

### 3.2 Validation is split from execution
`src/validation/task_registry.rs` checks whether a task name is known and whether a task file exists, but it currently only knows about built-in task files under `src/task/`.

It also uses relative path checks (`Path::new("src/task")`), which are tied to the current working directory.

### 3.3 Policy lookup is name-based

`src/task/policy.rs` maps task names to static policies through `get_policy(task_name)`. Unknown tasks fall back to `DEFAULT_TASK_POLICY`.

That is workable for built-in Rust tasks, but it becomes fragile if task identity ever expands beyond compiled-in names.

### 3.4 CLI parsing is task-group focused, not registry focused

`src/cli.rs` parses task groups and payloads, but it does not know anything about sources, metadata, or precedence rules.

### 3.5 The task API already has a strong foundation

`src/runtime/task_context.rs` already exposes the verbs that matter for task execution:

- `navigate(url, timeout_ms)`
- `click(selector)`
- `r#type(selector, text)`
- `keyboard(selector, text)`
- `pause(ms)`
- `wait_for_load(timeout_ms)`
- `wait_for_visible(selector, timeout_ms)`
- `scroll_to(selector)`
- `screenshot()`

That means the execution layer is not the problem; the registry and metadata layer is.

### 3.6 Task authoring rules already exist

`TASK.md` already defines expectations around:

- task duration constants
- timeout wrappers
- policy ownership
- tests for duration bounds and payload defaults

Any future task-source strategy must respect those rules rather than bypass them.

---

## 4. Proposal Overview

This proposal introduces a **registry-first task system**.

Instead of treating task name resolution as an ad hoc lookup, the codebase should define a task registry that answers three questions consistently:

1. **What task is this?**
2. **Where does it come from?**
3. **What policy and validation rules apply?**

### Core idea

A task should be represented by a small descriptor, not just a string.

Suggested descriptor shape:

```rust
pub struct TaskDescriptor {
    pub name: String,
    pub source: TaskSource,
    pub path: Option<PathBuf>,
    pub is_known: bool,
    pub policy_name: &'static str,
}
```

And a source enum such as:

```rust
pub enum TaskSource {
    BuiltInRust,
    ConfiguredRustPath,
    FutureExternalFormat(String),
}
```

This is not a final API requirement, but it expresses the design clearly:

- built-in Rust tasks remain the default
- the registry can later grow to include configured task directories
- future non-Rust formats can be added without reworking dispatch again

---

## 5. Design Principles

### 5.1 Single source of truth

Task resolution, validation, policy lookup, and task listing should all use the same registry layer.

### 5.2 Backward compatibility first

Existing commands must continue to work exactly as they do now:

```bash
cargo run cookiebot
cargo run pageview=url=https://example.com
cargo run cookiebot then pageview=reddit.com
```

### 5.3 Explicit precedence rules

If more than one source can provide the same task name in the future, precedence must be explicit and test-covered.

The proposal should define that behavior before any external format is added.

### 5.4 Policy follows the task descriptor

Policy should be resolved from the same task metadata used for validation and execution, not from a separate ad hoc lookup.

### 5.5 Paths must not depend on the current working directory

The registry should resolve task paths from a known root or config, not from whatever directory the process happens to be in.

---

## 6. Multi-Phase Delivery Plan

This proposal is intentionally split into phases so the foundation can be delivered safely.

### Phase 1: Registry Foundation

**Goal:** create a single registry abstraction while keeping runtime behavior unchanged.

#### Deliverables

- Introduce a registry module or service
- Represent tasks as descriptors with source/path/policy metadata
- Route name validation through the registry
- Centralize task existence checks
- Make policy lookup registry-aware
- Keep the existing Rust dispatch path intact
- Add task listing that reports source/path/policy

#### What this phase does not change

- No DSL
- No new execution format
- No change to current task invocation syntax
- No change to `TaskContext` behavior

#### Acceptance criteria

- `cargo check` passes
- Existing task tests still pass
- Existing CLI commands behave the same
- Registry can describe every built-in task with consistent metadata

---

### Phase 2: Configurable Discovery and Diagnostics

**Goal:** make discovery configurable and make validation/reporting clearer.

#### Deliverables

- Add configuration for task search roots
- Allow the registry to scan additional task locations in a controlled way
- Improve conflict detection and error messages
- Add dry-run / inspection mode for task resolution
- Surface path/source/policy in validation errors and task listing
- Update docs so users can understand where tasks come from

#### Acceptance criteria

- Discovery works from config, not just hardcoded repo paths
- Conflicts are explicit and test-covered
- Validation and runtime resolution report the same task source

---

### Phase 3: Future Task Format Exploration

**Goal:** only after the registry foundation is stable, evaluate whether another task format is worthwhile.

#### Important note

This phase is **out of scope** for the current implementation and should be treated as a separate design document.

That future proposal would define:

- format syntax
- parser/executor semantics
- policy metadata for the new format
- migration strategy
- compatibility rules

For now, the proposal should only state that the foundation is being built to make this possible later.

---

## 7. Implementation Plan by File

### `src/task/mod.rs`

- Replace direct task-name assumptions with registry lookups where appropriate
- Keep built-in execution match logic for Phase 1
- Expose registry helpers for task discovery and listing

### `src/validation/task_registry.rs`

- Convert task-file checks into registry-backed resolution
- Stop relying on hardcoded `src/task` relative-path checks
- Use the same resolution rules as the executor

### `src/task/policy.rs`

- Keep built-in static policies for now
- Add registry-aware policy resolution hooks
- Avoid duplicating task-name matching in multiple places

### `src/main.rs`

- Use registry-aware validation results when initializing execution
- Surface better diagnostics when a task is unknown or ambiguous
- Keep the task-running flow unchanged for existing commands

### `src/cli.rs`

- Add optional inspection flags such as `--list-tasks`
- Preserve the current positional task syntax
- Keep task-group parsing behavior unchanged

### `src/config/mod.rs` and validation modules

- Add configuration for task discovery roots in Phase 2
- Validate those roots clearly
- Make the behavior portable across environments

### Documentation

- Update `README.md`
- Update `TASK.md` if registry metadata changes require new guidance
- Add task discovery docs once the registry foundation exists

---

## 8. Testing Strategy

This proposal should not land without tests.

### Required test coverage

- Registry resolves built-in tasks correctly
- Registry reports unknown tasks correctly
- Validation and execution agree on task identity
- Policy lookup stays consistent with task registration
- Task listing includes source/path metadata
- Conflicts are detected and reported clearly
- Configured discovery roots are validated

### Existing baseline should remain green

At minimum, these must continue to pass after the foundation work:

- `cargo check`
- existing unit tests
- registry/validation tests
- task execution smoke tests

### Suggested additional test groups

- `task_registry_tests`
- `task_discovery_tests`
- `task_policy_registry_tests`
- `task_conflict_tests`
- `cli_list_tasks_tests`

---

## 9. Risks and Mitigations

| Risk | Why it matters | Mitigation |
|------|----------------|------------|
| Registry adds complexity | More moving parts than a direct match | Keep Phase 1 Rust-only and narrow |
| Discovery conflicts | Two sources may expose the same task name later | Define precedence and test it |
| CWD-dependent paths | Validation can behave differently across environments | Use explicit roots/config instead of relative paths |
| Policy drift | Execution and validation can diverge | Make policy resolution registry-backed |
| Scope creep | DSL discussion can overwhelm foundation work | Keep DSL out of this proposal |
| Regression risk | Existing tasks must continue to work | Preserve execution behavior in Phase 1 |

---

## 10. Open Questions

1. Should external task roots be opt-in or enabled by default?
2. If a task name appears in multiple places, should that be a hard error or an explicit override?
3. Should `--list-tasks` show policy details or only name/source/path?
4. Should task metadata live in code, in config, or in a companion file when Phase 2 arrives?
5. Should future formats be modeled as separate sources in the same registry, or as separate registries behind one facade?

---

## 11. Recommendation

This proposal should be treated as a **foundation proposal**, not a DSL proposal.

### Recommended decision

- **Approve Phase 1 only after revision**
- **Defer DSL work to a separate future proposal**
- **Do not implement new task syntax yet**

### Summary of the best next step

Build a registry-first task system that makes the current Rust-task architecture explicit and testable. Once that exists, the project can safely decide whether a second task format is worth adding.

---

## 12. Appendix: Current CLI Examples

These commands remain valid and should keep working unchanged:

```bash
cargo run cookiebot
cargo run pageview=www.reddit.com
cargo run cookiebot pageview=reddit.com
cargo run cookiebot then pageview=reddit.com then twitteractivity
```

A registry inspection command can be added later as part of the foundation:

```bash
cargo run --list-tasks
```

That command should report at least:

- task name
- source
- path, if applicable
- policy name
- known/unknown state
