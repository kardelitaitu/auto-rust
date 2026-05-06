# TwitterActivity Contract Alignment

Status: `done`

Owner: `spec-agent`
Implementer: `archived-spec-agent`

## Summary

Align the TwitterActivity task with its documented payload contract. Replace the hardcoded duration fallback, make scroll count a real runtime input, reject malformed numeric payloads, and keep the task docs/config examples in sync with runtime behavior.

## Scope

- In scope:
  - `TaskConfig` payload defaults and validation
  - TwitterActivity run-loop scroll budget behavior
  - strict numeric payload parsing
  - regression tests for defaults, malformed payloads, and scroll behavior
  - docs/config example sync for the task contract
- Out of scope:
  - sentiment, persona, or decision-engine changes
  - browser discovery or shutdown refactors
  - CLI parsing or task-registry redesign
  - new Twitter/X task verbs

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

Hand this package to the implementer after the contract and test order are approved.
