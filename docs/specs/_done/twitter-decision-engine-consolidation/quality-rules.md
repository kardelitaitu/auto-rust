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
