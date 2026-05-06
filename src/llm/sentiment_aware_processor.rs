//! Integration of sentiment analysis with reply/quote generation.
//! Provides unified processing with sentiment-aware content generation.

use crate::llm::models::ChatMessage;
use crate::llm::reply_strategies;
use crate::llm::unified_processor::SentimentAnalysis;
use crate::utils::twitter::sentiment::{Sentiment, analyze_sentiment_sync};

pub struct SentimentAwareProcessor {
    llm: crate::llm::Llm,
}

impl Default for SentimentAwareProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl SentimentAwareProcessor {
    pub fn new() -> Self {
        Self {
            llm: crate::llm::Llm::new().expect("Failed to create LLM"),
        }
    }

    pub fn try_new() -> anyhow::Result<Self> {
        Ok(Self {
            llm: crate::llm::Llm::new()?,
        })
    }

    /// Process reply with sentiment analysis in single request.
    pub async fn process_reply_with_sentiment(
        &self,
        tweet_text: &str,
        author: &str,
        replies: &[(&str, &str)],
    ) -> Result<ReplyWithSentiment, anyhow::Error> {
        // Build prompt with strategy context
        let context = reply_strategies::StrategyContext::default();
        
        // Convert replies to owned format for build_reply_prompt
        let replies_owned: Vec<(String, String)> = replies
            .iter()
            .map(|(a, t)| (a.to_string(), t.to_string()))
            .collect();
        
        let prompt = reply_strategies::build_reply_prompt(
            tweet_text,
            author,
            &replies_owned,
            &context,
        );

        // Single LLM request
        let response = self.llm.chat(vec![ChatMessage::user(prompt)]).await?;

        // Parse response with sentiment
        let (sentiment, content) = self.parse_response_with_sentiment(&response)?;

        Ok(ReplyWithSentiment {
            sentiment,
            content,
        })
    }

    /// Process quote with sentiment analysis in single request.
    pub async fn process_quote_with_sentiment(
        &self,
        tweet_text: &str,
        replies: &[(&str, &str)],
    ) -> Result<QuoteWithSentiment, anyhow::Error> {
        // Build prompt with strategy context
        let _context = reply_strategies::StrategyContext::default();
        
        let messages = reply_strategies::build_quote_messages(
            "author",
            tweet_text,
            replies,
        );

        // Single LLM request
        let response = self.llm.chat(messages).await?;

        // Parse response with sentiment
        let sentiment = self.extract_sentiment(&response)?;
        let content = self.extract_content(&response)?;
        
        // Use actual confidence from sentiment analysis
        let confidence = sentiment.confidence;

        Ok(QuoteWithSentiment {
            sentiment,
            content,
            confidence,
        })
    }

    /// Parse response and extract sentiment.
    fn parse_response_with_sentiment(
        &self,
        response: &str,
    ) -> Result<(SentimentAnalysis, String), anyhow::Error> {
        Self::parse_response_with_sentiment_static(response)
    }

    /// Static version of parse_response_with_sentiment for testing.
    pub fn parse_response_with_sentiment_static(
        response: &str,
    ) -> Result<(SentimentAnalysis, String), anyhow::Error> {
        // Clean the response content
        let content = Self::clean_response_content(response);
        
        // Use actual sentiment analysis on the LLM response
        let sentiment = analyze_sentiment_sync(&content);
        let indicators = Self::extract_sentiment_indicators_static(&content);
        let confidence = Self::calculate_confidence(&content, &indicators);
        
        let sentiment_analysis = SentimentAnalysis {
            sentiment,
            confidence,
            indicators,
        };
        
        Ok((sentiment_analysis, content))
    }

    /// Extract sentiment from quote response.
    fn extract_sentiment(
        &self,
        response: &str,
    ) -> Result<SentimentAnalysis, anyhow::Error> {
        Self::extract_sentiment_static(response)
    }

