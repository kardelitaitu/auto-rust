# Section 7.1 Development Plan: Strategy Pattern for Smart Decisions

## Overview

Transform the current boolean-flag-based smart decision system into a proper Strategy pattern with trait-based decision engines. This will enable pluggable decision strategies, easier testing, and cleaner separation of concerns.

---

## Current State Analysis

### Current Architecture
```
twitteractivity_engagement.rs
├── handle_engagement_decision()  # Checks boolean flag
│   └── if smart_decision_enabled:
│       └── decide_engagement()   # Rule-based engine
└── process_candidate()
    └── calls handle_engagement_decision() if flag set

TaskConfig
├── smart_decision_enabled: bool  # Runtime flag
└── llm_enabled: bool             # Another flag
```

### Problems with Current Approach
1. **Boolean flag coupling** - `smart_decision_enabled` check scattered in multiple places
2. **Hard-coded decision logic** - Only one decision engine (rule-based)
3. **Difficult to extend** - Adding new strategies requires modifying core code
4. **Testing complexity** - Cannot easily mock decision logic
5. **No runtime selection** - Can't switch strategies without config changes

---

## Simplification: Remove Internal Sentiment Analysis

Based on testing, we will **remove internal sentiment analysis** and let the LLM infer sentiment directly from tweet content and replies.

### Why Remove Sentiment Analysis?

| Approach | Complexity | Accuracy | Cost |
|----------|------------|----------|------|
| Internal sentiment + LLM | High (2 steps) | Moderate | Higher (API + LLM) |
| **LLM only with raw replies** | **Low (1 step)** | **Higher** | **Lower (LLM only)** |

### What Changes

**Remove:**
- `twitteractivity_sentiment.rs` (basic sentiment)
- `twitteractivity_sentiment_enhanced.rs` (enhanced sentiment)
- `enhanced_sentiment_enabled` config flag
- `sentiment_templates` field
- Pre-processing sentiment API calls

**Keep:**
- Extract first 5 replies from tweet data
- Pass raw reply text to LLM in prompt
- Let LLM infer community reception from actual reply content

### TweetContext Simplification

```rust
pub struct TweetContext {
    pub tweet_id: String,
    pub text: String,
    pub author: String,
    pub replies: Vec<String>,  // Raw reply text, max 5 items
    pub persona: PersonaWeights,
    pub task_config: TaskConfig,
    // REMOVED: sentiment, enhanced_sentiment fields
}
```

### Reply Extraction (First 5 Only)

```rust
pub fn extract_replies_for_llm(tweet: &Value) -> Vec<String> {
    tweet.get("replies")?
        .as_array()?
        .iter()
        .take(5)
        .filter_map(|r| {
            let author = r.get("author")?.as_str()?;
            let text = r.get("text")?.as_str()?;
            Some(format!("@{}: {}", author, text))
        })
        .collect()
}
```

### LLM Prompt Format

```
TWEET: "Just launched my AI startup!"
REPLIES:
- @user1: Congrats! What's the focus?
- @user2: Looking forward to trying it
- @user3: Exciting! Best of luck
- @user4: AI space is crowded but wishing you success
- @user5: Link? Would love to check it out

PERSONA: Casual (like_prob: 0.4, reply_prob: 0.05)
```

LLM infers sentiment from actual reply content. More accurate, simpler code, lower cost.

---

## LLM Provider: Qwen-Turbo (Alibaba Cloud Int.)

### Selected Model
| Provider | Model | Input | Output | Cache Hit |
|----------|-------|-------|--------|-----------|
| Alibaba Cloud Int. | **Qwen-Turbo** | **$0.023/M** | **$0.128/M** | **37.5%** |

### Cost Calculation (Qwen-Turbo)

**Per-Tweet Analysis:**
| Component | Tokens | Price / 1M | Cost |
|-----------|--------|------------|------|
| Input (tweet + 5 replies + persona) | ~280 | $0.023 | **$0.00000644** |
| Output (JSON decision) | ~50 | $0.128 | **$0.0000064** |
| **TOTAL per tweet** | ~330 | - | **$0.00001284** |

