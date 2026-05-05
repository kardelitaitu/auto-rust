# Sentiment Analysis Expansion History

**Status:** Superseded by the completed sentiment implementation  
**Priority:** Medium (Enhancement, not blocking)  
**Estimated Effort:** 8-12 hours total across 4 phases  
**Actual Effort:** ~4 hours for Phases 1-2

---

## Executive Summary

This file preserves the original expansion plan. The current codebase now includes the completed sentiment modules, so treat the remaining sections as historical planning notes.

This plan outlines the implementation of enhanced sentiment analysis for Twitter activity automation. The current implementation uses basic keyword matching (~50 positive, ~60 negative words). The enhanced version will add:

1. **Contextual understanding** (negation, sarcasm, intensifiers) ✅ **COMPLETE**
2. **Comprehensive emoji support** (300+ emojis) ✅ **COMPLETE**
3. **Domain-specific keyword sets** (Tech, Crypto, General) 📋 **PENDING**
4. **Optional LLM integration** (when enabled) 📋 **PENDING**

---

## Completion Status

### ✅ Phase 1: Contextual Analysis - COMPLETE

**Files Created:**
- `src/utils/twitter/twitteractivity_sentiment_context.rs` (319 lines)

**Features Implemented:**
- ✅ Negation detection ("not good" → negative)
- ✅ Sarcasm markers ("oh great", "thanks i hate it")
- ✅ Intensifier handling ("very bad" = 1.5x, "fucking amazing" = 2.0x)
- ✅ Contextual score calculation
- ✅ Excessive punctuation detection

**Test Coverage:** 14 tests, all passing

### ✅ Phase 2: Emoji Sentiment Expansion - COMPLETE

**Files Created:**
- `src/utils/twitter/twitteractivity_sentiment_emoji.rs` (386 lines)

**Features Implemented:**
- ✅ 300+ emoji lexicon with sentiment scores (-3.0 to +3.0)
- ✅ Organized by category (faces, hearts, gestures, celebration, animals, food, activities, travel, objects, symbols, weather, medical)
- ✅ Average sentiment calculation
- ✅ Detailed breakdown (positive/negative/neutral counts)
- ✅ Thread-safe lazy initialization using `OnceLock`

**Test Coverage:** 9 tests, all passing

### ✅ Integration - COMPLETE

**Files Modified:**
- `src/utils/twitter/twitteractivity_sentiment.rs` - Enhanced main analysis function
- `src/utils/twitter/mod.rs` - Added module exports

**Integration Features:**
- Emoji sentiment contributes to overall score
- Contextual modifiers applied (sarcasm, excessive punctuation)
- Negation and intensifiers applied to keyword scores
- Hysteresis thresholds (±1.0) to avoid borderline flips

**Test Coverage:** 23 total tests, all passing

---

## Architecture Design

### New Module Structure

```
src/utils/twitter/
├── twitteractivity_sentiment.rs          # Main sentiment analysis
├── twitteractivity_sentiment_context.rs  # NEW: Contextual analysis (negation, sarcasm, intensifiers)
├── twitteractivity_sentiment_emoji.rs    # NEW: Emoji sentiment lexicon
├── twitteractivity_sentiment_domains.rs  # NEW: Domain-specific keywords
└── twitteractivity_sentiment_llm.rs      # NEW: LLM-based sentiment (optional)
```

### Data Flow

```
Tweet Text
    │
    ├─► Preprocessing (normalize, tokenize)
    │
    ├─► Emoji Analysis ─────────────────┐
    │                                    │
    ├─► Domain Keyword Matching ─────────┤
    │                                    │
    ├─► Contextual Analysis ─────────────┼─► Score Aggregation ─► Final Sentiment
    │     (negation, intensifiers)       │
    ├─► LLM Analysis (if enabled) ───────┤
    │                                    │
    └─► Fallback Keyword Matching ───────┘
```

### Configuration

