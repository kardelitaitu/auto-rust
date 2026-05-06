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

# Baseline

## Current State

- `TaskContext` already exposes separate public verbs for click, nativeclick, focus, type, keyboard, select_all, clear, and click_and_wait.
- Click currently has its own timing, retry, fallback, and learning flow.
- Type currently has its own existence check, focus step, text entry, and verification step.
- `select_all` and `clear` already perform local validation and keyboard interaction.
- Existing integration tests cover many task API behaviors, but the interaction flow is still spread across several methods.

## Known Gaps

- Shared preflight and postflight logic are duplicated across action methods.
- Click and type do not yet share a single internal pipeline contract.
- Verification rules are implemented per method instead of being shaped by one shared result model.
- Reliability drift is possible when a change lands in one verb but not the others.

## Why This Matters

The public API is already stable, but the implementation is harder to reason about than it needs to be. A shared pipeline should make click and type behavior more reliable without changing task code.

