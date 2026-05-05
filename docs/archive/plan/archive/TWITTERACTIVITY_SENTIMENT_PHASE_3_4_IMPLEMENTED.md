# TwitterActivity Sentiment Phase 3 & 4 Implemented

**Status:** Implemented  
**Total Estimated Effort:** 5-7 hours (Phase 3: 2-3h, Phase 4: 3-4h)

---

## Phase 3: Domain-Specific Keywords (2-3 hours)

### Overview

Add domain-aware sentiment analysis that recognizes industry-specific terminology for Tech Twitter, Crypto Twitter, and Gaming communities. This improves accuracy for specialized content.

---

### 3.1 File Structure

```
src/utils/twitter/
├── twitteractivity_sentiment_domains.rs  (NEW - ~400 lines)
└── twitteractivity_sentiment.rs          (MODIFIED - add domain integration)
```

---

### 3.2 Domain Types

```rust
// src/utils/twitter/twitteractivity_sentiment_domains.rs

use serde::{Deserialize, Serialize};

/// Domain types for sentiment analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SentimentDomain {
    General,    // Default, broad keywords
    Tech,       // Software, startups, development
    Crypto,     // Cryptocurrency, blockchain, DeFi
    Gaming,     // Video games, streaming, esports
    Sports,     // Athletic events, teams, players
    Entertainment, // Movies, TV, music, celebrities
}
```

---

### 3.3 Keyword Sets

#### Tech Twitter Keywords (~80 terms)

```rust
const TECH_POSITIVE: &[&str] = &[
    // Shipping/Launch
    "shipping", "shipped", "launched", "deployed", "released",
    "going live", "production", "live in prod",
    
    // Code Quality
    "clean code", "elegant solution", "beautiful code", "well designed",
    "refactored", "optimized", "performance boost", "scalable",
    
    // Development Wins
    "tests passing", "ci green", "build passing", "no bugs",
    "merged", "pr approved", "code review passed",
    "feature complete", "milestone reached",
    
    // Technology Positive
    "upgrade", "migration successful", "new stack", "modern",
    "best practice", "solid architecture", "robust",
    "documentation", "well documented", "great dx", "developer experience",
    
    // Business/Startup
    "funding", "series a", "series b", "ipo", "acquisition",
    "partnership", "customer win", "growth", "traction",
    "product market fit", "pmf", "revenue", "profitable",
    
    // Community
    "open source", "oss", "contributor", "maintainer",
    "community driven", "collaboration", "teamwork",
];

const TECH_NEGATIVE: &[&str] = &[
    // Problems/Issues
    "bug", "regression", "outage", "downtime", "incident",
    "production issue", "hotfix", "firefighting", "on-call",
    
    // Code Quality Issues
    "technical debt", "tech debt", "spaghetti code", "hack",
    "workaround", "kludge", "brittle", "fragile", "legacy",
    
    // Development Blockers
    "merge conflict", "ci failed", "build broken", "tests failing",
    "blocking issue", "showstopper", "critical bug",
    
    // Business/Startup Negative
    "layoffs", "shutdown", "pivot", "failed", "bankrupt",
    "burnout", "toxic culture", "bad management",
    
    // Technology Negative
    "deprecated", "end of life", "eol", "sunset",
    "breaking change", "migration hell", "vendor lock-in",
    
    // Security
    "vulnerability", "security issue", "data breach", "exploit",
    "zero-day", "patch tuesday", "CVE",
];
```

#### Crypto Twitter Keywords (~80 terms)