```toml
# In task payload or config.toml
[sentiment]
# Enable contextual analysis (negation, sarcasm, intensifiers)
contextual_enabled = true

# Enable domain-specific keywords
domain_keywords = ["tech", "crypto", "general"]

# Emoji sentiment (always enabled, low performance impact)
emoji_enabled = true

# LLM-based sentiment (overrides keyword analysis when enabled)
llm_sentiment_enabled = false
llm_sentiment_probability = 0.3  # 30% of tweets use LLM
```

---

## Phase 1: Contextual Analysis (Medium Effort - 3-4 hours)

### 1.1 Negation Detection

**Goal:** Detect when positive words are negated ("not good", "never great")

**Implementation:**

```rust
// NEW FILE: src/utils/twitter/twitteractivity_sentiment_context.rs

/// Negation patterns that flip sentiment polarity.
const NEGATION_PATTERNS: &[&str] = &[
    "not", "no", "never", "neither", "nobody", "nothing", "nor",
    "can't", "can't", "couldn't", "shouldn't", "wouldn't", "don't",
    "doesn't", "didn't", "isn't", "aren't", "wasn't", "weren't",
    "without", "lack", "lacking", "absent", "hardly", "barely",
    "scarcely", "little", "few",
];

/// Detect if a positive/negative word is negated.
/// Returns true if negation found within 3 words before the target word.
pub fn is_negated(text: &str, target_word: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    let target_lower = target_word.to_lowercase();
    
    for (i, word) in words.iter().enumerate() {
        if word.to_lowercase() == target_lower {
            // Check 3 words before for negation
            let start = i.saturating_sub(3);
            for j in start..i {
                if NEGATION_PATTERNS.iter().any(|&n| words[j].to_lowercase() == n) {
                    return true;
                }
            }
        }
    }
    false
}
```

**Test Cases:**
```rust
#[test]
fn test_negation_detection() {
    assert!(is_negated("This is not good", "good"));
    assert!(is_negated("I never said it was great", "great"));
    assert!(!is_negated("This is good", "good"));
    assert!(!is_negated("not bad", "bad")); // "not bad" is actually positive
}
```

---

### 1.2 Sarcasm Detection

**Goal:** Identify sarcastic phrases that invert literal meaning

**Implementation:**

```rust
/// Sarcasm markers and patterns.
const SARCASM_PATTERNS: &[&str] = &[
    "oh great", "oh wonderful", "oh perfect", "oh good",
    "sure, because", "yeah right", "as if",
    "thanks, i hate it", "tanks, i hate it",
    "just what i needed", "exactly what i wanted",
    "because that's what i need",
];

/// Detect sarcasm indicators in text.
pub fn has_sarcasm_markers(text: &str) -> bool {
    let lower = text.to_lowercase();
    SARCASM_PATTERNS.iter().any(|&pattern| lower.contains(pattern))
}

/// Exclamation marks in negative context often indicate sarcasm.
pub fn is_excessive_punctuation(text: &str) -> bool {
    let exclamation_count = text.matches('!').count();
    let question_count = text.matches('?').count();
    // Multiple ?! or !? combinations
    text.contains("?!") || text.contains("!?") || exclamation_count > 2 || question_count > 2
}
```

---

### 1.3 Intensifier Handling

**Goal:** Amplify sentiment score based on intensifiers

**Implementation:**

```rust
/// Intensifiers that amplify sentiment (multiplier > 1.0).
const INTENSIFIERS: &[(&str, f32)] = &[
    ("very", 1.5),
    ("really", 1.5),
    ("extremely", 2.0),
    ("incredibly", 2.0),
    ("absolutely", 2.0),
    ("totally", 1.8),
    ("completely", 1.8),
    ("utterly", 2.0),
    ("highly", 1.5),
    ("super", 1.5),
    ("so", 1.3),
    ("quite", 1.2),
    ("rather", 1.2),
    ("pretty", 1.2),
    ("damn", 1.8),
    ("fucking", 2.0),
];

/// Calculate intensifier multiplier for a word.
/// Looks for intensifiers within 2 words before the target.
pub fn get_intensifier_multiplier(text: &str, target_word: &str) -> f32 {
    let words: Vec<&str> = text.split_whitespace().collect();
    let target_lower = target_word.to_lowercase();
    
    for (i, word) in words.iter().enumerate() {
        if word.to_lowercase() == target_lower {
            // Check 2 words before for intensifier
            let start = i.saturating_sub(2);
            for j in start..i {
                if let Some((_, multiplier)) = INTENSIFIERS.iter()
                    .find(|(intensifier, _)| words[j].to_lowercase() == *intensifier)
                {
                    return *multiplier;
                }
            }
        }
    }
    1.0 // No intensifier found
}
```