**At Scale (1,000 tweets/day):**
| Scenario | Daily Tokens | Daily Cost | Monthly Cost |
|----------|--------------|------------|--------------|
| LLM every tweet | 330K | **$0.0128** | **$0.39** |
| Hybrid 10% LLM | 33K | **$0.0013** | **$0.04** |
| Hybrid 10% + 37.5% cache | ~24K | **$0.0009** | **$0.03** |

**Comparison with other providers:**
| Model | Per 1K tweets | Monthly (30K) |
|-------|---------------|---------------|
| **Qwen-Turbo** | $0.0128 | **$0.39** |
| DeepSeek V4 Flash | $0.053 | $1.59 |
| GPT-4o Mini | $0.07 | $2.10 |
| GPT-3.5 Turbo | $0.023 | $0.69 |

**Qwen-Turbo is:**
- **4x cheaper** than DeepSeek V4 Flash
- **5.5x cheaper** than GPT-4o Mini
- **Same price** as GPT-3.5 Turbo but faster

### Cache Strategy with 37.5% Hit Rate

With Qwen's 37.5% cache hit rate on Alibaba Cloud:

```
1000 tweets/day breakdown:
├── 625 tweets: Fresh LLM calls (62.5%) = $0.0080
└── 375 tweets: Cache hits (37.5%) = $0.0009 (5x cheaper)

Total: ~$0.009/day = $0.27/month
```

**Why Qwen-Turbo wins:**
- ✅ Cheapest per token
- ✅ Built-in caching reduces cost further
- ✅ Fast enough for real-time decisions
- ✅ Good JSON output consistency

---

## Target Architecture

### Proposed Design
```
┌─────────────────────────────────────────────────────────────┐
│                    DecisionEngine Trait                     │
│  async fn decide(&self, tweet: &TweetContext) -> Decision     │
└─────────────────────────────────────────────────────────────┘
                              ▲
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
┌───────┴───────┐    ┌──────┴──────┐    ┌───────┴────────┐
│ PersonaEngine │    │ LLMEngine   │    │ HybridEngine   │
│               │    │             │    │                │
│ • Rule-based  │    │ • LLM calls │    │ • Combines     │
│ • Persona     │    │ • Scoring   │    │   multiple     │
│   weights     │    │ • Context   │    │ • Weighted     │
│ • Sentiment   │    │   aware     │    │   ensemble     │
└───────────────┘    └─────────────┘    └────────────────┘
```

---

## Implementation Phases

### Phase 1: Foundation (Day 1-2)
**Goal:** Create the trait and basic types

#### 1.1 Create `DecisionEngine` Trait
**File:** `src/utils/twitter/twitteractivity_decision.rs` (extend existing)

```rust
/// Context passed to decision engines
pub struct TweetContext {
    pub tweet_id: String,
    pub text: String,
    pub author: String,
    pub replies: Vec<(String, String)>,
    pub persona: PersonaWeights,
    pub task_config: TaskConfig,
    pub sentiment: Option<SentimentResult>,
    pub enhanced_sentiment: Option<EnhancedSentimentResult>,
}

/// Decision result from any engine
pub struct EngagementDecision {
    pub level: EngagementLevel,  // Skip, Low, Medium, High
    pub score: u8,               // 0-100 quality score
    pub reason: String,          // Human-readable reason
    pub interest_multiplier: f64, // Applied to base probabilities
    pub confidence: f64,           // Engine confidence 0.0-1.0
}

/// Core trait for all decision engines
#[async_trait::async_trait]
pub trait DecisionEngine: Send + Sync {
    /// Engine name for logging/metrics
    fn name(&self) -> &'static str;
    
    /// Make engagement decision for a tweet
    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision;
    
    /// Check if engine is available (e.g., LLM API reachable)
    fn is_available(&self) -> bool {
        true
    }
}
```

#### 1.2 Create Engine Factory
**File:** `src/utils/twitter/twitteractivity_decision.rs`