```rust
const CRYPTO_POSITIVE: &[&str] = &[
    // Price/Gains
    "moon", "to the moon", "pump", "gains", "green candle",
    "ath", "all time high", "breakout", "bullish", "bull run",
    
    // Holding Strategy
    "hodl", "diamond hands", "holding strong", "not selling",
    "accumulation", "accumulating", "buying the dip", "dca",
    
    // Project Development
    "mainnet", "testnet", "upgrade", "hard fork", "soft fork",
    "partnership", "integration", "listing", "exchange listing",
    "adoption", "mass adoption", "institutional adoption",
    
    // Technology
    "staking", "yield farming", "defi", "liquidity", "tvl",
    "governance", "dao", "decentralized", "non-custodial",
    
    // Community
    "community", "holders", "diamond hands", "strong hands",
    "telegram", "discord", "twitter army", "shilling",
    
    // Positive News
    "announcement", "ama", "roadmap", "milestone", "delivering",
    "undervalued", "gem", "hidden gem", "100x", "1000x",
];

const CRYPTO_NEGATIVE: &[&str] = &[
    // Price/Losses
    "rekt", "dump", "crash", "bearish", "bear market",
    "red candle", "loss", "liquidation", "margin call",
    "bag holder", "holding bags", "down bad", "ape'd in",
    
    // Scams/Security
    "rug pull", "scam", "exit scam", "honeypot", "fake project",
    "exploit", "hack", "stolen", "drained", "compromised",
    "phishing", "social engineering", "private keys",
    
    // Project Issues
    "delayed", "postponed", "no delivery", "vaporware",
    "empty promises", "overpromised", "fud", "uncertainty",
    "team sold", "dev dumped", "insider selling",
    
    // Market Sentiment
    "capitulation", "blood in streets", "crypto winter",
    "dead project", "ghost town", "abandoned",
    
    // Regulatory
    "ban", "regulation", "sec lawsuit", "investigation",
    "crackdown", "illegal", "compliance issue",
];
```

#### Gaming Keywords (~60 terms)

```rust
const GAMING_POSITIVE: &[&str] = &[
    // Wins/Achievements
    "epic win", "victory", "clutch", "pentakill", "ace",
    "legendary", "mvp", "potg", "play of the game",
    "achievement unlocked", "trophy", "platinum", "100%",
    
    // Items/Drops
    "legendary drop", "rare drop", "loot", "epic loot",
    "gacha luck", "pull rate", "5 star", "shiny",
    
    // Progress
    "level up", "rank up", "promotion", "climbing",
    "speedrun", "pb", "personal best", "world record",
    "no hit run", "perfect run", "flawless",
    
    // Games/Events
    "game of the year", "goty", "masterpiece", "must play",
    "dlc", "expansion", "season pass", "battle pass",
    "tournament", "championship", "worlds", "majors",
    
    // Streaming
    "twitch", "youtube gaming", "content creator",
    "streamer", "subscriber", "donation", "hype",
];

const GAMING_NEGATIVE: &[&str] = &[
    // Losses/Failures
    "game over", "wipe", "defeat", "loss streak",
    "throw", "thrown game", "feeder", "inting",
    
    // Technical Issues
    "lag", "disconnect", "dc", "server down", "maintenance",
    "bug", "glitch", "exploit", "cheater", "hacker",
    "fps drop", "crash", "optimization issue",
    
    // Game Design
    "nerf", "nerfed", "too weak", "unbalanced",
    "pay to win", "p2w", "microtransactions", "loot box",
    "grind", "grindy", "repetitive", "boring",
    
    // Community
    "toxic", "griefer", "griefing", "trolling",
    "smurf", "smurfing", "boosting", "scripting",
];
```

---

### 3.4 Domain Detection Algorithm

```rust
/// Detect domain from tweet content using keyword scoring.
pub fn detect_domain(text: &str) -> SentimentDomain {
    let lower = text.to_lowercase();
    
    // Score each domain based on indicator keywords
    let crypto_indicators = ["btc", "eth", "crypto", "bitcoin", "ethereum", 
        "defi", "nft", "hodl", "blockchain", "altcoin", "token"];
    let tech_indicators = ["code", "dev", "programming", "software", 
        "github", "pr", "merge", "deploy", "shipping", "startup"];
    let gaming_indicators = ["gaming", "game", "twitch", "esports", 
        "streamer", "lol", "valorant", "fortnite", "steam"];
    let sports_indicators = ["nfl", "nba", "mlb", "soccer", "football",
        "basketball", "baseball", "touchdown", "goal"];
    let entertainment_indicators = ["movie", "film", "netflix", "tv show",
        "album", "concert", "celebrity", "oscar"];
    
    let crypto_score = crypto_indicators.iter()
        .filter(|&&w| lower.contains(w)).count();
    let tech_score = tech_indicators.iter()
        .filter(|&&w| lower.contains(w)).count();
    let gaming_score = gaming_indicators.iter()
        .filter(|&&w| lower.contains(w)).count();
    let sports_score = sports_indicators.iter()
        .filter(|&&w| lower.contains(w)).count();
    let entertainment_score = entertainment_indicators.iter()
        .filter(|&&w| lower.contains(w)).count();
    
    // Find highest scoring domain
    let mut scores = vec![
        (SentimentDomain::Crypto, crypto_score),
        (SentimentDomain::Tech, tech_score),
        (SentimentDomain::Gaming, gaming_score),
        (SentimentDomain::Sports, sports_score),
        (SentimentDomain::Entertainment, entertainment_score),
    ];
    
    scores.sort_by(|a, b| b.1.cmp(&a.1));
    
    // Return highest if it has at least 2 indicators, otherwise General
    if scores[0].1 >= 2 {
        scores[0].0
    } else {
        SentimentDomain::General
    }
}
```

