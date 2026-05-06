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

# CI Commands

## Development Loop

```powershell
# Quick check during development
.\check-fast.ps1
```

## Pre-Commit Verification

```powershell
# Full verification before handoff
.\check.ps1
```

## Specific Checks

```powershell
# Run only twitter-related tests
cargo test twitteractivity_decision

# Check compilation of decision module
cargo check --lib 2>&1 | Select-String -Pattern "decision"

# Clippy focused on new code
cargo clippy --package rust-orchestrator --lib -- -W clippy::unwrap_used 2>&1 | Select-String -Pattern "decision"
```

## Coverage

```powershell
# Run coverage report
.\coverage.ps1

# Check decision module coverage
cargo tarpaulin --out Stdout 2>&1 | Select-String -Pattern "decision"
```

## Spec Lint

```powershell
# Validate spec package
.\spec-lint.ps1
```

# Quality Rules

## Code Quality

1. **No unwrap() in new code**
   - Use `?` operator for error propagation
   - Use `ok_or_else()` with context

2. **Preserve existing behavior exactly**
   - Copy logic verbatim initially
   - Match ordering of conditions
   - Preserve error messages

3. **Documentation required**
   - `///` on every public function
   - `//!` module documentation
   - SAFETY comments for any unwrap()

## Testing Standards

1. **Test coverage requirement**: 80% for new decision module
2. **Behavioral tests**: Verify identical output to original engines
3. **Strategy switching tests**: All strategies must be selectable

## API Design

1. **Public surface minimal**: Only what's needed by `engagement.rs`
2. **Config compatibility**: Existing TOML configs continue working
3. **Builder pattern**: Complex configuration uses builder

## File Organization

```
decision/
  mod.rs              # Public exports only
  types.rs            # Public types
  engine.rs           # UnifiedEngine implementation
  strategies/
    mod.rs            # Strategy exports (pub(crate))
    legacy.rs         # Legacy implementation
    persona.rs        # Persona implementation
    llm.rs            # LLM implementation
    hybrid.rs         # Hybrid implementation
    unified.rs        # Unified implementation
```

## Migration Rules

1. Copy code, verify tests, then delete old file
2. Never modify while moving
3. Add tests before deleting original

