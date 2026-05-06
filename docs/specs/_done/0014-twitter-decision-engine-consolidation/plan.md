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

# Internal API Outline

## Core Types (from decision.rs - consolidated)

### TweetContext

```rust
#[derive(Debug, Clone)]
pub struct TweetContext {
    pub tweet_id: String,
    pub text: String,
    pub author: String,
    pub replies: Vec<String>,
    pub persona: PersonaWeights,
    pub task_config: TaskConfig,
    pub tweet_age: String,
    pub topic_alignment: String,
}
```

### EngagementDecision

```rust
#[derive(Debug, Clone)]
pub struct EngagementDecision {
    pub level: EngagementLevel,
    pub score: i32,
    pub reason: String,
    pub multiplier: f64,
    pub confidence: f64,
}
```

### EngagementLevel

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngagementLevel {
    Full,    // like, retweet, reply, follow, quote
    Medium,  // like, retweet
    Minimal, // like only
    None,    // skip
}
```

### DecisionStrategy

```rust
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStrategy {
    #[default]
    Legacy,  // Rule-based only
    Persona, // Persona-weighted
    Llm,     // LLM-based
    Hybrid,  // Combined approach
    Unified, // Single LLM call for decision + content
    Auto,    // Auto-select based on config
}
```

## DecisionEngine Trait (moved to engine.rs)

```rust
#[async_trait]
pub trait DecisionEngine: Send + Sync {
    fn name(&self) -> &'static str;
    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision;
    fn is_available(&self) -> bool { true }
}
```

## UnifiedEngine (new main implementation)

```rust
pub struct UnifiedEngine {
    strategy: Box<dyn DecisionStrategyImpl>,
    fallback: Option<Box<dyn DecisionStrategyImpl>>,
}

impl UnifiedEngine {
    pub fn with_strategy(strategy: DecisionStrategy) -> Self;
    pub fn with_llm_key(strategy: DecisionStrategy, api_key: String) -> Self;
}

#[async_trait]
impl DecisionEngine for UnifiedEngine {
    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision {
        // Try primary strategy, fallback if unavailable
    }
}
```

## Strategy Implementation Trait (internal)

```rust
// Internal trait for strategy implementations
#[async_trait]
pub(crate) trait DecisionStrategyImpl: Send + Sync {
    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision;
    fn strategy_type(&self) -> DecisionStrategy;
    fn is_available(&self) -> bool;
    fn name(&self) -> &'static str;
}
```

## Strategy Modules

### LegacyStrategy (decision/strategies/legacy.rs)

```rust
pub(crate) struct LegacyStrategy {
    config: LegacyConfig,
}

#[async_trait]
impl DecisionStrategyImpl for LegacyStrategy {
    // Keyword blocklist logic from original decision.rs
    // Quality scoring algorithm
    // EngagementLevel determination
}
```

### PersonaStrategy (decision/strategies/persona.rs)

```rust
pub(crate) struct PersonaStrategy {
    base: LegacyStrategy,
    persona: PersonaWeights,
}

#[async_trait]
impl DecisionStrategyImpl for PersonaStrategy {
    // Wraps legacy with persona weight application
}
```

### LlmStrategy (decision/strategies/llm.rs)

```rust
pub(crate) struct LlmStrategy {
    client: LlmClient,
    config: LlmConfig,
}

#[async_trait]
impl DecisionStrategyImpl for LlmStrategy {
    // OpenAI API integration
    // Response parsing
}
```

### HybridStrategy (decision/strategies/hybrid.rs)

```rust
pub(crate) struct HybridStrategy {
    strategies: Vec<(Box<dyn DecisionStrategyImpl>, f64)>, // (strategy, weight)
}

#[async_trait]
impl DecisionStrategyImpl for HybridStrategy {
    // Weighted combination of multiple strategies
}
```

### UnifiedStrategy (decision/strategies/unified.rs)

```rust
pub(crate) struct UnifiedStrategy {
    client: LlmClient,
    config: UnifiedConfig,
}

#[async_trait]
impl DecisionStrategyImpl for UnifiedStrategy {
    // Single LLM call for decision + content generation
}
```

## Factory (preserved from decision.rs)

```rust
pub struct DecisionEngineFactory;

impl DecisionEngineFactory {
    pub fn create(
        strategy: DecisionStrategy,
        llm_api_key: Option<String>,
    ) -> Box<dyn DecisionEngine> {
        // Create UnifiedEngine with appropriate strategy
    }
}
```

## Module Exports (decision/mod.rs)

```rust
pub mod types;
pub mod engine;
pub(crate) mod strategies;

pub use types::{
    TweetContext,
    EngagementDecision,
    EngagementLevel,
    DecisionStrategy,
    DecisionEngineFactory,
};
pub use engine::{DecisionEngine, UnifiedEngine};
```

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