---

### 1.4 Integration into Main Analysis

```rust
// MODIFIED: src/utils/twitter/twitteractivity_sentiment.rs

pub fn analyze_sentiment(text: &str) -> Sentiment {
    let lower = text.to_ascii_lowercase();
    
    // Initialize score
    let mut score: f32 = 0.0;
    
    // Check for sarcasm first (overrides other signals)
    if has_sarcasm_markers(&lower) {
        return Sentiment::Negative; // Sarcasm is usually negative
    }
    
    // Count positive vs negative with context
    for &word in POSITIVE_WORDS {
        if lower.contains(word) {
            let multiplier = get_intensifier_multiplier(&lower, word);
            if is_negated(&lower, word) {
                score -= 1.0 * multiplier; // Negated positive = negative
            } else {
                score += 1.0 * multiplier;
            }
        }
    }
    
    for &word in NEGATIVE_WORDS {
        if lower.contains(word) {
            let multiplier = get_intensifier_multiplier(&lower, word);
            if is_negated(&lower, word) {
                score += 1.0 * multiplier; // Negated negative = positive
            } else {
                score -= 1.0 * multiplier;
            }
        }
    }
    
    // Classify based on score
    if score > 0.5 {
        Sentiment::Positive
    } else if score < -0.5 {
        Sentiment::Negative
    } else {
        Sentiment::Neutral
    }
}
```

---

## Phase 2: Emoji Sentiment Expansion (Low Effort - 1-2 hours)

### 2.1 Comprehensive Emoji Lexicon

**Implementation:**

