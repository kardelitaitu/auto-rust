# Plan

## Step 1: Baseline and inventory

- Re-read the TODO coverage section and confirm the three target modules.
- Inspect the current helper functions in each module and list the exact branch points to cover.
- Keep the scope limited to helper coverage unless a behavior path truly needs a smoke test.

## Step 2: Pure unit coverage

- Add deterministic tests for `twitteractivity.rs` payload/config helpers and entry-point selection.
- Add deterministic tests for `twitterfollow.rs` candidate ordering and URL or username extraction.
- Add deterministic tests for `twitterintent.rs` intent parsing and payload URL extraction.

## Step 3: Narrow regression checks

- Add a browser-backed or harness-backed smoke test only if a branch cannot be proven with unit tests.
- Keep any smoke test small and tied to one concrete behavior path.

## Step 4: Verification

- Run `.
check-fast.ps1` during iteration.
- Run `.
check.ps1` before handoff.
- Keep the implementation notes focused on what changed and what coverage gap was closed.

# Internal API Outline

## Coverage Seams

- `src/task/twitteractivity.rs`
  - Keep test focus on payload fixtures, config helpers, and entry-point selection.
  - Prefer pure helper tests over browser-backed `run()` coverage unless a helper cannot capture the branch.

- `src/task/twitterfollow.rs`
  - Keep test focus on locator candidate ordering, URL normalization, and username extraction.
  - Exercise parse and ordering helpers directly so the intent of the code stays visible.

- `src/task/twitterintent.rs`
  - Keep test focus on intent parsing, URL extraction, and query-parameter parsing.
  - Cover supported URL forms and the failure cases that feed task selection.

- `tests/task_api_behavior.rs`
  - Use only for narrow smoke checks when a helper test cannot prove the behavior.

## Rules

- Do not add new public task APIs for the sake of tests.
- Add small helper functions only if they reduce test brittleness and keep the implementation readable.
- Keep all new test seams aligned with current task-module boundaries.

# Decisions

- Limit the first pass to the three named task modules from `TODO.md`.
- Prefer pure helper tests over browser-backed coverage whenever possible.
- Keep the public task API unchanged.
- Use optional smoke tests only when a helper cannot prove the behavior cleanly.

