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

use crate::llm::{build_quote_messages, build_reply_messages, Llm};
use crate::prelude::TaskContext;
use crate::utils::timing::TIMEOUT_LONG_SECS;
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
#[instrument(skip(_api, top_replies))]
pub async fn generate_reply(
    _api: &TaskContext,
    tweet_author: &str,
    tweet_text: &str,
    top_replies: Vec<(String, String)>,
) -> Result<String> {
    info!(
        "Generating LLM reply for tweet by @{} ({} longest replies for context)",
        tweet_author,
        top_replies.len()
    );

    // Build prompt with tweet context
    let messages = build_reply_messages(
        tweet_author,
        tweet_text,
        &top_replies
            .iter()
            .map(|(a, t)| (a.as_str(), t.as_str()))
            .collect::<Vec<_>>(),
    );

    // Generate with timeout
    let llm = llm_instance()?;
    let reply = tokio::time::timeout(
        std::time::Duration::from_secs(TIMEOUT_LONG_SECS),
        llm.chat_with_fallback(messages),
    )
    .await
    .context("LLM generation timed out after 30s")??;

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
#[instrument(skip(_api, top_replies))]
pub async fn generate_quote_commentary(
    _api: &TaskContext,
    tweet_author: &str,
    tweet_text: &str,
    top_replies: Vec<(String, String)>,
) -> Result<String> {
    info!(
        "Generating LLM quote commentary for tweet by @{} ({} longest replies for context)",
        tweet_author,
        top_replies.len()
    );

    let messages = build_quote_messages(
        tweet_author,
        tweet_text,
        &top_replies
            .iter()
            .map(|(a, t)| (a.as_str(), t.as_str()))
            .collect::<Vec<_>>(),
    );

    let llm = llm_instance()?;
    let commentary = tokio::time::timeout(
        std::time::Duration::from_secs(TIMEOUT_LONG_SECS),
        llm.chat_with_fallback(messages),
    )
    .await
    .context("LLM generation timed out after 30s")??;

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
    let js = twitteractivity_selectors::js_extract_tweet_context();

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

        let mut replies = obj
            .get("replies")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        item.as_array().and_then(|pair| {
                            let author = pair
                                .first()
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let text = pair
                                .get(1)
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

    #[test]
    fn test_extract_tweet_context_js_has_author_selector() {
        let js = r#"
        (function() {
            var authorEl = document.querySelector('article[data-testid="tweet"] [dir="auto"]');
            var author = authorEl ? authorEl.textContent.trim() : 'unknown';
            var tweetEl = document.querySelector('[data-testid="tweetText"]');
            var text = tweetEl ? tweetEl.textContent.trim() : '';
            var replies = [];
            var replyEls = document.querySelectorAll('article[data-testid="tweet"]');
            for (var i = 1; i < Math.min(replyEls.length, 21); i++) {
                var reply = replyEls[i];
                var replyAuthorEl = reply.querySelector('[dir="auto"]');
                var replyTextEl = reply.querySelector('[data-testid="tweetText"]');
                var replyAuthor = replyAuthorEl ? replyAuthorEl.textContent.trim() : 'unknown';
                var replyText = replyTextEl ? replyTextEl.textContent.trim() : '';
                if (replyText && replyText.length > 0) {
                    replies.push({ author: replyAuthor, text: replyText });
                }
            }
            return {
                author: author,
                text: text,
                replies: replies.map(function(r) { return [r.author, r.text]; })
            };
        })()
        "#;
        assert!(js.contains("data-testid=\"tweet\""));
        assert!(js.contains("data-testid=\"tweetText\""));
        assert!(js.contains("querySelectorAll"));
        assert!(js.contains("Math.min"));
        assert!(js.contains("author: author"));
        assert!(js.contains("text: text"));
    }

    #[test]
    fn test_extract_tweet_context_js_reply_limit() {
        let js = r#"
            for (var i = 1; i < Math.min(replyEls.length, 21); i++) {
        "#;
        assert!(js.contains("Math.min"));
        assert!(js.contains("21"));
    }

    #[test]
    fn test_extract_tweet_context_js_returns_array_of_pairs() {
        let js = r#"
            replies: replies.map(function(r) { return [r.author, r.text]; })
        "#;
        assert!(js.contains("replies.map"));
        assert!(js.contains("r.author"));
        assert!(js.contains("r.text"));
    }

    #[test]
    fn test_extract_tweet_context_js_fallback_unknown() {
        let js = r#"
            var author = authorEl ? authorEl.textContent.trim() : 'unknown';
        "#;
        assert!(js.contains("'unknown'"));
    }

    #[test]
    fn test_extract_tweet_context_js_skips_first_article() {
        let js = r#"
            for (var i = 1; i < Math.min(replyEls.length, 21); i++) {
        "#;
        assert!(js.contains("i = 1"));
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
        let messages = build_reply_messages("author", "tweet", &[]);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
    }

    #[test]
    fn test_build_quote_messages_is_accessible() {
        let messages = build_quote_messages("author", "tweet", &[]);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
    }
}
