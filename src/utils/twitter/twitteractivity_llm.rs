//! LLM-powered engagement for Twitter automation.
//!
//! This module provides AI-generated content for Twitter engagement including
//! contextual replies and quote tweet commentary. It integrates with configurable
//! LLM providers (Ollama for local, OpenRouter for cloud) with automatic fallback.

pub use super::twitteractivity_llm_execute::quote_tweet;
pub use super::twitteractivity_llm_validation::validate_reply;

use anyhow::{Context, Result};
use log::info;
use tracing::instrument;

use crate::llm::{build_quote_messages, build_reply_messages, Llm};
use crate::prelude::TaskContext;
use crate::utils::timing::TIMEOUT_LONG_SECS;

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
    let llm = Llm::new().context("Failed to initialize LLM client")?;
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

    let llm = Llm::new().context("Failed to initialize LLM client")?;
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
    let js = r#"
        (function() {
            // Extract tweet author
            var authorEl = document.querySelector('[data-testid="tweet"] [dir="auto"]');
            var author = authorEl ? authorEl.textContent.trim() : 'unknown';
            
            // Extract tweet text
            var tweetEl = document.querySelector('[data-testid="tweetText"]');
            var text = tweetEl ? tweetEl.textContent.trim() : '';
            
            // Extract up to 20 replies (will be filtered to 10 longest in Rust)
            var replies = [];
            var replyEls = document.querySelectorAll('article [data-testid="tweet"] [dir="auto"]');
            for (var i = 1; i < Math.min(replyEls.length, 21); i++) {
                var replyEl = replyEls[i];
                var replyText = replyEl.textContent.trim();
                if (replyText && replyText.length > 0) {
                    replies.push({ author: author, text: replyText });
                }
            }
            
            return {
                author: author,
                text: text,
                replies: replies.map(r => [r.author, r.text])
            };
        })()
    "#;

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
                            if !text.is_empty() {
                                Some((author, text))
                            } else {
                                None
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