---

### 3.5 Domain-Aware Sentiment Analysis

```rust
/// Analyze sentiment with domain-specific keywords.
///
/// # Arguments
/// * `text` - The text to analyze
/// * `domain` - The domain to use for keyword matching
///
/// # Returns
/// Sentiment score (positive or negative contribution)
pub fn analyze_domain_sentiment(text: &str, domain: SentimentDomain) -> f32 {
    let lower = text.to_lowercase();
    let mut score = 0.0;
    
    // Get domain-specific keyword sets
    let (positive, negative) = match domain {
        SentimentDomain::Tech => (TECH_POSITIVE, TECH_NEGATIVE),
        SentimentDomain::Crypto => (CRYPTO_POSITIVE, CRYPTO_NEGATIVE),
        SentimentDomain::Gaming => (GAMING_POSITIVE, GAMING_NEGATIVE),
        SentimentDomain::Sports => (SPORTS_POSITIVE, SPORTS_NEGATIVE),
        SentimentDomain::Entertainment => (ENT_POSITIVE, ENT_NEGATIVE),
        SentimentDomain::General => (&[], &[]),
    };
    
    // Score positive keywords (weighted 1.5x for domain specificity)
    for &word in positive {
        if lower.contains(word) {
            score += 1.5;
        }
    }
    
    // Score negative keywords
    for &word in negative {
        if lower.contains(word) {
            score -= 1.5;
        }
    }
    
    score
}

/// Hybrid analysis: combines general + domain-specific sentiment.
pub fn analyze_sentiment_with_domain(text: &str) -> Sentiment {
    // Detect domain from content
    let domain = detect_domain(text);
    
    // Get base sentiment from general analysis
    let base_sentiment = analyze_sentiment(text);
    let mut base_score = match base_sentiment {
        Sentiment::Positive => 1.0,
        Sentiment::Neutral => 0.0,
        Sentiment::Negative => -1.0,
    };
    
    // Add domain-specific contribution
    let domain_score = analyze_domain_sentiment(text, domain);
    
    // Combine scores
    let final_score = base_score + (domain_score / 10.0); // Normalize domain contribution
    
    // Classify with hysteresis
    if final_score > 1.0 {
        Sentiment::Positive
    } else if final_score < -1.0 {
        Sentiment::Negative
    } else {
        Sentiment::Neutral
    }
}
```

---

### 3.6 Configuration Options

```toml
# In task payload or config.toml
[sentiment]
# Enable domain-specific analysis
domain_enabled = true

# Force specific domain (optional, auto-detect if not set)
# domain_override = "tech"  # or "crypto", "gaming", etc.

# Domain weighting (how much domain keywords affect final score)
domain_weight = 0.3  # 0.0-1.0, default 0.3
```

---