```rust
// NEW FILE: src/utils/twitter/twitteractivity_sentiment_emoji.rs

use std::collections::HashMap;

/// Positive emojis with sentiment strength (1.0-3.0).
const POSITIVE_EMOJIS: &[(&str, f32)] = &[
    // Faces - Positive
    ("😊", 2.0), ("😄", 2.5), ("😃", 2.5), ("😁", 2.5), ("😆", 2.5),
    ("😅", 1.5), ("😂", 2.0), ("☺️", 2.0), ("😊", 2.0), ("😍", 3.0),
    ("🥰", 3.0), ("😘", 2.5), ("😗", 2.0), ("😙", 2.0), ("😚", 2.0),
    ("🙂", 1.5), ("🤗", 2.5), ("🤩", 3.0), ("🤔", 0.5), ("🤨", 0.0),
    ("😐", 0.0), ("😑", 0.0), ("😶", 0.0), ("🙄", -1.0), ("😏", -0.5),
    ("😣", -1.5), ("😥", -1.5), ("😮", -0.5), ("🤐", -1.0), ("😯", -0.5),
    ("😪", -1.0), ("😫", -2.0), ("😴", -0.5), ("😌", 1.5), ("😛", 1.5),
    ("😜", 1.5), ("😝", 1.5), ("😋", 1.5), ("🤑", -0.5), ("🤓", 1.0),
    ("😎", 2.0), ("🤡", -1.0), ("🤠", 2.0), ("🥳", 3.0), ("🥺", 0.5),
    // Hearts - Positive
    ("❤️", 3.0), ("🧡", 2.5), ("💛", 2.5), ("💚", 2.5), ("💙", 2.5),
    ("💜", 2.5), ("🖤", 1.0), ("💔", -3.0), ("❣️", 2.5), ("💕", 3.0),
    ("💖", 3.0), ("💗", 3.0), ("💘", 3.0), ("💝", 3.0), ("💞", 3.0),
    ("💟", 2.5), ("💓", 3.0), ("💌", 2.0),
    // Gestures - Positive
    ("👍", 2.0), ("👎", -2.0), ("👏", 2.5), ("🙌", 3.0), ("👐", 1.5),
    ("🤲", 1.5), ("🤝", 2.0), ("🙏", 2.0), ("✌️", 1.5), ("🤟", 2.5),
    ("🤘", 1.5), ("👌", 2.0), ("🤌", 1.0), ("🤏", 0.5), ("👈", 0.0),
    ("👉", 0.0), ("👆", 0.0), ("👇", 0.0), ("☝️", 0.5), ("✊", 1.0),
    // Celebration - Positive
    ("🎉", 3.0), ("🎊", 3.0), ("🎈", 2.5), ("🎁", 2.5), ("🎀", 2.0),
    ("🏆", 3.0), ("🥇", 3.0), ("🥈", 2.5), ("🥉", 2.0), ("🏅", 3.0),
    ("🎯", 2.0), ("🔥", 2.5), ("💯", 3.0), ("✨", 2.5), ("⭐", 2.5),
    ("🌟", 3.0), ("💫", 2.0), ("🌈", 2.5), ("☀️", 2.0), ("🌞", 2.5),
    // Negative
    ("😢", -2.5), ("😭", -3.0), ("😞", -2.0), ("😟", -1.5), ("😠", -2.5),
    ("😡", -3.0), ("🤬", -3.0), ("😤", -2.0), ("😩", -2.5), ("😫", -2.5),
    ("😨", -2.0), ("😰", -2.0), ("😱", -2.5), ("😳", -1.0), ("🥺", -1.0),
    ("😦", -1.5), ("😧", -1.5), ("😨", -1.5), ("😬", -1.0), ("😕", -1.0),
    ("😖", -2.0), ("😗", -1.0), ("😘", -1.0), ("😙", -1.0), ("😚", -1.0),
    ("💀", -2.0), ("☠️", -2.5), ("💩", -3.0), ("🤮", -3.0), ("🤢", -3.0),
    ("🤧", -1.5), ("😷", -1.0), ("🤒", -1.5), ("🤕", -1.5),
    // Neutral/Context-dependent
    ("🤔", 0.0), ("😐", 0.0), ("😑", 0.0), ("😶", 0.0), ("🙄", -0.5),
    ("😏", -0.5), ("😒", -1.0), ("😔", -1.5), ("😕", -1.0),
];

/// Extract emojis from text and calculate sentiment score.
pub fn analyze_emoji_sentiment(text: &str) -> f32 {
    let mut score = 0.0;
    let mut emoji_count = 0;
    
    for ch in text.chars() {
        // Check if character is an emoji
        if let Some((_, emoji_score)) = POSITIVE_EMOJIS.iter()
            .find(|(emoji, _)| ch.to_string() == *emoji)
        {
            score += emoji_score;
            emoji_count += 1;
        }
    }
    
    // Normalize by emoji count, or return 0 if no emojis
    if emoji_count > 0 {
        score / emoji_count as f32
    } else {
        0.0
    }
}

/// Check if text contains emojis.
pub fn has_emojis(text: &str) -> bool {
    text.chars().any(|ch| {
        POSITIVE_EMOJIS.iter().any(|(emoji, _)| ch.to_string() == *emoji)
    })
}
```

---

## Phase 3: Domain-Specific Keywords (Medium Effort - 2-3 hours)

### 3.1 Domain Keyword Sets

**Implementation:**

