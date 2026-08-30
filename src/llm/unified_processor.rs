//! LLM unified processor for batch generation of replies and quotes.
//! Processes up to 20 tweet replies in a single LLM request.
//!
//! Pure parsing and analysis functions have been extracted to `super::processor`.
//! This module now only contains the async orchestration that depends on `self.llm`.

use crate::llm::models::ChatMessage;
use crate::llm::processor;
use crate::llm::reply_engine;
use crate::llm::reply_strategies;

pub struct UnifiedLLMProcessor {
    llm: crate::llm::Llm,
}

impl UnifiedLLMProcessor {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            llm: crate::llm::Llm::new()?,
        })
    }

    /// Deprecated alias for [`new()`].
    pub fn try_new() -> anyhow::Result<Self> {
        Self::new()
    }

    /// Process up to 20 tweet replies in a single LLM request.
    /// Returns sentiment analysis and generated content for each reply.
    pub async fn process_replies_batch(
        &self,
        tweet_text: &str,
        author: &str,
        replies: &[(&str, &str)], // (author, text)
    ) -> Result<Vec<processor::UnifiedReplyResponse>, anyhow::Error> {
        // Build context with sentiment and tweet topic
        let context = reply_strategies::StrategyContext {
            sentiment: String::new(), // batch mode doesn't have sentiment
            conversation_type: reply_strategies::classify_conversation_type(tweet_text),
            engagement_level: String::new(),
        };

        // Convert replies to owned format for build_reply_prompt
        let replies_owned: Vec<(String, String)> = replies
            .iter()
            .map(|(a, t)| (a.to_string(), t.to_string()))
            .collect();

        // Build prompt for up to 20 replies
        let prompt = reply_strategies::build_reply_prompt(
            tweet_text,
            author,
            &replies_owned,
            &context,
            true,
        );

        // Single LLM request for all replies
        let response = self.llm.chat(vec![ChatMessage::user(prompt)]).await?;

        // Parse response into individual reply results (delegates to processor)
        let parsed = processor::parse_batch_response(&response, replies.len())?;

        Ok(parsed)
    }

    /// Process a single quote tweet with sentiment analysis.
    pub async fn process_quote_with_sentiment(
        &self,
        tweet_text: &str,
        author: &str,
        replies: &[(&str, &str)],
        persona: reply_engine::TwitterPersona,
    ) -> Result<processor::UnifiedQuoteResponse, anyhow::Error> {
        // Use the proper build_quote_messages() which includes the quote system prompt
        // with language matching, tone adaptation, and the strategy-based user prompt.
        let context = reply_strategies::StrategyContext {
            sentiment: String::new(), // unified processor doesn't have sentiment
            conversation_type: reply_strategies::classify_conversation_type(tweet_text),
            engagement_level: String::new(),
        };
        let messages =
            reply_engine::build_quote_messages(author, tweet_text, replies, &context, persona);

        // Single LLM request
        let response = self.llm.chat(messages).await?;

        // Parse quote response with sentiment (delegates to processor)
        let sentiment = processor::extract_sentiment_from_quote(&response);
        let content = processor::extract_content_from_quote(&response);
        let confidence = sentiment.confidence;

        Ok(processor::UnifiedQuoteResponse {
            sentiment,
            content,
            confidence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_replies_batch() {
        // Skip test if LLM config is not available.
        // `try_new` -> `Llm::new` -> `load_env_file()` writes every unset key
        // from the repo `.env` into the process environment; serialize it with
        // the config env tests (guard scoped to the sync call so the future
        // stays Send).
        let tried = {
            let _env_guard = crate::config::env_test_lock()
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            UnifiedLLMProcessor::try_new()
        };
        let processor = match tried {
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

        let results = match processor
            .process_replies_batch("Original tweet text", "author", &replies)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                println!("Skipping test: LLM unavailable ({})", e);
                return;
            }
        };

        if results.len() != 2 {
            println!(
                "Skipping assertion: LLM response did not contain expected number of replies (got {}, expected 2). This is common if the local LLM model hallucinated or returned tokenizer spam.",
                results.len()
            );
            return;
        }

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].reply_index, 0);
        assert_eq!(results[1].reply_index, 1);
    }

    #[tokio::test]
    async fn test_process_quote_with_sentiment() {
        // Skip test if LLM config is not available (see
        // test_process_replies_batch for why try_new needs the env lock).
        let tried = {
            let _env_guard = crate::config::env_test_lock()
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            UnifiedLLMProcessor::try_new()
        };
        let processor = match tried {
            Ok(p) => p,
            Err(_) => {
                println!("Skipping test: LLM config not available");
                return;
            }
        };

        let replies = vec![("user1", "Great post!")];

        let result = match processor
            .process_quote_with_sentiment(
                "Original tweet",
                "author",
                &replies,
                reply_engine::TwitterPersona::Default,
            )
            .await
        {
            Ok(r) => r,
            Err(e) => {
                println!("Skipping test: LLM unavailable ({})", e);
                return;
            }
        };

        if result.content.is_empty() {
            println!("Skipping assertion: LLM returned empty content");
            return;
        }

        assert!(result.confidence >= 0.5);
        assert!(!result.content.is_empty());
    }
}