```rust
/// Factory for creating decision engines based on config
pub struct DecisionEngineFactory;

impl DecisionEngineFactory {
    pub fn create(config: &TaskConfig) -> Box<dyn DecisionEngine> {
        match config.decision_strategy {
            DecisionStrategy::Persona => Box::new(PersonaEngine::new(config)),
            DecisionStrategy::LLM => Box::new(LLMEngine::new(config)),
            DecisionStrategy::Hybrid => Box::new(HybridEngine::new(config)),
            DecisionStrategy::Auto => {
                // Auto-select based on config and availability
                if config.llm_enabled {
                    Box::new(HybridEngine::new(config))
                } else {
                    Box::new(PersonaEngine::new(config))
                }
            }
        }
    }
}

/// Strategy selection enum (replaces boolean flags)
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStrategy {
    #[default]
    Persona,    // Rule-based only
    LLM,        // LLM-based only
    Hybrid,     // Combined approach
    Auto,       // Auto-select based on config
}
```

#### 1.3 Update `TaskConfig`
**File:** `src/utils/twitter/twitteractivity_state.rs`

```rust
pub struct TaskConfig {
    // ... existing fields ...
    
    // NEW: Replace boolean flags with strategy enum
    pub decision_strategy: DecisionStrategy,
    
    // DEPRECATED: Remove these (backward compatibility only)
    // pub smart_decision_enabled: bool,  // REMOVED
    // pub llm_enabled: bool,             // REMOVED
}

impl TaskConfig {
    fn from_payload(payload: &Value) -> Self {
        // ... existing parsing ...
        
        // Parse new strategy field, with fallback to legacy flags
        let decision_strategy = payload
            .get("decision_strategy")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str(&format!("\"{}\"", s)).ok())
            .unwrap_or_else(|| {
                // Backward compatibility: derive from legacy flags
                let smart = payload
                    .get("smart_decision_enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let llm = payload
                    .get("llm_enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                
                if smart && llm {
                    DecisionStrategy::Hybrid
                } else if smart {
                    DecisionStrategy::Persona  // smart was rule-based
                } else if llm {
                    DecisionStrategy::LLM
                } else {
                    DecisionStrategy::Persona
                }
            });
        
        Self {
            // ...
            decision_strategy,
        }
    }
}
```

---

### Phase 2: Implement PersonaEngine (Day 3)
**Goal:** Extract existing rule-based logic into a proper engine

#### 2.1 Create `PersonaEngine`
**File:** `src/utils/twitter/twitteractivity_decision_persona.rs` (new file)

```rust
//! Rule-based decision engine using persona weights

use super::twitteractivity_decision::{DecisionEngine, TweetContext, EngagementDecision};
use super::twitteractivity_persona::PersonaWeights;

pub struct PersonaEngine {
    persona: PersonaWeights,
    sentiment_enabled: bool,
}

impl PersonaEngine {
    pub fn new(config: &TaskConfig) -> Self {
        Self {
            persona: config.persona_weights(),
            sentiment_enabled: config.enhanced_sentiment_enabled,
        }
    }
}

#[async_trait::async_trait]
impl DecisionEngine for PersonaEngine {
    fn name(&self) -> &'static str {
        "persona"
    }
    
    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision {
        // Move existing decide_engagement() logic here
        // Adapt to use ctx instead of raw parameters
        
        let text_lower = ctx.text.to_lowercase();
        
        // 1. Check hard blocklists
        if is_blocked(&text_lower) {
            return EngagementDecision {
                level: EngagementLevel::Skip,
                score: 0,
                reason: "Blocked by filter".to_string(),
                interest_multiplier: 0.0,
                confidence: 1.0,
            };
        }
        
        // 2. Calculate base score from persona + sentiment
        let base_score = calculate_base_score(ctx, &self.persona);
        
        // 3. Apply sentiment analysis if enabled
        let sentiment_boost = if self.sentiment_enabled {
            calculate_sentiment_boost(ctx)
        } else {
            0.0
        };
        
        // 4. Determine engagement level
        let (level, multiplier) = determine_engagement_level(
            base_score + sentiment_boost,
            &self.persona
        );
        
        EngagementDecision {
            level,
            score: (base_score + sentiment_boost).clamp(0.0, 100.0) as u8,
            reason: format!("Persona-based: {} (base={:.1}, sentiment={:.1})", 
                level.description(), base_score, sentiment_boost),
            interest_multiplier: multiplier,
            confidence: 0.7, // Rule-based has moderate confidence
        }
    }
}
```

