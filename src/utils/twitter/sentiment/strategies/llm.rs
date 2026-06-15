//! LLM-based sentiment analysis strategy.

use crate::llm::client::LlmClient;
use crate::llm::models::ChatMessage;
use crate::utils::twitter::sentiment::Sentiment;
use anyhow::Result;
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

const SENTIMENT_PROMPT: &str = r#"Analyze the sentiment of this tweet and respond with JSON ONLY.
Tweet: "{tweet_text}"
Respond with this exact JSON format:
{
    "sentiment": "positive" | "negative" | "neutral",
    "confidence": 0.0-1.0,
    "reasoning": "one sentence explanation"
}"#;

#[allow(clippy::cast_precision_loss)]
pub async fn analyze_sentiment_llm(llm: &LlmClient, text: &str) -> Result<LlmSentimentResult> {
    let truncated: String = text.chars().take(400).collect();
    let prompt = SENTIMENT_PROMPT.replace("{tweet_text}", &truncated);
    let messages = vec![ChatMessage::user(prompt)];
    let response_text = llm.chat(messages).await?;
    let json_start = response_text.find('{');
    let json_end = response_text.rfind('}');
    let json_str = match (json_start, json_end) {
        (Some(start), Some(end)) if start < end => &response_text[start..=end],
        _ => {
            return Err(anyhow::anyhow!(
                "No valid JSON object found in LLM response: {}",
                &response_text[..response_text.len().min(100)],
            ))
        }
    };
    let result: LlmSentimentResult = serde_json::from_str(json_str)?;
    Ok(result)
}

#[must_use]
pub fn llm_sentiment_to_enum(llm_sentiment: &str) -> Sentiment {
    match llm_sentiment.to_lowercase().as_str() {
        "positive" => Sentiment::Positive,
        "negative" => Sentiment::Negative,
        _ => Sentiment::Neutral,
    }
}

type SentimentCache = Arc<RwLock<HashMap<String, Sentiment>>>;
static SENTIMENT_CACHE: std::sync::LazyLock<SentimentCache> =
    std::sync::LazyLock::new(|| Arc::new(RwLock::new(HashMap::with_capacity(100))));

pub async fn analyze_sentiment_hybrid(
    llm: Option<&LlmClient>,
    text: &str,
    llm_probability: f32,
    min_confidence: f32,
) -> Sentiment {
    let cache_key: String = text.chars().take(100).collect();
    {
        let cache = SENTIMENT_CACHE.read().await;
        if let Some(&sentiment) = cache.get(&cache_key) {
            return sentiment;
        }
    }

    if llm.is_some() && rand::random::<f32>() < llm_probability {
        if let Some(llm_client) = llm {
            if let Ok(result) = analyze_sentiment_llm(llm_client, text).await {
                if result.confidence >= min_confidence {
                    let sentiment = llm_sentiment_to_enum(&result.sentiment);
                    cache_sentiment(cache_key, sentiment).await;
                    return sentiment;
                }
            }
        }
    }

    let sentiment = crate::utils::twitter::sentiment::analyze_sentiment_sync(text);
    cache_sentiment(cache_key, sentiment).await;
    sentiment
}

async fn cache_sentiment(key: String, sentiment: Sentiment) {
    let mut cache = SENTIMENT_CACHE.write().await;
    if cache.len() >= 1000 {
        cache.retain(|_, _| rand::random::<bool>());
    }
    cache.insert(key, sentiment);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // llm_sentiment_to_enum Tests
    // ========================================================================

    #[test]
    fn test_sentiment_positive() {
        assert_eq!(llm_sentiment_to_enum("positive"), Sentiment::Positive);
    }

    #[test]
    fn test_sentiment_negative() {
        assert_eq!(llm_sentiment_to_enum("negative"), Sentiment::Negative);
    }

    #[test]
    fn test_sentiment_neutral() {
        assert_eq!(llm_sentiment_to_enum("neutral"), Sentiment::Neutral);
    }

    #[test]
    fn test_sentiment_case_insensitive() {
        assert_eq!(llm_sentiment_to_enum("Positive"), Sentiment::Positive);
        assert_eq!(llm_sentiment_to_enum("POSITIVE"), Sentiment::Positive);
        assert_eq!(llm_sentiment_to_enum("NEGATIVE"), Sentiment::Negative);
    }

    #[test]
    fn test_sentiment_unknown_defaults_to_neutral() {
        assert_eq!(llm_sentiment_to_enum("unknown value"), Sentiment::Neutral);
        assert_eq!(llm_sentiment_to_enum(""), Sentiment::Neutral);
        assert_eq!(llm_sentiment_to_enum("mixed"), Sentiment::Neutral);
    }

    #[test]
    fn test_sentiment_whitespace() {
        assert_eq!(llm_sentiment_to_enum("  positive  "), Sentiment::Neutral);
    }

    // ========================================================================
    // LlmSentimentResult Tests
    // ========================================================================

    #[test]
    fn test_result_creation() {
        let result = LlmSentimentResult {
            sentiment: "positive".to_string(),
            confidence: 0.9,
            reasoning: Some("clear positive language".to_string()),
        };
        assert_eq!(result.sentiment, "positive");
        assert!((result.confidence - 0.9).abs() < 0.01);
        assert_eq!(result.reasoning.as_deref(), Some("clear positive language"));
    }

    #[test]
    fn test_result_default_confidence() {
        let result = LlmSentimentResult {
            sentiment: "neutral".to_string(),
            confidence: 0.0,
            reasoning: None,
        };
        assert_eq!(result.confidence, 0.0);
        assert!(result.reasoning.is_none());
    }

    #[test]
    fn test_result_serialize() {
        let result = LlmSentimentResult {
            sentiment: "positive".to_string(),
            confidence: 0.85,
            reasoning: None,
        };
        let json = serde_json::to_string(&result).expect("serialize");
        assert!(json.contains("positive"));
        assert!(json.contains("0.85"));
    }

    #[test]
    fn test_result_deserialize() {
        let json = r#"{"sentiment":"negative","confidence":0.3}"#;
        let result: LlmSentimentResult = serde_json::from_str(json).expect("deserialize");
        assert_eq!(result.sentiment, "negative");
        assert!((result.confidence - 0.3).abs() < 0.01);
    }
}

