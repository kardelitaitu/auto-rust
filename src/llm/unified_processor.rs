//! LLM unified processor for batch generation of replies and quotes.
//! Processes up to 20 tweet replies in a single LLM request.

use crate::llm::models::ChatMessage;
use crate::llm::reply_strategies;
use crate::utils::twitter::sentiment::Sentiment;
use serde_json::Value;

/// Sentiment analysis result with confidence score.
#[derive(Debug, Clone)]
pub struct SentimentAnalysis {
    pub sentiment: Sentiment,
    pub confidence: f32,
    pub indicators: Vec<String>,
}

pub struct UnifiedLLMProcessor {
    llm: crate::llm::Llm,
}

impl Default for UnifiedLLMProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl UnifiedLLMProcessor {
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

    /// Process up to 20 tweet replies in a single LLM request.
    /// Returns sentiment analysis and generated content for each reply.
    pub async fn process_replies_batch(
        &self,
        tweet_text: &str,
        author: &str,
        replies: &[(&str, &str)], // (author, text)
    ) -> Result<Vec<UnifiedReplyResponse>, anyhow::Error> {
        // Build context with all replies
        let context = reply_strategies::StrategyContext::default();

        // Convert replies to owned format for build_reply_prompt
        let replies_owned: Vec<(String, String)> = replies
            .iter()
            .map(|(a, t)| (a.to_string(), t.to_string()))
            .collect();

        // Build prompt for up to 20 replies
        let prompt =
            reply_strategies::build_reply_prompt(tweet_text, author, &replies_owned, &context);

        // Single LLM request for all replies
        let response = self.llm.chat(vec![ChatMessage::user(prompt)]).await?;

        // Parse response into individual reply results
        let parsed = self.parse_batch_response(&response, replies.len())?;

        Ok(parsed)
    }

    /// Process a single quote tweet with sentiment analysis.
    pub async fn process_quote_with_sentiment(
        &self,
        tweet_text: &str,
        _replies: &[(&str, &str)],
    ) -> Result<UnifiedQuoteResponse, anyhow::Error> {
        // Build context
        let _context = reply_strategies::StrategyContext::default();

        // Build prompt
        let system = crate::llm::reply_engine::reply_engine_system_prompt();
        let user = format!(
            "Quote this tweet:\n{}\n\nGenerate a short, engaging quote commentary (max 280 chars):",
            tweet_text
        );
        let messages = vec![ChatMessage::system(system), ChatMessage::user(user)];

        // Single LLM request
        let response = self.llm.chat(messages).await?;

        // Parse quote response with sentiment
        let sentiment = self.extract_sentiment_from_quote(&response)?;
        let content = self.extract_content_from_quote(&response)?;

        // Calculate confidence based on content
        let confidence = sentiment.confidence;

        Ok(UnifiedQuoteResponse {
            sentiment,
            content,
            confidence,
        })
    }

    /// Parse batch response into individual reply results.
    /// Parses LLM response that contains multiple replies separated by delimiters.
    fn parse_batch_response(
        &self,
        response: &str,
        expected_count: usize,
    ) -> Result<Vec<UnifiedReplyResponse>, anyhow::Error> {
        Self::parse_batch_response_static(response, expected_count)
    }

    /// Static version of parse_batch_response for testing without LLM client.
    pub fn parse_batch_response_static(
        response: &str,
        expected_count: usize,
    ) -> Result<Vec<UnifiedReplyResponse>, anyhow::Error> {
        // Try JSON parsing first if the response looks like JSON
        if Self::is_json_response(response) {
            match Self::parse_json_batch_response(response, expected_count) {
                Ok(results) => return Ok(results),
                Err(_) => {
                    // JSON parsing failed, fall back to line-based parsing
                    // This handles cases where the response looks like JSON but is malformed
                }
            }
        }

        // Fall back to line-based parsing
        Self::parse_line_based_batch_response(response, expected_count)
    }

    /// Check if the response appears to be JSON formatted.
    fn is_json_response(response: &str) -> bool {
        let trimmed = response.trim();
        trimmed.starts_with('{') || trimmed.starts_with('[')
    }

