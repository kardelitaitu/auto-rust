# Browser Discovery / Session Assembly

Status: `done`

Owner: `spec-agent`
Implementer: `implementation-agent`

## Summary

Split browser discovery and session assembly into explicit connector, factory, and pool boundaries so startup becomes easier to reason about, easier to test, and less tightly coupled to `browser.rs`.

## Scope

- In scope:
  - connector abstraction for discovery and connect flows
  - session factory ownership of `Session` construction
  - session pool management for retry and parallel discovery
  - browser capability detection attached to sessions
  - preserving current browser filter behavior and discovery semantics
- Out of scope:
  - CLI parsing and task registry work
  - runtime shutdown work
  - task execution or interaction refactors
  - new browser driver support beyond the existing discovery sources

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

Hand this package to the implementer agent after the CLI spec is available.
