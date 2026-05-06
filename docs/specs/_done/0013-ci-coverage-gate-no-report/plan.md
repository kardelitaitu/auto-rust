# Plan

1. Update `.github/workflows/ci.yml`.
2. Remove report and artifact steps.
3. Keep the coverage floor check.
4. Verify the workflow with repo gates.

# Internal API Outline

- Workflow step: install `cargo-llvm-cov`
- Workflow step: run a no-report coverage gate
- No new runtime API
- No new local script API

# Decisions

- Keep the gate, drop the report.
- Prefer one CI job that fails fast on coverage floor issues.
- Leave local coverage reports to developer workflows.