### 3.7 Test Cases

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tech_domain_detection() {
        assert_eq!(
            detect_domain("Just shipped a new feature! #coding #dev"),
            SentimentDomain::Tech
        );
        assert_eq!(
            detect_domain("CI is green, PR approved, merging now"),
            SentimentDomain::Tech
        );
    }

    #[test]
    fn test_crypto_domain_detection() {
        assert_eq!(
            detect_domain("BTC to the moon! 🚀 Hodl strong!"),
            SentimentDomain::Crypto
        );
        assert_eq!(
            detect_domain("New DeFi protocol launching on Ethereum"),
            SentimentDomain::Crypto
        );
    }

    #[test]
    fn test_gaming_domain_detection() {
        assert_eq!(
            detect_domain("Epic pentakill in ranked! Climbing to Diamond"),
            SentimentDomain::Gaming
        );
    }

    #[test]
    fn test_domain_sentiment_tech() {
        // Positive tech sentiment
        let score = analyze_domain_sentiment(
            "Just shipped a new feature! Tests passing, CI green!",
            SentimentDomain::Tech
        );
        assert!(score > 0.0);
        
        // Negative tech sentiment
        let score = analyze_domain_sentiment(
            "Production outage, firefighting all night. Technical debt is killing us.",
            SentimentDomain::Tech
        );
        assert!(score < 0.0);
    }

    #[test]
    fn test_domain_sentiment_crypto() {
        let score = analyze_domain_sentiment(
            "BTC pumping! Diamond hands paying off. To the moon!",
            SentimentDomain::Crypto
        );
        assert!(score > 0.0);
        
        let score = analyze_domain_sentiment(
            "Got rekt. Liquidated. Rug pulled again.",
            SentimentDomain::Crypto
        );
        assert!(score < 0.0);
    }

    #[test]
    fn test_hybrid_analysis() {
        // General positive + tech positive = stronger positive
        let sentiment = analyze_sentiment_with_domain(
            "Amazing! Just shipped a new feature! 🎉"
        );
        assert_eq!(sentiment, Sentiment::Positive);
    }
}
```

---

## Phase 4: LLM Integration (3-4 hours)

### Overview

Integrate LLM-based sentiment analysis as an optional enhancement. When enabled, a percentage of tweets will be analyzed using an LLM (Ollama/OpenRouter) for more nuanced understanding, with fallback to keyword analysis.

---

### 4.1 File Structure

```
src/utils/twitter/
├── twitteractivity_sentiment_llm.rs  (NEW - ~250 lines)
└── twitteractivity_sentiment.rs      (MODIFIED - add hybrid analysis)
```

---

### 4.2 LLM Sentiment Request/Response

```rust
// src/utils/twitter/twitteractivity_sentiment_llm.rs

use crate::llm::client::LlmClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// LLM sentiment analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSentimentResult {
    pub sentiment: String,  // "positive", "negative", or "neutral"
    pub confidence: f32,    // 0.0-1.0
    pub reasoning: Option<String>,  // Brief explanation
}

/// Prompt template for sentiment analysis.
const SENTIMENT_PROMPT: &str = r#"Analyze the sentiment of this tweet and respond with JSON ONLY.

Tweet: "{tweet_text}"

Respond with this exact JSON format (no other text):
{{
    "sentiment": "positive" | "negative" | "neutral",
    "confidence": 0.0-1.0,
    "reasoning": "one sentence explanation"
}}

Consider:
- Sarcasm and irony
- Context and nuance
- Emoji sentiment
- Domain-specific language (tech, crypto, gaming)
- Negation and intensifiers"#;

/// Analyze sentiment using LLM.
///
/// # Arguments
/// * `llm` - LLM client instance
/// * `tweet_text` - The tweet text to analyze
///
/// # Returns
/// LlmSentimentResult with sentiment, confidence, and reasoning
pub async fn analyze_sentiment_llm(
    llm: &LlmClient,
    tweet_text: &str,
) -> Result<LlmSentimentResult> {
    // Truncate to avoid token limits
    let truncated = if tweet_text.len() > 500 {
        &tweet_text[..500]
    } else {
        tweet_text
    };
    
    let prompt = SENTIMENT_PROMPT.replace("{tweet_text}", truncated);
    
    // Generate JSON response from LLM
    let response = llm.generate_json(&prompt).await?;
    
    // Parse response
    let sentiment = response
        .get("sentiment")
        .and_then(|v| v.as_str())
        .unwrap_or("neutral")
        .to_string();
    
    let confidence = response
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5) as f32;
    
    let reasoning = response
        .get("reasoning")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    
    Ok(LlmSentimentResult {
        sentiment,
        confidence,
        reasoning,
    })
}

