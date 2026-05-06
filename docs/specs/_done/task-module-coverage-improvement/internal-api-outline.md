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
