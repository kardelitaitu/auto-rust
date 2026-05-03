//! LLM-based sentiment analysis for Twitter.
//! Provides optional LLM-powered sentiment with keyword fallback.

use crate::llm::client::LlmClient;
use crate::llm::models::ChatMessage;
use crate::utils::twitter::twitteractivity_sentiment::Sentiment;
use anyhow::Result;
use log::{debug, info, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// LLM sentiment analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSentimentResult {
    pub sentiment: String,
    #[serde(default)]
    pub confidence: f32,
    #[serde(default)]
    pub reasoning: Option<String>,
}

/// Prompt template for sentiment analysis.
const SENTIMENT_PROMPT: &str = r#"Analyze the sentiment of this tweet and respond with JSON ONLY.

Tweet: "{tweet_text}"

Respond with this exact JSON format (no other text):
{
    "sentiment": "positive" | "negative" | "neutral",
    "confidence": 0.0-1.0,
    "reasoning": "one sentence explanation"
}

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
    let truncated = if tweet_text.len() > 400 {
        &tweet_text[..400]
    } else {
        tweet_text
    };

    let prompt = SENTIMENT_PROMPT.replace("{tweet_text}", truncated);

    // Create chat message
    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: prompt,
    }];

    // Generate response from LLM
    let response_text = llm.chat(messages).await?;

    // Parse JSON response
    // Try to extract JSON from response (in case there's extra text)
    let json_start = response_text.find('{').unwrap_or(0);
    let json_end = response_text.rfind('}').unwrap_or(response_text.len());
    let json_str = &response_text[json_start..json_end.min(response_text.len())];

    let result: LlmSentimentResult = serde_json::from_str(json_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse LLM response: {}", e))?;

    Ok(result)
}

/// Convert LLM sentiment string to our Sentiment enum.
pub fn llm_sentiment_to_enum(llm_sentiment: &str) -> Sentiment {
    match llm_sentiment.to_lowercase().as_str() {
        "positive" => Sentiment::Positive,
        "negative" => Sentiment::Negative,
        _ => Sentiment::Neutral,
    }
}

/// Cache for LLM sentiment results to avoid re-analyzing same text.
type SentimentCache = Arc<RwLock<HashMap<String, Sentiment>>>;

static SENTIMENT_CACHE: Lazy<SentimentCache> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::with_capacity(100))));

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
    // Check cache first (use first 100 chars as key)
    let cache_key = if tweet_text.len() > 100 {
        tweet_text[..100].to_string()
    } else {
        tweet_text.to_string()
    };

    {
        let cache = SENTIMENT_CACHE.read().await;
        if let Some(&sentiment) = cache.get(&cache_key) {
            debug!("Sentiment cache hit");
            return sentiment;
        }
    }

    // Decide whether to use LLM
    let use_llm = llm.is_some() && rand::random::<f32>() < llm_probability;

    if use_llm {
        if let Some(llm_client) = llm {
            match analyze_sentiment_llm(llm_client, tweet_text).await {
                Ok(result) if result.confidence >= min_confidence => {
                    // LLM result is confident enough, use it
                    info!(
                        "LLM sentiment: {:?} (confidence: {:.2}, reasoning: {})",
                        result.sentiment,
                        result.confidence,
                        result.reasoning.as_deref().unwrap_or("N/A")
                    );
                    let sentiment = llm_sentiment_to_enum(&result.sentiment);

                    // Cache result
                    cache_sentiment(cache_key, sentiment).await;

                    return sentiment;
                }
                Ok(result) => {
                    // LLM result not confident, log and fallback
                    debug!(
                        "LLM low confidence ({:.2}), falling back to keyword analysis",
                        result.confidence
                    );
                }
                Err(e) => {
                    // LLM failed, log and fallback
                    warn!(
                        "LLM sentiment analysis failed: {}, using keyword fallback",
                        e
                    );
                }
            }
        }
    }

    // Fallback to keyword-based analysis
    let sentiment = crate::utils::twitter::analyze_sentiment(tweet_text);

    // Cache result
    cache_sentiment(cache_key, sentiment).await;

    sentiment
}

/// Cache sentiment result for future use.
async fn cache_sentiment(cache_key: String, sentiment: Sentiment) {
    let mut cache = SENTIMENT_CACHE.write().await;
    // Limit cache size
    if cache.len() >= 1000 {
        // Simple eviction: clear 50% of cache
        cache.retain(|_, _| rand::random::<bool>());
    }
    cache.insert(cache_key, sentiment);
}

/// Clear the sentiment cache.
pub async fn clear_sentiment_cache() {
    let mut cache = SENTIMENT_CACHE.write().await;
    cache.clear();
}

/// Get cache statistics.
pub async fn get_cache_stats() -> (usize, usize) {
    let cache = SENTIMENT_CACHE.read().await;
    (cache.len(), 1000) // (current_size, max_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::client::LlmClient;
    use crate::llm::models::{LlmConfig, LlmProvider, OllamaConfig, OpenRouterConfig};

    #[tokio::test]
    async fn test_llm_sentiment_to_enum() {
        assert_eq!(llm_sentiment_to_enum("positive"), Sentiment::Positive);
        assert_eq!(llm_sentiment_to_enum("negative"), Sentiment::Negative);
        assert_eq!(llm_sentiment_to_enum("neutral"), Sentiment::Neutral);
        assert_eq!(llm_sentiment_to_enum("unknown"), Sentiment::Neutral);
    }

    #[tokio::test]
    async fn test_hybrid_fallback_no_llm() {
        // Test with no LLM (should use keyword fallback)
        let sentiment =
            analyze_sentiment_hybrid(None, "This is absolutely amazing! Love it! 😍❤️", 1.0, 0.7)
                .await;
        assert_eq!(sentiment, Sentiment::Positive);
    }

    #[tokio::test]
    async fn test_hybrid_fallback_keyword_analysis() {
        // Test with LLM but 0 probability (should use keyword)
        let config = LlmConfig {
            provider: LlmProvider::Ollama,
            ollama: OllamaConfig {
                base_url: "http://localhost:11434".to_string(),
                model: "llama3.2:3b".to_string(),
                timeout_ms: 30000,
            },
            openrouter: OpenRouterConfig::default(),
        };
        let llm = LlmClient::new(config);

        let sentiment = analyze_sentiment_hybrid(
            Some(&llm),
            "This is amazing! Love it! 😍",
            0.0, // 0% probability = always use keyword
            0.7,
        )
        .await;

        // Should use keyword analysis and return Positive
        assert_eq!(sentiment, Sentiment::Positive);
    }

    #[tokio::test]
    #[ignore = "Flaky: Global cache state causes contention in async test runner"]
    async fn test_cache_basic() {
        let key = "test_cache_key".to_string();
        cache_sentiment(key.clone(), Sentiment::Positive).await;

        let cache = SENTIMENT_CACHE.read().await;
        assert_eq!(cache.get(&key), Some(&Sentiment::Positive));
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let key = "test_cache_key_clear".to_string();
        cache_sentiment(key.clone(), Sentiment::Negative).await;

        clear_sentiment_cache().await;

        let cache = SENTIMENT_CACHE.read().await;
        assert!(!cache.contains_key(&key));
    }
}