```rust
// NEW FILE: src/utils/twitter/twitteractivity_sentiment_domains.rs

use serde::{Deserialize, Serialize};

/// Domain types for sentiment analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SentimentDomain {
    General,
    Tech,
    Crypto,
    Gaming,
    Sports,
    Entertainment,
}

/// Domain-specific positive keywords.
const DOMAIN_POSITIVE: &[(&str, SentimentDomain)] = &[
    // Tech
    ("shipping", SentimentDomain::Tech),
    ("launched", SentimentDomain::Tech),
    ("deployed", SentimentDomain::Tech),
    ("released", SentimentDomain::Tech),
    ("feature", SentimentDomain::Tech),
    ("update", SentimentDomain::Tech),
    ("upgrade", SentimentDomain::Tech),
    ("optimized", SentimentDomain::Tech),
    ("refactored", SentimentDomain::Tech),
    ("performance", SentimentDomain::Tech),
    ("scalable", SentimentDomain::Tech),
    ("elegant", SentimentDomain::Tech),
    ("clean code", SentimentDomain::Tech),
    ("tests passing", SentimentDomain::Tech),
    ("ci green", SentimentDomain::Tech),
    ("merged", SentimentDomain::Tech),
    ("pr approved", SentimentDomain::Tech),
    // Crypto
    ("moon", SentimentDomain::Crypto),
    ("hodl", SentimentDomain::Crypto),
    ("diamond hands", SentimentDomain::Crypto),
    ("bullish", SentimentDomain::Crypto),
    ("pump", SentimentDomain::Crypto),
    ("gains", SentimentDomain::Crypto),
    ("ath", SentimentDomain::Crypto),
    ("breakout", SentimentDomain::Crypto),
    ("accumulation", SentimentDomain::Crypto),
    ("adoption", SentimentDomain::Crypto),
    ("partnership", SentimentDomain::Crypto),
    ("mainnet", SentimentDomain::Crypto),
    ("upgrade", SentimentDomain::Crypto),
    ("staking", SentimentDomain::Crypto),
    ("yield", SentimentDomain::Crypto),
    // Gaming
    ("epic win", SentimentDomain::Gaming),
    ("legendary", SentimentDomain::Gaming),
    ("rare drop", SentimentDomain::Gaming),
    ("level up", SentimentDomain::Gaming),
    ("achievement", SentimentDomain::Gaming),
    ("speedrun", SentimentDomain::Gaming),
    ("no hit", SentimentDomain::Gaming),
    ("perfect run", SentimentDomain::Gaming),
];

/// Domain-specific negative keywords.
const DOMAIN_NEGATIVE: &[(&str, SentimentDomain)] = &[
    // Tech
    ("bug", SentimentDomain::Tech),
    ("regression", SentimentDomain::Tech),
    ("outage", SentimentDomain::Tech),
    ("downtime", SentimentDomain::Tech),
    ("breaking change", SentimentDomain::Tech),
    ("deprecated", SentimentDomain::Tech),
    ("legacy", SentimentDomain::Tech),
    ("technical debt", SentimentDomain::Tech),
    ("spaghetti code", SentimentDomain::Tech),
    ("hack", SentimentDomain::Tech),
    ("workaround", SentimentDomain::Tech),
    ("hotfix", SentimentDomain::Tech),
    ("rollback", SentimentDomain::Tech),
    ("ci failed", SentimentDomain::Tech),
    ("tests failing", SentimentDomain::Tech),
    ("merge conflict", SentimentDomain::Tech),
    // Crypto
    ("rekt", SentimentDomain::Crypto),
    ("dump", SentimentDomain::Crypto),
    ("bearish", SentimentDomain::Crypto),
    ("crash", SentimentDomain::Crypto),
    ("rug pull", SentimentDomain::Crypto),
    ("scam", SentimentDomain::Crypto),
    ("fud", SentimentDomain::Crypto),
    ("liquidation", SentimentDomain::Crypto),
    ("bag holder", SentimentDomain::Crypto),
    ("paper hands", SentimentDomain::Crypto),
    ("delayed", SentimentDomain::Crypto),
    ("postponed", SentimentDomain::Crypto),
    ("bug", SentimentDomain::Crypto),
    ("exploit", SentimentDomain::Crypto),
    // Gaming
    ("nerf", SentimentDomain::Gaming),
    ("game over", SentimentDomain::Gaming),
    ("wipe", SentimentDomain::Gaming),
    ("lag", SentimentDomain::Gaming),
    ("disconnect", SentimentDomain::Gaming),
    ("toxic", SentimentDomain::Gaming),
    ("griefer", SentimentDomain::Gaming),
    ("cheater", SentimentDomain::Gaming),
];

/// Analyze sentiment with domain-specific keywords.
pub fn analyze_domain_sentiment(text: &str, domain: SentimentDomain) -> f32 {
    let lower = text.to_lowercase();
    let mut score = 0.0;
    
    // Check domain-specific positive
    for &(word, word_domain) in DOMAIN_POSITIVE {
        if word_domain == domain && lower.contains(word) {
            score += 1.5; // Domain keywords weighted higher
        }
    }
    
    // Check domain-specific negative
    for &(word, word_domain) in DOMAIN_NEGATIVE {
        if word_domain == domain && lower.contains(word) {
            score -= 1.5;
        }
    }
    
    score
}

/// Detect domain from tweet content.
pub fn detect_domain(text: &str) -> SentimentDomain {
    let lower = text.to_lowercase();
    
    // Crypto indicators
    let crypto_score = ["btc", "eth", "crypto", "bitcoin", "ethereum", "defi", "nft"]
        .iter()
        .filter(|&&w| lower.contains(w))
        .count();
    
    // Tech indicators
    let tech_score = ["code", "dev", "programming", "software", "github", "pr", "merge"]
        .iter()
        .filter(|&&w| lower.contains(w))
        .count();
    
    // Gaming indicators
    let gaming_score = ["gaming", "game", "twitch", "esports", "streamer"]
        .iter()
        .filter(|&&w| lower.contains(w))
        .count();
    
    if crypto_score > tech_score && crypto_score > gaming_score {
        SentimentDomain::Crypto
    } else if tech_score > gaming_score {
        SentimentDomain::Tech
    } else if gaming_score > 0 {
        SentimentDomain::Gaming
    } else {
        SentimentDomain::General
    }
}
```