/// Convert LLM sentiment string to our Sentiment enum.
pub fn llm_sentiment_to_enum(llm_sentiment: &str) -> super::Sentiment {
    match llm_sentiment.to_lowercase().as_str() {
        "positive" => super::Sentiment::Positive,
        "negative" => super::Sentiment::Negative,
        _ => super::Sentiment::Neutral,
    }
}
```

---

### 4.3 Hybrid Analysis (LLM + Keyword Fallback)

```rust
/// Hybrid sentiment analysis: uses LLM with probability, fallback to keyword.
///
/// # Arguments
/// * `llm` - Optional LLM client (if None, always uses keyword)
/// * `tweet_text` - The tweet text to analyze
/// * `llm_probability` - Probability of using LLM (0.0-1.0)
/// * `min_confidence` - Minimum LLM confidence to accept result (0.0-1.0)
///
/// # Returns
/// Combined sentiment from LLM (if used and confident) or keyword analysis
pub async fn analyze_sentiment_hybrid(
    llm: Option<&LlmClient>,
    tweet_text: &str,
    llm_probability: f32,
    min_confidence: f32,
) -> Sentiment {
    // Decide whether to use LLM
    let use_llm = llm.is_some() && rand::random::<f32>() < llm_probability;
    
    if use_llm {
        if let Some(llm_client) = llm {
            match analyze_sentiment_llm(llm_client, tweet_text).await {
                Ok(result) if result.confidence >= min_confidence => {
                    // LLM result is confident enough, use it
                    log::info!(
                        "LLM sentiment: {:?} (confidence: {:.2}, reasoning: {})",
                        result.sentiment,
                        result.confidence,
                        result.reasoning.as_deref().unwrap_or("N/A")
                    );
                    return llm_sentiment_to_enum(&result.sentiment);
                }
                Ok(result) => {
                    // LLM result not confident, log and fallback
                    log::debug!(
                        "LLM low confidence ({:.2}), falling back to keyword analysis",
                        result.confidence
                    );
                }
                Err(e) => {
                    // LLM failed, log and fallback
                    log::warn!("LLM sentiment analysis failed: {}, using keyword fallback", e);
                }
            }
        }
    }
    
    // Fallback to keyword-based analysis
    analyze_sentiment(tweet_text)
}
```

---

### 4.4 Configuration

```toml
# In task payload or config.toml
[sentiment]
# LLM-based sentiment (overrides keyword analysis when enabled)
llm_sentiment_enabled = true

# Probability of using LLM for each tweet (0.0-1.0)
# Higher = more accurate but slower and costs more
llm_probability = 0.3  # 30% of tweets use LLM

# Minimum confidence to accept LLM result
llm_min_confidence = 0.7

# LLM provider settings (inherits from main LLM config)
# LLM_PROVIDER=ollama or openrouter
# OLLAMA_MODEL=llama3.2:3b or similar
```

---

### 4.5 Integration with Twitter Activity Task

```rust
// In task/twitteractivity.rs, modify the sentiment analysis call:

use crate::utils::twitter::{analyze_sentiment_hybrid, get_llm_client};

// ... in the main loop ...

// Parse LLM config from payload
let llm_enabled = payload
    .get("llm_enabled")
    .and_then(|v| v.as_bool())
    .unwrap_or(false);
let llm_probability = payload
    .get("llm_probability")
    .and_then(|v| v.as_f64())
    .unwrap_or(0.3);

// Get LLM client (if configured)
let llm_client = if llm_enabled {
    get_llm_client().await.ok()
} else {
    None
};

// Analyze sentiment with hybrid approach
let sentiment = analyze_sentiment_hybrid(
    llm_client.as_ref(),
    tweet_text,
    llm_probability,
    0.7,
).await;
```

---

### 4.6 Performance Optimization

```rust
/// Cache for LLM sentiment results to avoid re-analyzing same text.
use std::collections::HashMap;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;

static SENTIMENT_CACHE: Lazy<RwLock<HashMap<String, Sentiment>>> = Lazy::new(|| {
    RwLock::new(HashMap::with_capacity(100))
});

