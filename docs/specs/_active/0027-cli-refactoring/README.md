# CLI Command Refactoring

Status: `pending`

Owner: `spec-agent`
Implementer: `pending`

## Summary
The `src/cli.rs` file may be handling too many commands directly, mixing argument parsing with execution logic. This spec proposes extracting command handlers into a `src/commands/` directory for better organization.

## Scope
- **In scope**: Extract command handlers from `cli.rs` into `commands/` modules.
- **Out of scope**: Changing command behavior, adding new commands.

## Next Step
Analyze `cli.rs` line count and identify extraction candidates.

# Baseline

## What I Find
The CLI module handles command parsing and execution. Need to verify current state.

## What I Claim
If `cli.rs` exceeds 500 lines with multiple command handlers, extraction will improve maintainability.

## What Is the Proof
1. Large CLI files are harder to navigate
2. Command handlers have distinct logic that can be separated
3. Testing command handlers in isolation is difficult when mixed with parsing