    /// Static version of extract_sentiment for testing.
    pub fn extract_sentiment_static(
        response: &str,
    ) -> Result<SentimentAnalysis, anyhow::Error> {
        let content = Self::clean_response_content(response);
        let sentiment = analyze_sentiment_sync(&content);
        let indicators = Self::extract_sentiment_indicators_static(&content);
        let confidence = Self::calculate_confidence(&content, &indicators);
        
        Ok(SentimentAnalysis {
            sentiment,
            confidence,
            indicators,
        })
    }

    /// Static version of extract_sentiment_indicators for testing.
    pub fn extract_sentiment_indicators_static(text: &str) -> Vec<String> {
        let lower = text.to_ascii_lowercase();
        let mut indicators = Vec::new();
        
        // Common positive indicators
        if lower.contains("great") || lower.contains("amazing") || lower.contains("excellent") {
            indicators.push("positive_word".to_string());
        }
        if lower.contains("!") {
            indicators.push("exclamation".to_string());
        }
        
        // Common negative indicators
        if lower.contains("bad") || lower.contains("terrible") || lower.contains("awful") {
            indicators.push("negative_word".to_string());
        }
        
        // Question indicators
        if lower.contains("?") {
            indicators.push("question".to_string());
        }
        
        if indicators.is_empty() {
            indicators.push("neutral".to_string());
        }
        
        indicators
    }

    /// Extract content from response.
    fn extract_content(
        &self,
        response: &str,
    ) -> Result<String, anyhow::Error> {
        Ok(Self::clean_response_content(response))
    }

    /// Clean and sanitize response content.
    pub fn clean_response_content(text: &str) -> String {
        text.trim()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '!' || *c == '?' || *c == '.' || *c == ',' || *c == '\'' || *c == '-')
            .collect::<String>()
            .trim()
            .to_string()
    }

    /// Calculate confidence score based on text and indicators.
    pub fn calculate_confidence(text: &str, indicators: &[String]) -> f32 {
        let mut confidence: f32 = 0.5; // Base confidence
        
        // Increase confidence for longer, more substantive content
        if text.len() > 20 {
            confidence += 0.1;
        }
        
        // Increase confidence if we have clear indicators
        if !indicators.is_empty() && !indicators.contains(&"neutral".to_string()) {
            confidence += 0.2;
        }
        
        // Cap at 0.95
        confidence.min(0.95)
    }
}

/// Reply with sentiment analysis.
pub struct ReplyWithSentiment {
    pub sentiment: SentimentAnalysis,
    pub content: String,
}

/// Quote with sentiment analysis.
pub struct QuoteWithSentiment {
    pub sentiment: SentimentAnalysis,
    pub content: String,
    pub confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_reply_with_sentiment() {
        // Skip test if LLM config is not available
        let processor = match SentimentAwareProcessor::try_new() {
            Ok(p) => p,
            Err(_) => {
                println!("Skipping test: LLM config not available");
                return;
            }
        };
        
        let replies = vec![
            ("user1", "Great point!"),
            ("user2", "I agree"),
        ];
        
        let result = processor.process_reply_with_sentiment(
            "Original tweet",
            "author",
            &replies,
        ).await.expect("Failed to process reply with sentiment");
        
        assert!(result.content.contains("Reply"));
        assert!(result.sentiment.confidence > 0.5);
    }

    #[tokio::test]
    async fn test_process_quote_with_sentiment() {
        // Skip test if LLM config is not available
        let processor = match SentimentAwareProcessor::try_new() {
            Ok(p) => p,
            Err(_) => {
                println!("Skipping test: LLM config not available");
                return;
            }
        };
        
        let replies = vec![
            ("user1", "Great post!"),
        ];
        
        let result = processor.process_quote_with_sentiment(
            "Original tweet",
            &replies,
        ).await.expect("Failed to process quote with sentiment");
        
        assert!(result.confidence > 0.5);
        assert!(!result.content.is_empty());
    }

    #[test]
    fn test_parse_response_with_sentiment_empty() {
        let result = SentimentAwareProcessor::parse_response_with_sentiment_static("")
            .expect("Failed to parse response");
        assert!(result.0.confidence > 0.0);
        assert!(result.1.is_empty());
    }

