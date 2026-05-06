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
