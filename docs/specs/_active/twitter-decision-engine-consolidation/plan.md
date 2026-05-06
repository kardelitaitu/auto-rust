# Plan

## Phase 2: Create Unified Foundation (2-3 hours)

1. **Create `decision/` directory structure**
   ```
   src/utils/twitter/decision/
     mod.rs
     types.rs
     engine.rs
     strategies/
       mod.rs
   ```

2. **Extract shared types to `types.rs`**
   - `TweetContext`
   - `EngagementDecision`
   - `EngagementLevel`
   - `DecisionStrategy`
   - Move from decision.rs

3. **Implement `DecisionEngine` trait in `engine.rs`**
   - Define trait (preserve exact signature)
   - Create `UnifiedEngine` struct
   - Implement strategy dispatch

## Phase 3: Migrate Legacy Strategy (2-3 hours)

1. **Create `strategies/legacy.rs`**
   - Copy keyword blocklists
   - Copy quality scoring
   - Copy `decide_engagement()` logic
   - Implement `DecisionStrategyImpl`

2. **Add tests for legacy strategy**
   - Verify identical behavior

## Phase 4: Migrate Persona Strategy (1-2 hours)

1. **Create `strategies/persona.rs`**
   - Copy persona weight application
   - Wrap legacy strategy
   - Implement trait

## Phase 5: Migrate LLM Strategy (2-3 hours)

1. **Create `strategies/llm.rs`**
   - Copy OpenAI integration
   - Copy response parsing
   - Implement trait

## Phase 6: Migrate Hybrid Strategy (2-3 hours)

1. **Create `strategies/hybrid.rs`**
   - Copy strategy combination logic
   - Implement trait

## Phase 7: Migrate Unified Strategy (2-3 hours)

1. **Create `strategies/unified.rs`**
   - Copy single-call LLM logic
   - Copy content generation
   - Implement trait

## Phase 8: Integration (2-3 hours)

1. **Update `decision/mod.rs`**
   - Export clean API
   - Re-export from types/engine

2. **Update `twitter/mod.rs`**
   - Replace 5 exports with single `decision` module

3. **Update consumers**
   - Fix imports in engagement.rs

4. **Delete old files**
   - Remove 5 individual engine files

## Phase 9: Validation (2-3 hours)

1. **Test migration**
   - All existing tests pass
   - No behavioral changes

2. **Run full CI**
   - `cargo test`
   - `cargo clippy`
   - `cargo build --release`

## Estimated Total: 17-25 hours
