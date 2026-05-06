# Decisions

## Architecture Decisions

### ADR 1: Keep Shared Types in decision.rs

**Decision**: Extract types to `decision/types.rs` but keep backward-compatible re-exports.

**Rationale**:
- Consumers import from `decision` module, not individual files
- Easier to refactor internals later
- Clear separation: types vs implementation

### ADR 2: UnifiedEngine as Primary Interface

**Decision**: All consumers use `UnifiedEngine`, not individual strategies.

**Rationale**:
- Simpler API: one type to import
- Config-driven: Strategy selection via `DecisionStrategy` enum
- Fallback support built-in

### ADR 3: Internal Strategy Trait

**Decision**: Use `DecisionStrategyImpl` as internal trait, `DecisionEngine` as public trait.

**Rationale**:
- Allows UnifiedEngine to dispatch to strategies
- Public API stays clean
- Strategies can be swapped without changing consumer code

### ADR 4: Preserve Exact Behavior

**Decision**: Copy-paste logic initially, refactor only after tests pass.

**Rationale**:
- No behavioral changes during consolidation
- Easier to verify correctness
- Refactoring is Phase 2 (separate effort)

### ADR 5: Strategy-per-File

**Decision**: Each strategy in its own file under `strategies/`.

**Rationale**:
- Clear organization
- Independent testing
- Easy to add new strategies

## Deferred Decisions

- Extract shared safety checks to separate module
- Refactor EngagementLevel scoring thresholds
- Add metrics/telemetry to decision engines