    #[test]
    fn test_parse_response_with_sentiment_whitespace() {
        let result = SentimentAwareProcessor::parse_response_with_sentiment_static("   ")
            .expect("Failed to parse response");
        assert!(result.1.is_empty());
    }

    #[test]
    fn test_parse_response_with_sentiment_positive() {
        let result = SentimentAwareProcessor::parse_response_with_sentiment_static("This is great and amazing!")
            .expect("Failed to parse response");
        assert!(result.0.confidence > 0.5);
        assert!(!result.0.indicators.is_empty());
        assert!(result.0.indicators.contains(&"positive_word".to_string()));
    }

    #[test]
    fn test_parse_response_with_sentiment_negative() {
        let result = SentimentAwareProcessor::parse_response_with_sentiment_static("This is terrible and awful")
            .expect("Failed to parse response");
        assert!(result.0.indicators.contains(&"negative_word".to_string()));
    }

    #[test]
    fn test_clean_response_content_empty() {
        let result = SentimentAwareProcessor::clean_response_content("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_clean_response_content_special_chars() {
        let result = SentimentAwareProcessor::clean_response_content("  @user #tag  ");
        assert!(!result.contains("@"));
        assert!(!result.contains("#"));
        assert!(result.contains("user"));
        assert!(result.contains("tag"));
    }

    #[test]
    fn test_clean_response_content_valid_chars() {
        let result = SentimentAwareProcessor::clean_response_content("Great post!");
        assert!(result.contains("Great"));
        assert!(result.contains("!"));
    }

    #[test]
    fn test_extract_sentiment_indicators_unicode() {
        let result = SentimentAwareProcessor::extract_sentiment_indicators_static("Great post! 😊");
        assert!(result.contains(&"exclamation".to_string()));
    }

    #[test]
    fn test_extract_sentiment_indicators_multiple() {
        let result = SentimentAwareProcessor::extract_sentiment_indicators_static("Great! What do you think?");
        assert!(result.contains(&"exclamation".to_string()));
        assert!(result.contains(&"question".to_string()));
        assert!(result.contains(&"positive_word".to_string()));
    }

    #[test]
    fn test_extract_sentiment_indicators_empty() {
        let result = SentimentAwareProcessor::extract_sentiment_indicators_static("");
        assert!(result.contains(&"neutral".to_string()));
    }

    #[test]
    fn test_calculate_confidence_empty() {
        let result = SentimentAwareProcessor::calculate_confidence("", &["neutral".to_string()]);
        assert!(result >= 0.5);
        assert!(result <= 0.95);
    }

    #[test]
    fn test_calculate_confidence_long_text() {
        let text = "This is a very long and substantive response that should increase the confidence score significantly";
        let result = SentimentAwareProcessor::calculate_confidence(text, &vec!["positive_word".to_string()]);
        assert!(result > 0.5);
    }

    #[test]
    fn test_extract_sentiment_static_empty() {
        let result = SentimentAwareProcessor::extract_sentiment_static("")
            .expect("Failed to extract sentiment");
        assert!(result.confidence > 0.0);
    }

    #[test]
    fn test_extract_sentiment_static_malformed() {
        let result = SentimentAwareProcessor::extract_sentiment_static("!!!@@@###")
            .expect("Failed to extract sentiment");
        assert!(result.confidence > 0.0);
        assert!(result.indicators.contains(&"exclamation".to_string()));
    }

    #[test]
    fn test_extract_sentiment_static_unicode_emoji() {
        let result = SentimentAwareProcessor::extract_sentiment_static("Great post! 😊🎉")
            .expect("Failed to extract sentiment");
        assert!(result.confidence > 0.0);
    }

    #[test]
    fn test_clean_response_content_unicode() {
        let result = SentimentAwareProcessor::clean_response_content("Great post! 😊");
        assert!(result.contains("Great"));
        // Emojis are filtered out by the character filter
        assert!(!result.contains("😊"));
    }
}