    /// Parse JSON-formatted batch response.
    fn parse_json_batch_response(
        response: &str,
        expected_count: usize,
    ) -> Result<Vec<UnifiedReplyResponse>, anyhow::Error> {
        let json: Value = serde_json::from_str(response)?;
        let mut results = Vec::new();

        // Handle array of responses
        if let Value::Array(array) = json {
            for (i, item) in array.iter().enumerate() {
                if i >= expected_count {
                    break;
                }

                if let Value::Object(obj) = item {
                    let content = obj
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let content = Self::clean_reply_content(&content);

                    if content.is_empty() {
                        continue;
                    }

                    let sentiment = Self::analyze_sentiment_from_text(&content);

                    results.push(UnifiedReplyResponse {
                        reply_index: i,
                        sentiment,
                        content,
                    });
                }
            }
        } else if let Value::Object(obj) = json {
            // Handle single object response
            let content = obj
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let content = Self::clean_reply_content(&content);

            if !content.is_empty() {
                let sentiment = Self::analyze_sentiment_from_text(&content);
                results.push(UnifiedReplyResponse {
                    reply_index: 0,
                    sentiment,
                    content,
                });
            }
        }

        Ok(results)
    }

    /// Parse line-based batch response (fallback).
    fn parse_line_based_batch_response(
        response: &str,
        expected_count: usize,
    ) -> Result<Vec<UnifiedReplyResponse>, anyhow::Error> {
        // Split response by common delimiters (newlines, numbers, etc.)
        let lines: Vec<&str> = response
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

        let mut results = Vec::new();

        // Try to parse each line as a separate reply
        for (i, line) in lines.iter().enumerate() {
            if i >= expected_count {
                break;
            }

            // Clean the content
            let content = Self::clean_reply_content(line);

            if content.is_empty() {
                continue;
            }

            // Analyze sentiment of the reply
            let sentiment = Self::analyze_sentiment_from_text(&content);

            results.push(UnifiedReplyResponse {
                reply_index: i,
                sentiment,
                content,
            });
        }

        // If we didn't get enough results, use the full response as a single reply
        if results.is_empty() && !response.trim().is_empty() {
            let content = Self::clean_reply_content(response);
            let sentiment = Self::analyze_sentiment_from_text(&content);
            results.push(UnifiedReplyResponse {
                reply_index: 0,
                sentiment,
                content,
            });
        }

        Ok(results)
    }

    /// Clean and sanitize reply content.
    pub fn clean_reply_content(text: &str) -> String {
        text.trim()
            .chars()
            .filter(|c| {
                c.is_alphanumeric()
                    || c.is_whitespace()
                    || *c == '!'
                    || *c == '?'
                    || *c == '.'
                    || *c == ','
                    || *c == '\''
                    || *c == '-'
            })
            .collect::<String>()
            .trim()
            .to_string()
    }

    /// Analyze sentiment from text using sentiment analysis utilities.
    pub fn analyze_sentiment_from_text(text: &str) -> SentimentAnalysis {
        use crate::utils::twitter::sentiment::analyze_sentiment_sync;

        let sentiment = analyze_sentiment_sync(text);
        let indicators = Self::extract_sentiment_indicators(text);
        let confidence = Self::calculate_confidence(text, &indicators);

        SentimentAnalysis {
            sentiment,
            confidence,
            indicators,
        }
    }

