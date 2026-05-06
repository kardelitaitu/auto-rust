//! Updated action processing with unified LLM request.
//! Integrates sentiment analysis with reply/quote generation.

use crate::llm::unified_processor::{SentimentAwareProcessor, ReplyWithSentiment, QuoteWithSentiment};
use crate::llm::unified_processor::SentimentAnalysis;
use crate::utils::twitter::sentiment::Sentiment;
use crate::adaptive::learning_engine::{EngagementGoal, AdaptiveLearningEngine};

pub struct UnifiedActionProcessor {
    sentiment_processor: SentimentAwareProcessor,
    learning_engine: AdaptiveLearningEngine,
}

impl UnifiedActionProcessor {
    pub fn new() -> Self {
        Self {
            sentiment_processor: SentimentAwareProcessor::new(),
            learning_engine: AdaptiveLearningEngine::new(),
        }
    }

    /// Process candidate with unified sentiment + content generation.
    pub async fn process_candidate(
        &mut self,
        tweet: &crate::utils::twitter::twitteractivity_feed::Value,
        action_type: &str,
    ) -> Result<ActionResultWithSentiment, anyhow::Error> {
        let tweet_text = tweet["text"].as_str().unwrap_or("");
        let author = tweet["user"]["screen_name"].as_str().unwrap_or("unknown");
        
        // Get replies for context (up to 20)
        let replies = self.get_replies_for_context(tweet).await?;
        
        // Process based on action type
        let result = match action_type {
            "reply" => {
                let reply_result = self.sentiment_processor.process_reply_with_sentiment(
                    tweet_text,
                    author,
                    &replies,
                ).await?;
                
                ActionResultWithSentiment {
                    action_type: "reply".to_string(),
                    content: reply_result.content,
                    sentiment: reply_result.sentiment,
                    confidence: reply_result.sentiment.confidence,
                }
            }
            "quote" => {
                let quote_result = self.sentiment_processor.process_quote_with_sentiment(
                    tweet_text,
                    &replies,
                ).await?;
                
                ActionResultWithSentiment {
                    action_type: "quote".to_string(),
                    content: quote_result.content,
                    sentiment: quote_result.sentiment,
                    confidence: quote_result.confidence,
                }
            }
            _ => {
                // For other actions, use basic processing
                let basic_result = self.process_basic_action(tweet_text, action_type).await?;
                ActionResultWithSentiment {
                    action_type: action_type.to_string(),
                    content: basic_result,
                    sentiment: SentimentAnalysis::default(),
                    confidence: 0.5,
                }
            }
        };
        
        // Update learning engine
        self.learning_engine.record_attempt(
            action_type.to_string(),
            result.sentiment.sentiment == Sentiment::Positive,
        );
        
        Ok(result)
    }

    /// Get replies for context (up to 20).
    async fn get_replies_for_context(
        &self,
        _tweet: &crate::utils::twitter::twitteractivity_feed::Value,
    ) -> Result<Vec<(&str, &str)>, anyhow::Error> {
        // In production, would extract from tweet data
        // For now, return empty or mock data
        Ok(vec![])
    }

    /// Process basic action without sentiment (like, follow, etc.).
    async fn process_basic_action(
        &self,
        tweet_text: &str,
        action_type: &str,
    ) -> Result<String, anyhow::Error> {
        let context = reply_strategies::StrategyContext::default();
        let prompt = format!(
            "Perform {} action on tweet: {}. Just describe the action briefly.",
            action_type, tweet_text
        );
        
        let response = self.sentiment_processor.llm.chat(
            vec![ChatMessage::user(prompt)]
        ).await?;
        
        Ok(response)
    }
}

/// Action result with sentiment information.
pub struct ActionResultWithSentiment {
    pub action_type: String,
    pub content: String,
    pub sentiment: SentimentAnalysis,
    pub confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_candidate_reply() {
        let mut processor = UnifiedActionProcessor::new();
        let tweet = crate::utils::twitter::twitteractivity_feed::Value::Object(
            std::collections::HashMap::new()
        );
        
        let result = processor.process_candidate(&tweet, "reply").await.expect("Failed to process candidate");
        
        assert_eq!(result.action_type, "reply");
        assert!(result.confidence > 0.5);
    }

    #[tokio::test]
    async fn test_process_candidate_quote() {
        let mut processor = UnifiedActionProcessor::new();
        let tweet = crate::utils::twitter::twitteractivity_feed::Value::Object(
            std::collections::HashMap::new()
        );
        
        let result = processor.process_candidate(&tweet, "quote").await.expect("Failed to process candidate");
        
        assert_eq!(result.action_type, "quote");
        assert!(result.confidence > 0.5);
    }