---

### Phase 3: Implement LLMEngine (Day 4-5)
**Goal:** Create LLM-based decision engine

#### 3.1 Create `LLMEngine`
**File:** `src/utils/twitter/twitteractivity_decision_llm.rs` (new file)

```rust
//! LLM-based decision engine for smart engagement

use super::twitteractivity_decision::{DecisionEngine, TweetContext, EngagementDecision};
use super::twitteractivity_llm::{call_llm_api, EngagementPrompt};

pub struct LLMEngine {
    api_url: String,
    api_key: String,
    model: String,
    timeout_ms: u64,
}

impl LLMEngine {
    pub fn new(config: &TaskConfig) -> Self {
        Self {
            api_url: config.llm_api_url.clone(),
            api_key: config.llm_api_key.clone(),
            model: config.llm_model.clone(),
            timeout_ms: config.llm_timeout_ms,
        }
    }
}

#[async_trait::async_trait]
impl DecisionEngine for LLMEngine {
    fn name(&self) -> &'static str {
        "llm"
    }
    
    fn is_available(&self) -> bool {
        !self.api_key.is_empty() && !self.api_url.is_empty()
    }
    
    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision {
        // Build prompt from context
        let prompt = EngagementPrompt {
            tweet_text: &ctx.text,
            replies: &ctx.replies,
            persona: &ctx.persona,
            author: &ctx.author,
        };
        
        // Call LLM API
        match call_llm_api(&prompt, &self.api_url, &self.api_key, self.timeout_ms).await {
            Ok(llm_response) => {
                EngagementDecision {
                    level: llm_response.engagement_level,
                    score: llm_response.quality_score,
                    reason: llm_response.reasoning,
                    interest_multiplier: llm_response.suggested_multiplier,
                    confidence: llm_response.confidence,
                }
            }
            Err(e) => {
                log::warn!("LLM decision failed: {}, falling back to persona", e);
                // Fallback: return neutral decision
                EngagementDecision {
                    level: EngagementLevel::Medium,
                    score: 50,
                    reason: format!("LLM error fallback: {}", e),
                    interest_multiplier: 1.0,
                    confidence: 0.5,
                }
            }
        }
    }
}
```

---

### Phase 4: Implement HybridEngine (Day 6)
**Goal:** Create ensemble engine combining multiple strategies

#### 4.1 Create `HybridEngine`
**File:** `src/utils/twitter/twitteractivity_decision_hybrid.rs` (new file)

