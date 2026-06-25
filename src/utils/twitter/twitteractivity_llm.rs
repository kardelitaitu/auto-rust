//! LLM-powered engagement for Twitter automation.
//!
//! This module provides AI-generated content for Twitter engagement including
//! contextual replies and quote tweet commentary. It integrates with configurable
//! LLM providers (Ollama for local, `OpenRouter` for cloud) with automatic fallback.

pub use super::twitteractivity_llm_execute::quote_tweet;
pub use super::twitteractivity_llm_validation::validate_reply;

use anyhow::{Context, Result};
use log::info;
use std::sync::OnceLock;
use tracing::instrument;

use crate::llm::reply_strategies::sentiment_to_strategy_context;
// StrategyContext is used in tests via crate::llm::reply_strategies::StrategyContext
use crate::llm::reply_engine::{build_quote_messages, build_reply_messages, TwitterPersona};
use crate::llm::Llm;
use crate::prelude::TaskContext;
use crate::utils::timing::TIMEOUT_LONG_SECS;
use crate::utils::twitter::sentiment::Sentiment;
use crate::utils::twitter::twitteractivity_retry::{retry_with_backoff, RetryConfig};
use crate::utils::twitter::twitteractivity_selectors;

fn llm_instance() -> Result<&'static Llm> {
    static LLM: OnceLock<Llm> = OnceLock::new();
    if let Some(llm) = LLM.get() {
        return Ok(llm);
    }

    let llm = Llm::new().context("Failed to initialize LLM client")?;
    let _ = LLM.set(llm);
    LLM.get()
        .context("LLM client initialized but could not be retrieved")
}

/// Generates a contextual reply to a tweet using LLM.
#[instrument(skip(api, top_replies))]
pub async fn generate_reply(
    api: &TaskContext,
    tweet_author: &str,
    tweet_text: &str,
    top_replies: Vec<(String, String)>,
    sentiment: Sentiment,
) -> Result<String> {
    info!(
        "Generating LLM reply for tweet by @{} ({} longest replies for context)",
        tweet_author,
        top_replies.len()
    );

    // Build StrategyContext from detected sentiment and tweet content
    let strategy_context = sentiment_to_strategy_context(sentiment, tweet_text);

    // Build prompt with tweet context
    let messages = build_reply_messages(
        tweet_author,
        tweet_text,
        &top_replies
            .iter()
            .map(|(a, t)| (a.as_str(), t.as_str()))
            .collect::<Vec<_>>(),
        &strategy_context,
        TwitterPersona::select_for_session(api.session_id()),
    );

    // Generate with retry and timeout (each attempt gets its own 30s window)
    let llm = llm_instance()?;
    let reply = retry_with_backoff(
        || {
            let msgs = messages.clone();
            async move {
                tokio::time::timeout(
                    std::time::Duration::from_secs(TIMEOUT_LONG_SECS),
                    llm.chat_with_fallback(msgs),
                )
                .await
                .context("LLM generation timed out after 30s")?
            }
        },
        &RetryConfig::conservative(),
        api,
        "generate_reply",
    )
    .await?;

    // Validate and sanitize output
    let sanitized = validate_reply(&reply)?;

    // Ensure non-empty after sanitization
    if sanitized.is_empty() {
        anyhow::bail!("Generated reply is empty after sanitization");
    }

    info!("Generated reply ({} chars): {}", sanitized.len(), sanitized);

    Ok(sanitized)
}

/// Generates a quote tweet commentary using LLM.
#[instrument(skip(api, top_replies))]
pub async fn generate_quote_commentary(
    api: &TaskContext,
    tweet_author: &str,
    tweet_text: &str,
    top_replies: Vec<(String, String)>,
    sentiment: Sentiment,
) -> Result<String> {
    info!(
        "Generating LLM quote commentary for tweet by @{} ({} longest replies for context)",
        tweet_author,
        top_replies.len()
    );

    // Build StrategyContext from detected sentiment and tweet content
    let strategy_context = sentiment_to_strategy_context(sentiment, tweet_text);

    let messages = build_quote_messages(
        tweet_author,
        tweet_text,
        &top_replies
            .iter()
            .map(|(a, t)| (a.as_str(), t.as_str()))
            .collect::<Vec<_>>(),
        &strategy_context,
        TwitterPersona::select_for_session(api.session_id()),
    );

    // Generate with retry and timeout (each attempt gets its own 30s window)
    let llm = llm_instance()?;
    let commentary = retry_with_backoff(
        || {
            let msgs = messages.clone();
            async move {
                tokio::time::timeout(
                    std::time::Duration::from_secs(TIMEOUT_LONG_SECS),
                    llm.chat_with_fallback(msgs),
                )
                .await
                .context("LLM generation timed out after 30s")?
            }
        },
        &RetryConfig::conservative(),
        api,
        "generate_quote_commentary",
    )
    .await?;

    // Validate and sanitize output
    let sanitized = validate_reply(&commentary)?;

    // Ensure non-empty after sanitization
    if sanitized.is_empty() {
        anyhow::bail!("Generated quote commentary is empty after sanitization");
    }

    info!(
        "Generated quote commentary ({} chars): {}",
        sanitized.len(),
        sanitized
    );

    Ok(sanitized)
}

