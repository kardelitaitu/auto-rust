# Plan

## Step 1: Baseline and contract

- Confirm the current click and type method flow in TaskContext.
- Confirm the existing task API tests that already cover click, type, focus, select_all, clear, and click_and_wait.
- Lock the public API stability rule before any code changes.

## Step 2: Shared types

- Add the smallest set of shared interaction request/result types.
- Keep the types focused on action kind, selector, verification, and fallback data.
- Avoid moving unrelated TaskContext state into the new types.

## Step 3: Shared pipeline

- Add a new internal interaction pipeline module.
- Route click and type through the shared preflight and postflight stages.
- Reuse the pipeline where focus, select_all, and clear can share the same guard rails.

## Step 4: Reliability tests

- Add tests that prove click and type use the same shared flow.
- Add regression tests for selector verification and fallback handling.
- Keep browser-backed tests narrow and deterministic.

## Step 5: Verification

- Run `.
check-fast.ps1` during implementation.
- Run `.
check.ps1` before handoff.
- Move the spec to `_done/` only after the full gate passes.