```rust
//! Hybrid decision engine combining multiple strategies with weighting

use super::twitteractivity_decision::{DecisionEngine, TweetContext, EngagementDecision};

pub struct HybridEngine {
    engines: Vec<(Box<dyn DecisionEngine>, f64)>, // (engine, weight)
    fallback_strategy: FallbackStrategy,
}

pub enum FallbackStrategy {
    UseFirst,       // Use first successful engine
    WeightedVote,   // Combine all engine scores
    BestScore,      // Pick highest confidence
}

impl HybridEngine {
    pub fn new(config: &TaskConfig) -> Self {
        let mut engines: Vec<(Box<dyn DecisionEngine>, f64)> = Vec::new();
        
        // Always include persona engine as baseline
        engines.push((
            Box::new(PersonaEngine::new(config)),
            config.persona_weight.unwrap_or(0.3)
        ));
        
        // Add LLM if enabled and available
        if config.llm_enabled {
            let llm_engine = LLMEngine::new(config);
            if llm_engine.is_available() {
                engines.push((
                    Box::new(llm_engine),
                    config.llm_weight.unwrap_or(0.7)
                ));
            }
        }
        
        Self {
            engines,
            fallback_strategy: FallbackStrategy::WeightedVote,
        }
    }
}

#[async_trait::async_trait]
impl DecisionEngine for HybridEngine {
    fn name(&self) -> &'static str {
        "hybrid"
    }
    
    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision {
        let mut decisions: Vec<(EngagementDecision, f64)> = Vec::new();
        
        // Collect decisions from all engines
        for (engine, weight) in &self.engines {
            let decision = engine.decide(ctx).await;
            decisions.push((decision, *weight));
        }
        
        // Combine using weighted vote
        match self.fallback_strategy {
            FallbackStrategy::WeightedVote => {
                combine_weighted(decisions)
            }
            FallbackStrategy::BestScore => {
                decisions.into_iter()
                    .max_by(|a, b| a.0.score.cmp(&b.0.score))
                    .map(|(d, _)| d)
                    .unwrap_or_else(|| default_decision())
            }
            FallbackStrategy::UseFirst => {
                decisions.into_iter()
                    .next()
                    .map(|(d, _)| d)
                    .unwrap_or_else(|| default_decision())
            }
        }
    }
}

fn combine_weighted(decisions: Vec<(EngagementDecision, f64)>) -> EngagementDecision {
    let total_weight: f64 = decisions.iter().map(|(_, w)| w).sum();
    
    // Weighted average of scores
    let weighted_score: f64 = decisions.iter()
        .map(|(d, w)| d.score as f64 * w)
        .sum::<f64>() / total_weight;
    
    // Weighted average of multipliers
    let weighted_multiplier: f64 = decisions.iter()
        .map(|(d, w)| d.interest_multiplier * w)
        .sum::<f64>() / total_weight;
    
    // Combine reasons
    let combined_reason = decisions.iter()
        .map(|(d, _)| format!("{}: {}", d.level, d.reason))
        .collect::<Vec<_>>()
        .join(" | ");
    
    EngagementDecision {
        level: EngagementLevel::from_score(weighted_score as u8),
        score: weighted_score as u8,
        reason: combined_reason,
        interest_multiplier: weighted_multiplier,
        confidence: decisions.iter().map(|(d, _)| d.confidence).sum::<f64>() 
            / decisions.len() as f64,
    }
}
```

---

### Phase 5: Refactor Main Code (Day 7-8)
**Goal:** Integrate engines into the main processing flow

#### 5.1 Update `process_candidate()`
**File:** `src/utils/twitter/twitteractivity_engagement.rs`

```rust
pub async fn process_candidate(
    mut ctx: CandidateContext<'_>,
    actions_this_scan: u32,
    next_scroll: Instant,
    _actions_taken: u32,
) -> Result<CandidateResult> {
    // ... existing setup code ...
    
    // NEW: Create decision engine at start of task
    let decision_engine = DecisionEngineFactory::create(task_config);
    
    // ... later in the function ...
    
    // OLD: Boolean flag check
    // let decision = if task_config.smart_decision_enabled {
    //     handle_engagement_decision(tweet, task_config)
    // } else {
    //     None
    // };
    
    // NEW: Use decision engine
    let tweet_context = TweetContext {
        tweet_id: tweet_id.clone(),
        text: tweet_text.to_string(),
        author: tweet_author.clone(),
        replies: extract_replies(tweet),
        persona: persona.clone(),
        task_config: task_config.clone(),
        sentiment: sentiment_result,
        enhanced_sentiment: enhanced_result,
    };
    
    let decision = decision_engine.decide(&tweet_context).await;
    
    // Apply decision to action selection
    let adjusted_probs = apply_decision_to_probs(&persona, &decision);
    
    // ... rest of processing ...
}
```

#### 5.2 Remove Old Functions
- Remove `handle_engagement_decision()` (lines 29-66)
- Keep `decide_engagement()` but move logic to `PersonaEngine`
- Update all call sites

---

### Phase 6: Update Tests (Day 9)
**Goal:** Ensure all tests pass with new architecture

#### 6.1 Update Existing Tests
**File:** `src/task/twitteractivity.rs` (test section)

