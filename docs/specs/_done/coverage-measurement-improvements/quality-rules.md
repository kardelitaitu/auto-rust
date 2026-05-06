# Quality Rules

- The coverage command must be deterministic and runnable from the repo root.
- Coverage policy must be explicit in the workflow, not hidden in ad hoc scripts.
- Trend output must be stable enough to compare across runs.
- The gate must not block on flaky formatting or unrelated build noise.
- Coverage tooling must not replace real test coverage work.
- Keep the local command short enough for repeated use during iteration.
