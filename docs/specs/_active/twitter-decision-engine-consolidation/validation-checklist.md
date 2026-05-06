# Validation Checklist

## Pre-Implementation

- [x] Baseline analysis complete for all 5 engines
- [x] Unique capabilities documented per engine
- [x] Migration strategy approved
- [x] Consumer impact assessed (engagement.rs)

## During Implementation

### Phase 2: Foundation
- [ ] `decision/` directory created
- [ ] `types.rs` with all shared types
- [ ] `DecisionEngine` trait defined
- [ ] `UnifiedEngine` structure implemented

### Phase 3: Legacy Strategy
- [ ] `strategies/legacy.rs` created
- [ ] Logic identical to original
- [ ] Tests pass

### Phase 4-7: Other Strategies
- [ ] Persona strategy migrated
- [ ] LLM strategy migrated
- [ ] Hybrid strategy migrated
- [ ] Unified strategy migrated

### Phase 8: Integration
- [ ] `mod.rs` updated with clean exports
- [ ] `twitter/mod.rs` updated
- [ ] Consumers compile
- [ ] Old files removed

## Post-Implementation

### Testing
- [ ] All existing tests pass (`cargo test`)
- [ ] No test modifications required
- [ ] Decision behavior identical

### Code Quality
- [ ] `cargo clippy` clean
- [ ] No `unwrap()` in new code
- [ ] Documentation complete

### Metrics
- [ ] Lines reduced by ~40% (target: ~900 vs ~2100)
- [ ] Public API surface reduced

### API Verification
- [ ] `DecisionEngine` trait callable
- [ ] `UnifiedEngine::with_strategy()` works
- [ ] All 5 strategies selectable

## Sign-off

- [ ] `spec-lint.ps1` passes
- [ ] `check-fast.ps1` passes
- [ ] `check.ps1` passes
- [ ] Ready to move spec to `_done/`
