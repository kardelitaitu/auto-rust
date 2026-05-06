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

# Baseline

## What I Find
The `src/runtime/task_context.rs` file is a massive **5,547 lines** long. However, analysis reveals that only ~100 lines are actual production code (defining the 84 async methods of the `TaskContext` struct). The remaining **5,445 lines** are trapped inside a single `#[cfg(test)]` block at the bottom of the file.

## What I Claim
This is an extreme case of integration test bloat. Placing thousands of lines of tests inside a core runtime file makes the file nearly impossible to navigate, pollutes the IDE experience, and slows down local development. Extracting these will drastically improve developer velocity without changing any runtime behavior.

## What Is the Proof
1. Running line-count analysis shows the `#[cfg(test)]` attribute begins near the top and consumes over 98% of the file's weight.
2. The tests cover a huge array of integration scenarios (mouse clicks, scrolling, DOM evaluation) that belong in dedicated test files, not inline with the struct definition.

