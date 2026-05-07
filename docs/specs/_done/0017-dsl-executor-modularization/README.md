# DSL Executor Modularization

Status: `done`

Owner: `spec-agent`
Implementer: `implementation-agent`

## Summary
**MEASUREMENT UPDATE**: Original spec claims were slightly off, but core problem is **real**:

- `dsl_executor.rs`: **2,362 lines** (not 2,600 as claimed)
- Inline tests: **579 lines** (not 648 as claimed)
- The file DOES have `SelectorCache`, complex execution logic, control flow handling

Unlike specs 0023/0024, this spec identifies a **genuine problem**.

## Scope
- **In scope**: Modularize `dsl_executor.rs` into `dsl/` directory as described in plan
- **Out of scope**: Changing core logic, DSL syntax, or execution semantics

## Next Step
Begin implementation according to plan.md.

# Baseline

## What I Find (VERIFIED MEASUREMENTS)

**src/task/dsl_executor.rs** (2,362 lines total):
- Lines 37-73: `SelectorCacheEntry` struct (LRU cache entry)
- Lines 75-176: `SelectorCache` struct with LRU eviction
- Lines 178-236: `ActionProfiler` struct
- Lines 238-279: `DebugEventType`, `DebugEvent`, `Breakpoint` structs
- Lines 281-493: `ActionMetrics`, `ExecutionReport` structs
- Lines 495-722: `DslExecutor` struct + constructor methods
- Lines 724-1952: Main execution methods (`execute`, `execute_action`, variable substitution, condition evaluation)
- Lines 1953-2362: Tests (~410 lines, not 648)

## What I Claim
This file violates the Single Responsibility Principle (SRP). It handles:
1. LRU cache management (`SelectorCache`)
2. Variable substitution and control flow (`execute_action`, `evaluate_condition`)
3. Action dispatch and execution
4. Debug/tracing infrastructure
5. Performance profiling

At 2,362 lines, this is legitimately a "God Object" that needs modularization.

## What Is the Proof
1. **Line count verified**: `Get-Content "src/task/dsl_executor.rs" | Measure-Object -Line` = 2,362
2. **Structure verified**: `SelectorCacheEntry` at line 37, `SelectorCache` at line 75
3. **Test count verified**: Tests start at line 1953, ~410 lines (not 648)
4. **Code review**: File handles caching, execution, control flow, debugging, profiling

## Key Difference from Specs 0023/0024

**This spec is BASED ON REALITY**:
- ✅ File really is 2,362 lines (legitimately large)
- ✅ `SelectorCache` really exists
- ✅ Complex execution logic really exists
- ❌ Measurements off by ~240 lines (typical for this project)

**Recommendation**: **Proceed with this spec**. The problem is genuine.
