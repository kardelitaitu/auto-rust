# DSL Executor Modularization

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary
This file violates the Single Responsibility Principle (SRP). It handles everything from parsing AST nodes, managing an internal LRU cache (`SelectorCache`), evaluating conditions (`if`/`else`), handling loop unrolling, and executing physical browser actions. This makes the DSL engine rigid and highly prone to bugs when adding new syntax or features.

## Scope
- **In scope**: Refactoring described in the plan.
- **Out of scope**: Changing core logic.

## Next Step
Begin implementation according to plan.md.

# Baseline

## What I Find
The `dsl_executor.rs` file is **2,600 lines** long. It acts as a "God Object" for the entire DSL (Domain Specific Language) execution pipeline.

## What I Claim
This file violates the Single Responsibility Principle (SRP). It handles everything from parsing AST nodes, managing an internal LRU cache (`SelectorCache`), evaluating conditions (`if`/`else`), handling loop unrolling, and executing physical browser actions. This makes the DSL engine rigid and highly prone to bugs when adding new syntax or features.

## What Is the Proof
1. The file contains a custom implementation of an LRU cache (`SelectorCacheEntry` and `SelectorCache`).
2. It houses deeply nested recursive functions to handle variable substitution and control flow blocks (`execute_block`, `evaluate_condition`).
3. Inline tests account for an additional 648 lines inside this same file.