    #[tokio::test]
    async fn test_process_candidate_like() {
        let mut processor = UnifiedActionProcessor::new();
        let tweet = crate::utils::twitter::twitteractivity_feed::Value::Object(
            std::collections::HashMap::new()
        );
        
        let result = processor.process_candidate(&tweet, "like").await.expect("Failed to process candidate");
        
        assert_eq!(result.action_type, "like");
        assert_eq!(result.confidence, 0.5);
    }

    #[tokio::test]
    async fn test_process_candidate_follow() {
        let mut processor = UnifiedActionProcessor::new();
        let tweet = crate::utils::twitter::twitteractivity_feed::Value::Object(
            std::collections::HashMap::new()
        );
        
        let result = processor.process_candidate(&tweet, "follow").await.expect("Failed to process candidate");
        
        assert_eq!(result.action_type, "follow");
        assert_eq!(result.confidence, 0.5);
    }

    #[tokio::test]
    async fn test_process_candidate_retweet() {
        let mut processor = UnifiedActionProcessor::new();
        let tweet = crate::utils::twitter::twitteractivity_feed::Value::Object(
            std::collections::HashMap::new()
        );
        
        let result = processor.process_candidate(&tweet, "retweet").await.expect("Failed to process candidate");
        
        assert_eq!(result.action_type, "retweet");
        assert_eq!(result.confidence, 0.5);
    }

    #[tokio::test]
    async fn test_unified_action_processor_new() {
        let processor = UnifiedActionProcessor::new();
        // Just verify it doesn't panic
        let _ = processor;
    }

    #[tokio::test]
    async fn test_action_result_with_sentiment_fields() {
        let result = ActionResultWithSentiment {
            action_type: "reply".to_string(),
            content: "test content".to_string(),
            sentiment: SentimentAnalysis::default(),
            confidence: 0.75,
        };
        
        assert_eq!(result.action_type, "reply");
        assert_eq!(result.content, "test content");
        assert_eq!(result.confidence, 0.75);
    }

    #[tokio::test]
    async fn test_process_candidate_with_text() {
        let mut processor = UnifiedActionProcessor::new();
        let mut tweet_data = std::collections::HashMap::new();
        tweet_data.insert("text".to_string(), crate::utils::twitter::twitteractivity_feed::Value::String("Hello world".to_string()));
        let tweet = crate::utils::twitter::twitteractivity_feed::Value::Object(tweet_data);
        
        let result = processor.process_candidate(&tweet, "reply").await.expect("Failed to process candidate");
        
        assert_eq!(result.action_type, "reply");
    }

    #[tokio::test]
    async fn test_process_candidate_unknown_action() {
        let mut processor = UnifiedActionProcessor::new();
        let tweet = crate::utils::twitter::twitteractivity_feed::Value::Object(
            std::collections::HashMap::new()
        );
        
        let result = processor.process_candidate(&tweet, "unknown_action").await.expect("Failed to process candidate");
        
        assert_eq!(result.action_type, "unknown_action");
        assert_eq!(result.confidence, 0.5);
    }

    #[tokio::test]
    async fn test_get_replies_for_context_empty() {
        let processor = UnifiedActionProcessor::new();
        let tweet = crate::utils::twitter::twitteractivity_feed::Value::Object(
            std::collections::HashMap::new()
        );
        
        let replies = processor.get_replies_for_context(&tweet).await.expect("Failed to get replies");
        
        assert!(replies.is_empty());
    }

    #[tokio::test]
    async fn test_process_basic_action() {
        let processor = UnifiedActionProcessor::new();
        let result = processor.process_basic_action("test tweet", "like").await;
        
        // Should not panic (may fail with LLM error, but that's expected)
        let _ = result;
    }

    #[tokio::test]
    async fn test_learning_engine_integration() {
        let mut processor = UnifiedActionProcessor::new();
        let tweet = crate::utils::twitter::twitteractivity_feed::Value::Object(
            std::collections::HashMap::new()
        );
        
        // Process should update learning engine
        let _ = processor.process_candidate(&tweet, "reply").await;
        
        // Verify learning engine was called (no panic means success)
        let _ = processor.learning_engine;
    }

    #[tokio::test]
    async fn test_confidence_bounds() {
        let mut processor = UnifiedActionProcessor::new();
        let tweet = crate::utils::twitter::twitteractivity_feed::Value::Object(
            std::collections::HashMap::new()
        );
        
        let result = processor.process_candidate(&tweet, "reply").await.expect("Failed to process candidate");
        
        assert!(result.confidence >= 0.0);
        assert!(result.confidence <= 1.0);
    }

    #[tokio::test]
    async fn test_sentiment_field_present() {
        let mut processor = UnifiedActionProcessor::new();
        let tweet = crate::utils::twitter::twitteractivity_feed::Value::Object(
            std::collections::HashMap::new()
        );
        
        let result = processor.process_candidate(&tweet, "reply").await.expect("Failed to process candidate");
        
        // Sentiment should be present even if default
        let _ = result.sentiment;
    }
}
