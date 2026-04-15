# Agent 6 Spec: Release, Wrapper Scripts, Logging, And CI

## Scope
Own deployment and operations readiness docs/scripts.

## Files
- `migration-steps/phase-6-polish-deployment.md`
- wrapper scripts references (`run.bat`, `run.sh`)
- CI workflow docs/examples

## Functional Requirements
- Windows examples must use Windows-compatible commands.
- Wrapper scripts must resolve binary path correctly.
- Release checklist must match actual build/test flow.
- Logging guidance must keep non-blocking file safety notes.

## Acceptance Checks
- `run.bat` path variable resolves correctly.
- Windows binary inspection command is valid.
- CI section uses non-interactive cargo commands and sane cache keys.

## Out Of Scope
- Parser logic
- Task migration internals

## Report Format
- Script/doc fixes
- Commands validated on target shell assumptions
- Remaining environment-specific caveats
