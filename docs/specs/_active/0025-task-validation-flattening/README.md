# Task Validation Deep Nesting Flattening

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary
"Arrow code" (extreme deep nesting) indicates that the validation logic is trying to traverse deeply nested JSON structures or ASTs using nested `if let`, `match`, or loops all in a single block. This destroys readability and exponentially increases cyclomatic complexity.

## Scope
- **In scope**: Refactoring described in the plan.
- **Out of scope**: Changing core logic.

## Next Step
Begin implementation according to plan.md.

# Baseline

## What I Find
The `src/task/validation.rs` file is 1,460 lines long. Its primary function, `validate_task_with_known_tasks`, is nearly 400 lines long. More critically, over **300 lines of code in this file are indented by 20 spaces or more** (5+ levels of deep nesting).

## What I Claim
"Arrow code" (extreme deep nesting) indicates that the validation logic is trying to traverse deeply nested JSON structures or ASTs using nested `if let`, `match`, or loops all in a single block. This destroys readability and exponentially increases cyclomatic complexity.

## What Is the Proof
1. Line indentation analysis found `src/task/validation.rs` has the highest density of deeply nested code in the entire project.
2. The file is further bloated by inline tests (nearly 1,000 lines of tests appended to the bottom).