/// Get sentiment from cache or compute with LLM.
pub async fn analyze_sentiment_cached(
    llm: Option<&LlmClient>,
    tweet_text: &str,
    llm_probability: f32,
    min_confidence: f32,
) -> Sentiment {
    // Check cache first (use first 100 chars as key)
    let cache_key = if tweet_text.len() > 100 {
        tweet_text[..100].to_string()
    } else {
        tweet_text.to_string()
    };
    
    {
        let cache = SENTIMENT_CACHE.read().await;
        if let Some(&sentiment) = cache.get(&cache_key) {
            return sentiment;
        }
    }
    
    // Compute sentiment
    let sentiment = analyze_sentiment_hybrid(
        llm,
        tweet_text,
        llm_probability,
        min_confidence,
    ).await;
    
    // Cache result
    {
        let mut cache = SENTIMENT_CACHE.write().await;
        // Limit cache size
        if cache.len() >= 1000 {
            cache.clear(); // Simple eviction
        }
        cache.insert(cache_key, sentiment);
    }
    
    sentiment
}
```

---

### 4.7 Test Cases

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::client::LlmClient;

    #[tokio::test]
    async fn test_llm_sentiment_positive() {
        // Skip if no LLM configured
        let llm = match LlmClient::new_test_instance() {
            Ok(client) => client,
            Err(_) => return,
        };
        
        let result = analyze_sentiment_llm(&llm, "I love this product! Amazing!").await.unwrap();
        assert_eq!(result.sentiment, "positive");
        assert!(result.confidence > 0.5);
    }

    #[tokio::test]
    async fn test_llm_sentiment_sarcasm() {
        let llm = match LlmClient::new_test_instance() {
            Ok(client) => client,
            Err(_) => return,
        };
        
        // LLM should detect sarcasm better than keyword analysis
        let result = analyze_sentiment_llm(&llm, "oh great, another bug 🙄").await.unwrap();
        assert_eq!(result.sentiment, "negative");
        // Keyword analysis might miss the sarcasm
    }

    #[tokio::test]
    async fn test_hybrid_fallback() {
        // Test with no LLM (should use keyword fallback)
        let sentiment = analyze_sentiment_hybrid(
            None,
            "This is great!",
            1.0,
            0.7,
        ).await;
        assert_eq!(sentiment, Sentiment::Positive);
    }

    #[tokio::test]
    async fn test_hybrid_probability() {
        let llm = match LlmClient::new_test_instance() {
            Ok(client) => client,
            Err(_) => return,
        };
        
        // Run multiple times to test probability
        let mut llm_used = 0;
        for _ in 0..100 {
            let _ = analyze_sentiment_hybrid(
                Some(&llm),
                "Test tweet",
                0.3, // 30% probability
                0.7,
            ).await;
            // In real implementation, we'd track if LLM was called
        }
        // Should use LLM approximately 30 times out of 100
    }
}
```

---

## Integration Checklist

### Phase 3 Integration Steps

1. [ ] Create `twitteractivity_sentiment_domains.rs`
2. [ ] Add domain keyword sets (Tech, Crypto, Gaming, Sports, Entertainment)
3. [ ] Implement `detect_domain()` function
4. [ ] Implement `analyze_domain_sentiment()` function
5. [ ] Modify `analyze_sentiment()` to call domain analysis
6. [ ] Add configuration options to task payload
7. [ ] Write unit tests (10+ tests)
8. [ ] Run `cargo test` and fix any issues

### Phase 4 Integration Steps

1. [ ] Create `twitteractivity_sentiment_llm.rs`
2. [ ] Implement LLM prompt template
3. [ ] Implement `analyze_sentiment_llm()` function
4. [ ] Implement `analyze_sentiment_hybrid()` function
5. [ ] Add caching layer for performance
6. [ ] Modify `twitteractivity.rs` to support LLM config
7. [ ] Add configuration options (probability, min_confidence)
8. [ ] Write unit tests (8+ tests)
9. [ ] Integration test with real LLM (if available)
10. [ ] Run `cargo test` and fix any issues

---

## Success Metrics

| Metric | Phase 3 Target | Phase 4 Target |
|--------|---------------|---------------|
| **Accuracy Improvement** | +15% for domain-specific tweets | +25% for nuanced content |
| **Sarcasm Detection** | No improvement | +40% better than keywords |
| **Performance Overhead** | <1ms per tweet | <50ms average (with caching) |
| **LLM Cost** | N/A | <$0.01 per 100 tweets |
| **Test Coverage** | 10+ tests | 8+ tests |

---

## Risks and Mitigations

| Risk | Phase | Likelihood | Impact | Mitigation |
|------|-------|-----------|--------|------------|
| Domain detection false positives | 3 | Medium | Low | Require 2+ indicators, fallback to General |
| Keyword list maintenance | 3 | High | Low | Community contributions, periodic updates |
| LLM API costs | 4 | Medium | High | Probability-based usage, caching |
| LLM latency | 4 | High | Medium | Async calls, caching, timeout limits |
| LLM unavailable | 4 | Medium | Low | Graceful fallback to keyword analysis |

---

## Timeline

### Week 1: Phase 3
- **Day 1-2:** Implement domain keyword sets and detection
- **Day 3:** Integration and testing
- **Day 4:** Tuning and optimization

### Week 2: Phase 4
- **Day 1-2:** Implement LLM integration
- **Day 3:** Add caching and hybrid logic
- **Day 4:** Testing and documentation

---

**Archived after implementation landed. Current code lives in `src/utils/twitter/twitteractivity_sentiment.rs` and the related sentiment modules.**
