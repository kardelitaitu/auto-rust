//! Updated action processing with unified LLM request.
//! Integrates sentiment analysis with reply/quote generation.

use crate::llm::unified_processor::{SentimentAwareProcessor, ReplyWithSentiment, QuoteWithSentiment};
use crate::utils::twitter::twitteractivity_sentiment::{Sentiment, SentimentAnalysis};
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
        
        let result = processor.process_candidate(&tweet, "reply").await.unwrap();
        
        assert_eq!(result.action_type, "reply");
        assert!(result.confidence > 0.5);
    }

    #[tokio::test]
    async fn test_process_candidate_quote() {
        let mut processor = UnifiedActionProcessor::new();
        let tweet = crate::utils::twitter::twitteractivity_feed::Value::Object(
            std::collections::HashMap::new()
        );
        
        let result = processor.process_candidate(&tweet, "quote").await.unwrap();
        
        assert_eq!(result.action_type, "quote");
        assert!(result.confidence > 0.5);
    }
}
