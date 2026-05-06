# TaskContext Integration Test Extraction

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary
This is an extreme case of integration test bloat. Placing thousands of lines of tests inside a core runtime file makes the file nearly impossible to navigate, pollutes the IDE experience, and slows down local development. Extracting these will drastically improve developer velocity without changing any runtime behavior.

## Scope
- **In scope**: Refactoring described in the plan.
- **Out of scope**: Changing core logic.

## Next Step
Begin implementation according to plan.md.
