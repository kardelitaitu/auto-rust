# Task Runner Preparation Plan

last audited 08-05-26 by Kilo

> Companion to: `TASK_RUNNER_PROPOSAL.md`

This document prepares the foundation for the future DSL runner.

---

## 1. Purpose

Before introducing a DSL, we must stabilize the task registry, validation, and policy model.

This is the **preparation phase** - no DSL code yet.

---

## 2. Registry Foundation

### Current State
- Tasks registered in `src/task/mod.rs` via `TASK_NAMES` array
- `perform_task()` dispatches to task functions
- Policies defined in `src/task/policy.rs`
- Validation in `src/task/validation.rs`

### Goals
1. Stabilize task registration pattern
2. Complete policy system (✅ done - see `docs/TASK_POLICY_IMPLEMENTED.md`)
3. Complete validation module
4. Document task authoring contract

---

## 3. Preparation Checklist

- [x] Task policy system implemented
- [x] Permission-based security complete
- [x] Registry pattern stable
- [ ] Validation module documented
- [ ] Task authoring guide complete (see `docs/TUTORIAL_BUILDING_FIRST_TASK.md`)
- [ ] DSL syntax finalized (see `TASK_RUNNER_PROPOSAL.md`)

---

## 4. Next Steps

After preparation:
1. Implement DSL parser (see `TASK_RUNNER_DSL_BUILD.md`)
2. Add DSL executor
3. Migrate suitable tasks to DSL format
4. Keep Rust tasks for complex logic

---

**Status:** Preparation phase - no DSL code yet.