// ============================================================================
// Fuzz-style proptests for LlmSentimentResult deserialization
// ============================================================================

#[cfg(test)]
mod fuzz_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Fuzz: LlmSentimentResult deserialization must never panic.
        /// serde_json returns Err for malformed JSON, which is fine.
        #[test]
        fn fuzz_sentiment_result_deserialize(
            content in any::<String>(),
        ) {
            let _result: Result<LlmSentimentResult, _> = serde_json::from_str(&content);
        }

        /// Fuzz: JSON extraction logic (used in analyze_sentiment_llm) must never panic.
        /// The code does response_text.find('{').unwrap_or(0) and
        /// response_text.rfind('}').unwrap_or(response_text.len()) then slices.
        #[test]
        fn fuzz_json_extraction(
            response_text in any::<String>(),
        ) {
            // Simulate the fixed extraction logic (now safe on all inputs):
            let json_start = response_text.find('{');
            let json_end = response_text.rfind('}');
            match (json_start, json_end) {
                (Some(start), Some(end)) if start < end => {
                    let json_str = &response_text[start..=end];
                    let _ = serde_json::from_str::<LlmSentimentResult>(json_str);
                }
                _ => {
                    // No valid JSON — should not panic
                }
            }
        }

        /// Fuzz: llm_sentiment_to_enum must never panic for any string.
        /// Defaults to Sentiment::Neutral for unrecognized values.
        #[test]
        fn fuzz_sentiment_to_enum(
            sentiment_str in any::<String>(),
        ) {
            let result = llm_sentiment_to_enum(&sentiment_str);
            // Result must always be a valid Sentiment variant
            let variants = [
                crate::utils::twitter::sentiment::Sentiment::Positive,
                crate::utils::twitter::sentiment::Sentiment::Negative,
                crate::utils::twitter::sentiment::Sentiment::Neutral,
            ];
            prop_assert!(variants.contains(&result),
                "llm_sentiment_to_enum('{:?}') returned {:?}, not a valid variant",
                sentiment_str, result);
        }

        /// Fuzz: simulate the full LLM sentiment JSON pipeline with edge-case JSON.
        /// Tests various JSON structures the LLM might return for sentiment.
        #[test]
        fn fuzz_sentiment_json_pipeline(
            sentiment in any::<String>(),
            confidence in any::<f32>(),
        ) {
            let json = format!(
                r#"{{"sentiment":"{}","confidence":{}}}"#,
                sentiment, confidence
            );
            if let Ok(result) = serde_json::from_str::<LlmSentimentResult>(&json) {
                // Downstream processing should also not panic
                let _enum = llm_sentiment_to_enum(&result.sentiment);
                // confidence can be any f32 — the downstream code uses it as-is
                // No assertion needed; we just verify no panic during deserialization/enum conversion
            }
        }

        /// Fuzz: LlmSentimentResult with extra/unexpected fields.
        /// serde by default ignores unknown fields, so this should never fail.
        #[test]
        fn fuzz_sentiment_result_extra_fields(
            sentiment in any::<String>(),
            extra_key in any::<String>(),
            extra_value in any::<String>(),
        ) {
            let json = format!(
                r#"{{"sentiment":"{}","{}":"{}"}}"#,
                sentiment, extra_key, extra_value
            );
            let _ = serde_json::from_str::<LlmSentimentResult>(&json);
        }
    }
}