---

## Phase 4: LLM Integration (High Effort - 3-4 hours)

### 4.1 LLM-Based Sentiment

**Implementation:**

```rust
// NEW FILE: src/utils/twitter/twitteractivity_sentiment_llm.rs

use crate::llm::client::LlmClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// LLM sentiment analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSentimentResult {
    pub sentiment: Sentiment,
    pub confidence: f32,
    pub reasoning: Option<String>,
}

/// Analyze sentiment using LLM.
pub async fn analyze_sentiment_llm(
    llm: &LlmClient,
    tweet_text: &str,
) -> Result<LlmSentimentResult> {
    let prompt = format!(
        r#"Analyze the sentiment of this tweet and respond with JSON:

Tweet: "{}"

Respond with this exact JSON format:
{{
    "sentiment": "positive" | "negative" | "neutral",
    "confidence": 0.0-1.0,
    "reasoning": "brief explanation"
}}

Be concise and accurate."#,
        tweet_text
    );
    
    let response = llm.generate_json(&prompt).await?;
    
    // Parse response
    let sentiment = match response.get("sentiment").and_then(|v| v.as_str()) {
        Some("positive") => Sentiment::Positive,
        Some("negative") => Sentiment::Negative,
        _ => Sentiment::Neutral,
    };
    
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

/// Hybrid analysis: use LLM with probability, fallback to keyword analysis.
pub async fn analyze_sentiment_hybrid(
    llm: &LlmClient,
    tweet_text: &str,
    llm_probability: f32,
) -> Sentiment {
    // Decide whether to use LLM
    if rand::random::<f32>() < llm_probability {
        match analyze_sentiment_llm(llm, tweet_text).await {
            Ok(result) if result.confidence > 0.7 => {
                return result.sentiment;
            }
            Ok(_) | Err(_) => {
                // Fallback to keyword analysis
            }
        }
    }
    
    // Fallback to keyword-based analysis
    analyze_sentiment(tweet_text)
}
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_negation_flips_sentiment() {
        assert_eq!(analyze_sentiment("This is good"), Sentiment::Positive);
        assert_eq!(analyze_sentiment("This is not good"), Sentiment::Negative);
    }
    
    #[test]
    fn test_intensifier_amplifies() {
        assert_eq!(analyze_sentiment("good"), Sentiment::Positive);
        // "very good" should have stronger positive score
    }
    
    #[test]
    fn test_sarcasm_detected() {
        assert_eq!(analyze_sentiment("oh great, another bug"), Sentiment::Negative);
    }
    
    #[test]
    fn test_emoji_sentiment() {
        assert!(analyze_emoji_sentiment("I love this! 😍❤️🔥") > 0.0);
        assert!(analyze_emoji_sentiment("This sucks 😢💔😡") < 0.0);
    }
    
    #[test]
    fn test_domain_detection() {
        assert_eq!(
            detect_domain("Just shipped a new feature! #coding"),
            SentimentDomain::Tech
        );
        assert_eq!(
            detect_domain("BTC to the moon! 🚀"),
            SentimentDomain::Crypto
        );
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_llm_sentiment_integration() {
    let llm = LlmClient::new_test_instance();
    let result = analyze_sentiment_llm(&llm, "I love this product!").await.unwrap();
    assert_eq!(result.sentiment, Sentiment::Positive);
    assert!(result.confidence > 0.5);
}
```

