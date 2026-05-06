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