/// Extracts tweet context from the current page for LLM processing.
pub async fn extract_tweet_context(
    api: &TaskContext,
) -> Result<(String, String, Vec<(String, String)>)> {
    let js = twitteractivity_selectors::js_extract_all_tweets();

    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value().context("Failed to extract tweet context")?;

    // Parse the result
    if let Some(obj) = value.as_object() {
        let author = obj
            .get("author")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let text = obj
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let replies: Vec<(String, String)> = obj
            .get("replies")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        item.as_object().and_then(|reply_obj| {
                            let author = reply_obj
                                .get("author")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let text = reply_obj
                                .get("text")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            if text.is_empty() {
                                None
                            } else {
                                Some((author, text))
                            }
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Sort by text length descending and take top 10 longest replies
        let mut replies = replies;
        replies.sort_by_key(|b| std::cmp::Reverse(b.1.len()));
        replies.truncate(10);

        Ok((author, text, replies))
    } else {
        anyhow::bail!("Invalid tweet context format")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::Role;

    #[test]
    fn test_extract_tweet_context_js_has_author_selector() {
        let js = twitteractivity_selectors::js_extract_all_tweets();
        assert!(js.contains("data-testid=\"tweet\""));
        assert!(js.contains("data-testid=\"tweetText\""));
        assert!(js.contains("querySelectorAll"));
        assert!(js.contains("Math.min"));
        assert!(js.contains("author: author"));
        assert!(js.contains("text: text"));
    }

    #[test]
    fn test_extract_tweet_context_js_reply_limit() {
        let js = twitteractivity_selectors::js_extract_all_tweets();
        assert!(js.contains("Math.min"));
        assert!(js.contains("21"));
    }

    #[test]
    fn test_extract_tweet_context_js_returns_object_replies() {
        let js = twitteractivity_selectors::js_extract_all_tweets();
        assert!(js.contains("replies.push"));
        assert!(js.contains("id: tweetId"));
        assert!(js.contains("text: elText"));
        assert!(js.contains("author: elAuthor"));
    }

    #[test]
    fn test_extract_tweet_context_js_fallback_unknown() {
        let js = twitteractivity_selectors::js_extract_all_tweets();
        assert!(js.contains("'unknown'"));
    }

    #[test]
    fn test_extract_tweet_context_js_skips_first_article() {
        let js = twitteractivity_selectors::js_extract_all_tweets();
        assert!(js.contains("i === 0"));
        assert!(js.contains("continue"));
    }

    #[test]
    fn test_validate_reply_re_exported() {
        // Verify validate_reply is re-exported from the validation module
        let _ = validate_reply;
    }

    #[test]
    fn test_quote_tweet_re_exported() {
        // Verify quote_tweet is re-exported from the execute module
        let _ = quote_tweet;
    }

    #[test]
    fn test_generate_reply_signature() {
        // Verify the function exists with the expected signature
        // by checking it's a function (can be referenced)
        fn assert_fn<T>(_: T) {}
        assert_fn(generate_reply);
    }

    #[test]
    fn test_generate_quote_commentary_signature() {
        fn assert_fn<T>(_: T) {}
        assert_fn(generate_quote_commentary);
    }

    #[test]
    fn test_extract_tweet_context_signature() {
        fn assert_fn<T>(_: T) {}
        assert_fn(extract_tweet_context);
    }

    #[test]
    fn test_llm_instance_returns_static_ref() {
        // Verify llm_instance() returns Result<&'static Llm> by checking the lifetime constraint
        fn assert_static<T: 'static>(_: T) {}
        assert_static(llm_instance);
    }

    #[test]
    fn test_build_reply_messages_is_accessible() {
        // Verify the function is accessible from the crate
        let context = crate::llm::reply_strategies::StrategyContext::default();
        let messages =
            build_reply_messages("author", "tweet", &[], &context, TwitterPersona::Default);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::System);
        assert_eq!(messages[1].role, Role::User);
    }

    #[test]
    fn test_build_quote_messages_is_accessible() {
        let context = crate::llm::reply_strategies::StrategyContext::default();
        let messages =
            build_quote_messages("author", "tweet", &[], &context, TwitterPersona::Default);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::System);
        assert_eq!(messages[1].role, Role::User);
    }
}