---

## Performance Considerations

| Feature | Performance Impact | Mitigation |
|---------|-------------------|------------|
| Negation detection | Low (string scan) | None needed |
| Sarcasm detection | Low (pattern match) | None needed |
| Intensifier detection | Low (string scan) | None needed |
| Emoji analysis | Low-Medium (char iteration) | Pre-compile emoji set |
| Domain keywords | Low (single pass) | Domain detection first |
| LLM integration | **High** (API call) | Probability-based, caching |

---

## Rollout Plan

### Week 1: Core Enhancements
- [ ] Phase 1: Contextual analysis (negation, sarcasm, intensifiers)
- [ ] Phase 2: Emoji expansion
- [ ] Unit tests for Phases 1-2

### Week 2: Advanced Features
- [ ] Phase 3: Domain-specific keywords
- [ ] Phase 4: LLM integration (optional)
- [ ] Integration tests
- [ ] Performance benchmarks

### Week 3: Validation
- [ ] A/B testing with real Twitter sessions
- [ ] Tune thresholds and weights
- [ ] Documentation updates
- [ ] Merge to main

---

## Success Metrics

1. **Accuracy Improvement:** 20% better sentiment classification vs. manual review
2. **Engagement Quality:** Higher reply engagement rate (measured via LLM reply quality)
3. **Performance:** <5ms overhead per tweet for keyword-based analysis
4. **LLM Cost:** <30% of tweets use LLM (controlled by probability)

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| LLM API costs | Medium | High | Probability-based usage, caching |
| Performance regression | Low | Medium | Benchmarking, profiling |
| False positives in sarcasm | Medium | Low | Conservative thresholds |
| Domain detection errors | Medium | Low | Fallback to General domain |

---

## Appendix: Files to Create/Modify

### New Files
1. `src/utils/twitter/twitteractivity_sentiment_context.rs`
2. `src/utils/twitter/twitteractivity_sentiment_emoji.rs`
3. `src/utils/twitter/twitteractivity_sentiment_domains.rs`
4. `src/utils/twitter/twitteractivity_sentiment_llm.rs`

### Modified Files
1. `src/utils/twitter/twitteractivity_sentiment.rs` - Main integration
2. `src/utils/twitter/mod.rs` - Module exports
3. `task/twitteractivity.rs` - Add configuration options
4. `src/utils/twitter/twitteractivity_decision.rs` - Use enhanced sentiment

---

**Ready to begin implementation. Awaiting approval to proceed with Phase 1.**
