# CI Coverage Gate No Report

Status: Done (Archived)

Owner: `spec-agent`
Implementer: `pending`

## Summary

Keep the CI coverage floor, but stop generating or uploading coverage reports in GitHub Actions. The gate should still fail on low coverage, yet CI should stay lighter and easier to read.

## Scope

- In scope:
  - simplify `.github/workflows/ci.yml` coverage job
  - keep the coverage threshold check
  - remove HTML, JSON, and artifact upload steps from CI
- Out of scope:
  - changing local coverage tooling
  - changing runtime behavior
  - changing test coverage policy

## Files

- `spec.yaml`
- `baseline.md`
- `internal-api-outline.md`
- `plan.md`
- `validation-checklist.md`
- `ci-commands.md`
- `decisions.md`
- `quality-rules.md`
- `implementation-notes.md`

## Next Step

Implement the workflow change, then verify CI and local gates still pass.