```rust
#[test]
fn test_persona_engine_decision() {
    let config = TaskConfig {
        decision_strategy: DecisionStrategy::Persona,
        // ... other fields ...
    };
    
    let engine = PersonaEngine::new(&config);
    let ctx = create_test_context("This is a great tweet!");
    
    let decision = engine.decide(&ctx);
    
    assert!(decision.score > 0);
    assert!(decision.confidence > 0.0);
}

#[test]
fn test_decision_engine_factory() {
    // Test factory creates correct engine types
    let persona_config = TaskConfig {
        decision_strategy: DecisionStrategy::Persona,
        ..Default::default()
    };
    let engine = DecisionEngineFactory::create(&persona_config);
    assert_eq!(engine.name(), "persona");
    
    let llm_config = TaskConfig {
        decision_strategy: DecisionStrategy::LLM,
        llm_enabled: true,
        ..Default::default()
    };
    let engine = DecisionEngineFactory::create(&llm_config);
    assert_eq!(engine.name(), "llm");
}

#[test]
fn test_backward_compatibility() {
    // Test legacy payload still works
    let payload = json!({
        "smart_decision_enabled": true,
        "llm_enabled": true,
        // no decision_strategy field
    });
    
    let config = TaskConfig::from_payload(&payload);
    assert_eq!(config.decision_strategy, DecisionStrategy::Hybrid);
}
```

#### 6.2 Add New Engine Tests
Create `src/utils/twitter/tests/decision_tests.rs`:

```rust
#[tokio::test]
async fn test_llm_engine_fallback() {
    let config = TaskConfig {
        decision_strategy: DecisionStrategy::LLM,
        llm_api_url: "invalid-url".to_string(),
        llm_api_key: "".to_string(), // Will cause failure
        ..Default::default()
    };
    
    let engine = LLMEngine::new(&config);
    let ctx = create_test_context("Test tweet");
    
    // Should fallback gracefully
    let decision = engine.decide(&ctx).await;
    assert!(decision.confidence < 0.6); // Low confidence due to fallback
}

#[tokio::test]
async fn test_hybrid_engine_combines_scores() {
    let config = TaskConfig {
        decision_strategy: DecisionStrategy::Hybrid,
        persona_weight: Some(0.5),
        llm_weight: Some(0.5),
        ..Default::default()
    };
    
    let engine = HybridEngine::new(&config);
    let ctx = create_test_context("Interesting content here");
    
    let decision = engine.decide(&ctx).await;
    
    // Score should be between persona and LLM scores
    // (LLM might fail in test, so just check it's valid)
    assert!(decision.score <= 100);
    assert!(decision.interest_multiplier > 0.0);
}
```

---

### Phase 7: Documentation & Polish (Day 10)
**Goal:** Document the new architecture and ensure clean code

#### 7.1 Update Module Documentation
```rust
//! Decision engine system for Twitter activity task.
//! 
//! This module provides a pluggable decision engine architecture
//! for determining engagement actions on tweets.
//! 
//! ## Architecture
//! 
//! The system uses the Strategy pattern with a core `DecisionEngine` trait
//! that can be implemented by different decision strategies:
//! 
//! - `PersonaEngine`: Rule-based decisions using persona weights
//! - `LLMEngine`: LLM-based smart decisions with context awareness  
//! - `HybridEngine`: Ensemble combining multiple strategies
//! 
//! ## Usage
//! 
//! ```rust
//! let engine = DecisionEngineFactory::create(&task_config);
//! let decision = engine.decide(&tweet_context).await;
//! ```

pub mod twitteractivity_decision;
pub mod twitteractivity_decision_persona;
pub mod twitteractivity_decision_llm;
pub mod twitteractivity_decision_hybrid;

