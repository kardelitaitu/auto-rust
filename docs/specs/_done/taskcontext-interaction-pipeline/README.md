# TaskContext Interaction Pipeline

Status: `done`

Owner: `spec-agent`
Implementer: `code-implementer`

## Summary

Unify click and type behavior behind one TaskContext interaction pipeline so the public verbs share the same preflight, fallback, verification, and pause rules. The goal is to reduce drift between click, nativeclick, focus, select_all, clear, and type paths and make browser interactions more reliable.

## Scope

- In scope:
  - shared interaction request and result types
  - a new internal pipeline module for TaskContext interactions
  - routing click/type/focus/select_all/clear through shared stages where practical
  - preserving the existing public TaskContext verb surface
  - deterministic tests for click/type reliability and regression protection
- Out of scope:
  - CLI parsing or task registry work
  - browser discovery or session factory refactors
  - runtime shutdown work
  - new task verbs or API redesign

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

## Completed

- Shared interaction types added (InteractionKind, InteractionRequest, InteractionResult)
- Interaction pipeline module created with preflight/execution/postflight
- Internal pipeline methods added to TaskContext
- Public `interact()` method provides unified pipeline access
- 16 deterministic tests added for pipeline reliability
- All 2243 tests pass, CI checks pass
