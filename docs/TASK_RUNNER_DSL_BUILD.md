# Task Runner DSL Build Plan

last audited 08-05-26 by Kilo

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