    /// Extract sentiment indicators from text.
    pub fn extract_sentiment_indicators(text: &str) -> Vec<String> {
        let lower = text.to_ascii_lowercase();
        let mut indicators = Vec::new();

        // Positive indicators
        if lower.contains("great") || lower.contains("amazing") || lower.contains("excellent") {
            indicators.push("positive_word".to_string());
        }

        // Negative indicators
        if lower.contains("bad") || lower.contains("terrible") || lower.contains("awful") {
            indicators.push("negative_word".to_string());
        }

        // Exclamation marks indicate strong emotion
        if lower.contains('!') {
            indicators.push("exclamation".to_string());
        }

        // Question marks indicate inquiry
        if lower.contains('?') {
            indicators.push("question".to_string());
        }

        if indicators.is_empty() {
            indicators.push("neutral".to_string());
        }

        indicators
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

    /// Extract sentiment from quote response.
    fn extract_sentiment_from_quote(
        &self,
        response: &str,
    ) -> Result<SentimentAnalysis, anyhow::Error> {
        // Use actual sentiment analysis on the LLM response
        let content = self.extract_content_from_quote(response)?;
        Ok(Self::analyze_sentiment_from_text(&content))
    }

    /// Extract content from quote response.
    fn extract_content_from_quote(&self, response: &str) -> Result<String, anyhow::Error> {
        // Extract generated content
        Ok(response.to_string())
    }
}

/// Response for a single unified reply.
pub struct UnifiedReplyResponse {
    pub reply_index: usize,
    pub sentiment: SentimentAnalysis,
    pub content: String,
}

/// Response for unified quote processing.
pub struct UnifiedQuoteResponse {
    pub sentiment: SentimentAnalysis,
    pub content: String,
    pub confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_replies_batch() {
        // Skip test if LLM config is not available
        let processor = match UnifiedLLMProcessor::try_new() {
            Ok(p) => p,
            Err(_) => {
                println!("Skipping test: LLM config not available");
                return;
            }
        };

        let replies = vec![
            ("user1", "Great post!"),
            ("user2", "Interesting perspective"),
        ];

        let results = processor
            .process_replies_batch("Original tweet text", "author", &replies)
            .await
            .expect("Failed to process replies batch");

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].reply_index, 0);
        assert_eq!(results[1].reply_index, 1);
    }

    #[tokio::test]
    async fn test_process_quote_with_sentiment() {
        // Skip test if LLM config is not available
        let processor = match UnifiedLLMProcessor::try_new() {
            Ok(p) => p,
            Err(_) => {
                println!("Skipping test: LLM config not available");
                return;
            }
        };

        let replies = vec![("user1", "Great post!")];

        let result = processor
            .process_quote_with_sentiment("Original tweet", &replies)
            .await
            .expect("Failed to process replies batch");

        assert!(result.confidence > 0.5);
        assert!(!result.content.is_empty());
    }

    #[test]
    fn test_parse_batch_response_single_line() {
        let response = "This is a great reply!";
        let results = UnifiedLLMProcessor::parse_batch_response_static(response, 1)
            .expect("Failed to parse batch response");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].reply_index, 0);
        assert!(!results[0].content.is_empty());
    }

    #[test]
    fn test_parse_batch_response_empty() {
        let response = "";
        let results = UnifiedLLMProcessor::parse_batch_response_static(response, 1)
            .expect("Failed to parse batch response");

        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_parse_batch_response_whitespace() {
        let response = "   \n   \n   ";
        let results = UnifiedLLMProcessor::parse_batch_response_static(response, 3)
            .expect("Failed to parse batch response");

        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_parse_batch_response_multiple_lines() {
        let response = "First reply here\nSecond reply there\nThird reply somewhere";
        let results = UnifiedLLMProcessor::parse_batch_response_static(response, 3)
            .expect("Failed to parse batch response");

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].reply_index, 0);
        assert_eq!(results[1].reply_index, 1);
        assert_eq!(results[2].reply_index, 2);
    }

    #[test]
    fn test_parse_batch_response_mixed_empty_lines() {
        let response = "First reply\n\n\nSecond reply\n\nThird reply";
        let results = UnifiedLLMProcessor::parse_batch_response_static(response, 3)
            .expect("Failed to parse batch response");

        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_clean_reply_content() {
        let dirty = "  This has @mentions and #hashtags!  ";
        let clean = UnifiedLLMProcessor::clean_reply_content(dirty);

        assert!(clean.contains("This"));
        assert!(!clean.contains("@"));
        assert!(!clean.contains("#"));
    }

    #[test]
    fn test_clean_reply_content_empty() {
        let result = UnifiedLLMProcessor::clean_reply_content("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_clean_reply_content_special_chars() {
        let result = UnifiedLLMProcessor::clean_reply_content("  @user #tag  ");
        assert!(!result.contains("@"));
        assert!(!result.contains("#"));
        assert!(result.contains("user"));
        assert!(result.contains("tag"));
    }

    #[test]
    fn test_clean_reply_content_unicode() {
        let result = UnifiedLLMProcessor::clean_reply_content("Great post! 😊");
        assert!(result.contains("Great"));
        // Emojis are filtered out by the character filter
        assert!(!result.contains("😊"));
    }

    #[test]
    fn test_analyze_sentiment_from_text_positive() {
        let text = "This is great and amazing!";
        let sentiment = UnifiedLLMProcessor::analyze_sentiment_from_text(text);

        assert!(sentiment.confidence > 0.5);
        assert!(!sentiment.indicators.is_empty());
    }

    #[test]
    fn test_analyze_sentiment_from_text_empty() {
        let text = "";
        let sentiment = UnifiedLLMProcessor::analyze_sentiment_from_text(text);

        assert!(sentiment.confidence >= 0.0);
    }

    #[test]
    fn test_analyze_sentiment_from_text_negative() {
        let text = "This is terrible and awful";
        let sentiment = UnifiedLLMProcessor::analyze_sentiment_from_text(text);

        assert!(sentiment.indicators.contains(&"negative_word".to_string()));
    }

    #[test]
    fn test_extract_sentiment_indicators() {
        let text = "Great post!";
        let indicators = UnifiedLLMProcessor::extract_sentiment_indicators(text);

        assert!(indicators.contains(&"positive_word".to_string()));
        assert!(indicators.contains(&"exclamation".to_string()));
    }

    #[test]
    fn test_extract_sentiment_indicators_empty() {
        let text = "";
        let indicators = UnifiedLLMProcessor::extract_sentiment_indicators(text);

        assert!(indicators.contains(&"neutral".to_string()));
    }

    #[test]
    fn test_extract_sentiment_indicators_unicode() {
        let text = "Great post! 😊";
        let indicators = UnifiedLLMProcessor::extract_sentiment_indicators(text);

        assert!(indicators.contains(&"exclamation".to_string()));
    }

    #[test]
    fn test_extract_sentiment_indicators_question() {
        let text = "What do you think?";
        let indicators = UnifiedLLMProcessor::extract_sentiment_indicators(text);

        assert!(indicators.contains(&"question".to_string()));
    }

    #[test]
    fn test_calculate_confidence() {
        let text = "This is a very long and substantive response with great content!";
        let indicators = vec!["positive_word".to_string(), "exclamation".to_string()];
        let confidence = UnifiedLLMProcessor::calculate_confidence(text, &indicators);

        assert!(confidence > 0.5);
        assert!(confidence <= 0.95);
    }

    #[test]
    fn test_calculate_confidence_empty() {
        let text = "";
        let indicators = vec!["neutral".to_string()];
        let confidence = UnifiedLLMProcessor::calculate_confidence(text, &indicators);

        assert!(confidence >= 0.5);
        assert!(confidence <= 0.95);
    }

    #[test]
    fn test_extract_sentiment_from_quote() {
        let response = "This is an amazing quote!";
        let sentiment = UnifiedLLMProcessor::analyze_sentiment_from_text(response);

        assert!(sentiment.confidence > 0.0);
        assert!(!sentiment.indicators.is_empty());
    }

    #[test]
    fn test_extract_sentiment_from_quote_malformed() {
        let response = "!!!@@@###";
        let sentiment = UnifiedLLMProcessor::analyze_sentiment_from_text(response);

        assert!(sentiment.confidence > 0.0);
    }

    #[test]
    fn test_parse_json_batch_response_array() {
        let response = r#"[
            {"content": "First reply"},
            {"content": "Second reply"},
            {"content": "Third reply"}
        ]"#;

        let results = UnifiedLLMProcessor::parse_batch_response_static(response, 3)
            .expect("Failed to parse batch response");

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].reply_index, 0);
        assert_eq!(results[1].reply_index, 1);
        assert_eq!(results[2].reply_index, 2);
        assert!(results[0].content.contains("First"));
    }

    #[test]
    fn test_parse_json_batch_response_single_object() {
        let response = r#"{"content": "Single reply"}"#;

        let results = UnifiedLLMProcessor::parse_batch_response_static(response, 1)
            .expect("Failed to parse batch response");

        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("Single"));
    }

    #[test]
    fn test_parse_json_batch_response_empty_array() {
        let response = r#"[]"#;

        let results = UnifiedLLMProcessor::parse_batch_response_static(response, 3)
            .expect("Failed to parse batch response");

        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_parse_json_batch_response_malformed_json() {
        let response = r#"{"invalid json"#;

        // Should fall back to line-based parsing for invalid JSON
        let _results = UnifiedLLMProcessor::parse_batch_response_static(response, 1)
            .expect("Failed to parse batch response");
        // Will parse as line-based, so may have 0 or 1 results depending on content
    }

    #[test]
    fn test_is_json_response_array() {
        assert!(UnifiedLLMProcessor::is_json_response("[]"));
        assert!(UnifiedLLMProcessor::is_json_response("[1, 2, 3]"));
    }

    #[test]
    fn test_is_json_response_object() {
        assert!(UnifiedLLMProcessor::is_json_response("{}"));
        assert!(UnifiedLLMProcessor::is_json_response(r#"{"key": "value"}"#));
    }

    #[test]
    fn test_is_json_response_plain_text() {
        assert!(!UnifiedLLMProcessor::is_json_response("plain text"));
        assert!(!UnifiedLLMProcessor::is_json_response(
            "Some text\nMore text"
        ));
    }

    #[test]
    fn test_parse_batch_response_json_with_whitespace() {
        let response = r#"  
            [
                {"content": "First reply"},
                {"content": "Second reply"}
            ]  
        "#;

        let results = UnifiedLLMProcessor::parse_batch_response_static(response, 2)
            .expect("Failed to parse batch response");

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_parse_batch_response_line_based_fallback() {
        let response = "First reply\nSecond reply\nThird reply";

        let results = UnifiedLLMProcessor::parse_batch_response_static(response, 3)
            .expect("Failed to parse batch response");

        assert_eq!(results.len(), 3);
    }
}