pub use twitteractivity_decision::{DecisionEngine, TweetContext, EngagementDecision, DecisionStrategy};
pub use twitteractivity_decision_persona::PersonaEngine;
pub use twitteractivity_decision_llm::LLMEngine;
pub use twitteractivity_decision_hybrid::HybridEngine;
```

#### 7.2 Update Main Documentation
Update `src/task/twitteractivity.md` with new configuration options.

---

## File Changes Summary

### New Files
| File | Lines | Purpose |
|------|-------|---------|
| `twitteractivity_decision_persona.rs` | ~150 | Rule-based engine |
| `twitteractivity_decision_llm.rs` | ~120 | LLM-based engine |
| `twitteractivity_decision_hybrid.rs` | ~180 | Ensemble engine |

### Files to Delete
| File | Reason |
|------|--------|
| `twitteractivity_sentiment.rs` | Replaced by LLM inference |
| `twitteractivity_sentiment_enhanced.rs` | Replaced by LLM inference |

### Modified Files
| File | Changes | Purpose |
|------|---------|---------|
| `twitteractivity_decision.rs` | +80 | Add trait and factory |
| `twitteractivity_state.rs` | +30, -20 | Add `DecisionStrategy` enum, remove sentiment fields |
| `twitteractivity_engagement.rs` | ~-20, +15 | Use new engine, remove sentiment calls |
| `twitteractivity.rs` (tests) | ~+50 | Update test cases |
| `mod.rs` | +4, -2 | Re-export new modules, remove sentiment re-exports |
| `twitteractivity_feed.rs` | +15 | Add `extract_replies_for_llm()` function |

---

## Testing Strategy

### Unit Tests (Priority 1)
- [ ] `PersonaEngine::decide()` with various tweet types
- [ ] `LLMEngine` fallback behavior
- [ ] `HybridEngine` weighted combination
- [ ] `DecisionEngineFactory` creates correct types
- [ ] Backward compatibility with legacy payloads

### Integration Tests (Priority 2)
- [ ] End-to-end with `process_candidate()`
- [ ] Factory integration with real config
- [ ] Engine switching at runtime

### Property Tests (Priority 3)
- [ ] Decision scores always in valid range (0-100)
- [ ] Multipliers always positive
- [ ] Confidence always in range [0, 1]

---

## Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Backward compatibility break | Medium | High | Keep legacy flag support, add deprecation warning |
| Performance regression | Low | Medium | Add benchmarks, engines are lazy-initialized |
| LLM timeout issues | Medium | Medium | Add timeouts, fallback to PersonaEngine |
| Trait object overhead | Low | Low | Engines created once per task, not per-tweet |
| Testing complexity | Medium | Low | Mock engines for unit tests |

---

## Success Criteria

### Must Have
- [ ] All existing tests pass without modification (backward compatibility)
- [ ] New `DecisionStrategy` enum works in payload
- [ ] `PersonaEngine` produces identical results to old `decide_engagement()`
- [ ] `DecisionEngineFactory` correctly routes to engines
- [ ] No compilation warnings or errors

### Should Have
- [ ] `LLMEngine` has graceful fallback when API unavailable
- [ ] `HybridEngine` properly weights multiple strategies
- [ ] Documentation updated with examples
- [ ] Performance within 5% of original

### Nice to Have
- [ ] Metrics for engine usage and performance
- [ ] A/B testing capability between engines
- [ ] Runtime engine switching without restart

---

## Validation Commands

```bash
# 1. Check compilation
cd "C:\My Script\auto-rust"
cargo check

# 2. Run decision-related tests
cargo test decision

# 3. Run all tests
cargo test --lib

# 4. Check for warnings
cargo clippy --package auto --lib

# 5. Full validation
.\check.ps1
```

---

## Post-Implementation Notes

### For Future PRs
1. **Engine Registry**: Consider a registry pattern for dynamic engine discovery
2. **Metrics**: Add counters for engine usage, decision scores, fallback rates
3. **Caching**: Cache LLM decisions for identical tweets
4. **A/B Testing**: Add experiment framework for testing new engines

### Deprecation Timeline
- **Phase 1 (Now)**: Support both old and new config formats
- **Phase 2 (v0.9)**: Add deprecation warning for `smart_decision_enabled`
- **Phase 3 (v1.0)**: Remove legacy boolean flags

---

*Plan Version: 1.0*
*Last Updated: May 2, 2026*
*Estimated Effort: 10 days*
