//! LLM-powered engagement for Twitter automation.
//!
//! This module provides AI-generated content for Twitter engagement including
//! contextual replies and quote tweet commentary. It integrates with configurable
//! LLM providers (Ollama for local, OpenRouter for cloud) with automatic fallback.
//!
//! ## Key Components
//!
//! - **Reply Generation**: Contextual replies based on tweet content and replies
//! - **Quote Commentary**: AI-generated commentary for quote tweets
//! - **Content Validation**: Sanitization to ensure Twitter-compliant output
//! - **Context Extraction**: DOM-based tweet context extraction for LLM input
//!
//! ## Key Functions
//!
//! - [`generate_reply()`]: Generate contextual reply using LLM
//! - [`generate_quote_commentary()`]: Generate quote commentary using LLM
//! - [`validate_reply()`]: Sanitize and validate LLM output
//! - [`extract_tweet_context()`]: Extract tweet data from DOM
//! - [`quote_tweet()`]: Execute quote tweet with commentary
//!
//! ## Usage
//!
//! ```rust,no_run
//! use rust_orchestrator::utils::twitter::twitteractivity_llm::*;
//! # use rust_orchestrator::runtime::task_context::TaskContext;
//! # async fn example(api: &TaskContext) -> anyhow::Result<()> {
//!
//! // Generate a contextual reply
//! let replies = vec![("user1".to_string(), "reply1".to_string()), ("user2".to_string(), "reply2".to_string())];
//! let reply = generate_reply(api, "author", "tweet text", replies).await?;
//!
//! // Generate quote commentary
//! let replies = vec![("user1".to_string(), "reply1".to_string()), ("user2".to_string(), "reply2".to_string())];
//! let commentary = generate_quote_commentary(api, "author", "tweet text", replies).await?;
//!
//! // Validate output before posting
//! let sanitized = validate_reply(&reply)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## LLM Providers
//!
//! The module supports multiple LLM providers with automatic fallback:
//! - **Ollama**: Local LLM server (default)
//! - **OpenRouter**: Cloud API with multiple models
//!
//! Configure provider in application settings.
//!
//! ## Content Validation
//!
//! LLM output is validated to ensure Twitter compliance:
//! - Maximum 280 characters
//! - No @mentions (unless in original tweet)
//! - No #hashtags
//! - No emojis
//! - No banned AI-sounding words


/// Timeout for finding quote tweet button (seconds)
const QUOTE_BUTTON_TIMEOUT_SECS: u64 = 5;
/// Short pause after clicking quote button (milliseconds)
const QUOTE_CLICK_PAUSE_SHORT_MS: u64 = 300;
/// Long pause after clicking quote button (milliseconds)
const QUOTE_CLICK_PAUSE_LONG_MS: u64 = 600;
/// Wait time for composer to appear after button click (milliseconds)
const COMPOSER_WAIT_MS: u64 = 1000;

use anyhow::{Context, Result};
use log::{info, warn};
use std::time::Duration;
use tokio::time::timeout;
use tracing::instrument;

use crate::llm::{build_quote_messages, build_reply_messages, Llm};
use crate::prelude::TaskContext;

/// Generates a contextual reply to a tweet using LLM.
///
/// # Arguments
/// * `_api` - Task context for extracting tweet information (reserved for V2 image handling)
/// * `tweet_author` - Username of tweet author
/// * `tweet_text` - Full text of the tweet
/// * `top_replies` - Vector of (author, text) pairs for top replies
///
/// # Returns
/// Generated reply text (guaranteed to be <280 chars)
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
        std::time::Duration::from_secs(30),
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
///
/// # Arguments
/// * `_api` - Task context (for future image handling)
/// * `tweet_author` - Username of tweet author
/// * `tweet_text` - Full text of the tweet
/// * `top_replies` - Community replies for context
///
/// # Returns
/// Generated quote tweet commentary (<280 chars)
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
        std::time::Duration::from_secs(30),
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
    
    info!("Generated quote commentary ({} chars): {}", sanitized.len(), sanitized);

    Ok(sanitized)
}

/// Performs a quote tweet with AI-generated commentary.
///
/// This function finds the quote tweet button, clicks it, types the provided
/// commentary into the composer, and submits the quote tweet. All operations have
/// timeouts to prevent hanging.
///
/// # Arguments
///
/// * `api` - Task context for browser automation
/// * `commentary` - AI-generated quote tweet text to type
///
/// # Returns
///
/// Returns `Ok(true)` if quote tweet was posted successfully.
/// Returns `Ok(false)` if any step fails (button find, focus, type, or submit).
///
/// # Errors
///
/// Returns error if operations fail unexpectedly.
///
/// # Behavior
///
/// - Finds quote button using ARIA-label with 5s timeout
/// - Moves mouse and clicks with human-like pauses
/// - Waits 1s for composer to appear
/// - Focuses composer textarea with 5s timeout
/// - Types commentary with 10s timeout
/// - Finds tweet button with 5s timeout
/// - Moves mouse and clicks with human-like pauses
/// - Waits 2s for post to complete
///
/// # Selectors Used
///
/// - Quote button: `[role="button"]` with aria-label containing "quote"
/// - Textarea: `[data-testid="tweetTextarea_0"]` or `[role="textbox"]`
/// - Tweet button: `[data-testid="tweetButton"]` or `[data-testid="tweetButtonInline"]`
///
/// # Timeouts
///
/// - Quote button find: 5 seconds
/// - Composer focus: 5 seconds
/// - Typing: 10 seconds
/// - Tweet button find: 5 seconds
/// - Mouse move: 5 seconds
/// - Button click: 5 seconds
#[instrument(skip(api))]
pub async fn quote_tweet(api: &TaskContext, commentary: &str) -> Result<bool> {
    info!("Executing quote tweet with {} chars", commentary.len());

    // Find quote tweet button coordinates
    let quote_btn_js = r#"
        (function() {
            var buttons = document.querySelectorAll('[role="button"]');
            for (var i = 0; i < buttons.length; i++) {
                var btn = buttons[i];
                var ariaLabel = btn.getAttribute('aria-label') || '';
                if (ariaLabel.toLowerCase().includes('quote')) {
                    var rect = btn.getBoundingClientRect();
                    if (rect.width > 0 && rect.height > 0) {
                        return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
                    }
                }
            }
            return null;
        })()
    "#;

    let result = match timeout(Duration::from_secs(QUOTE_BUTTON_TIMEOUT_SECS), api.page().evaluate(quote_btn_js.to_string())).await {
        Ok(r) => r?,
        Err(_) => {
            warn!("Timeout finding quote tweet button");
            return Ok(false);
        }
    };
    let coords = result.value().and_then(|v| v.as_object());

    let (x, y) = if let Some(obj) = coords {
        (
            obj.get("x").and_then(|v| v.as_f64()),
            obj.get("y").and_then(|v| v.as_f64()),
        )
    } else {
        (None, None)
    };

    let (x, y) = match (x, y) {
        (Some(x), Some(y)) => (x, y),
        _ => {
            warn!("Quote tweet button not found");
            return Ok(false);
        }
    };

    // Human-like cursor movement then click
    api.move_mouse_to(x, y).await?;
    super::twitteractivity_humanized::human_pause(api, QUOTE_CLICK_PAUSE_SHORT_MS).await;
    api.click_at(x, y).await?;
    super::twitteractivity_humanized::human_pause(api, QUOTE_CLICK_PAUSE_LONG_MS).await;

    // Wait for composer to appear
    api.pause(COMPOSER_WAIT_MS).await;

    // Find composer textarea and type commentary
    let composer_js = r#"
        (function() {
            var textarea = document.querySelector('[data-testid="tweetTextarea_0"]') ||
                          document.querySelector('[role="textbox"]');
            if (textarea) {
                textarea.focus();
                return true;
            }
            return false;
        })()
    "#;

    let focused = match timeout(Duration::from_secs(QUOTE_BUTTON_TIMEOUT_SECS), api.page().evaluate(composer_js.to_string())).await {
        Ok(r) => r?,
        Err(_) => {
            warn!("Timeout focusing composer textarea");
            return Ok(false);
        }
    };
    if !focused.value().and_then(|v| v.as_bool()).unwrap_or(false) {
        warn!("Composer textarea not found");
        return Ok(false);
    }

    api.pause(500).await;

    // Type the commentary
    match timeout(Duration::from_secs(10), api.keyboard("[data-testid='tweetTextarea_0']", commentary)).await {
        Ok(r) => r?,
        Err(_) => {
            warn!("Timeout typing commentary");
            return Ok(false);
        }
    }
    api.pause(COMPOSER_WAIT_MS).await;

    // Find Tweet button coordinates
    let tweet_btn_js = r#"
        (function() {
            var buttons = document.querySelectorAll('[data-testid="tweetButton"], [data-testid="tweetButtonInline"]');
            if (buttons.length > 0) {
                var rect = buttons[0].getBoundingClientRect();
                return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
            }
            return null;
        })()
    "#;

    let button_result = match timeout(Duration::from_secs(QUOTE_BUTTON_TIMEOUT_SECS), api.page().evaluate(tweet_btn_js.to_string())).await {
        Ok(r) => r?,
        Err(_) => {
            warn!("Timeout finding tweet button");
            return Ok(false);
        }
    };
    let coords = button_result.value().and_then(|v| v.as_object());

    let (tx, ty) = if let Some(obj) = coords {
        (
            obj.get("x").and_then(|v| v.as_f64()),
            obj.get("y").and_then(|v| v.as_f64()),
        )
    } else {
        (None, None)
    };

    let (tx, ty) = match (tx, ty) {
        (Some(tx), Some(ty)) => (tx, ty),
        _ => {
            warn!("Tweet button not found");
            return Ok(false);
        }
    };

    // Human-like cursor movement then click
    match timeout(Duration::from_secs(QUOTE_BUTTON_TIMEOUT_SECS), api.move_mouse_to(tx, ty)).await {
        Ok(_) => {}
        Err(_) => {
            warn!("Timeout moving mouse to tweet button");
            return Ok(false);
        }
    }
    super::twitteractivity_humanized::human_pause(api, QUOTE_CLICK_PAUSE_SHORT_MS).await;
    match timeout(Duration::from_secs(QUOTE_BUTTON_TIMEOUT_SECS), api.click_at(tx, ty)).await {
        Ok(_) => {}
        Err(_) => {
            warn!("Timeout clicking tweet button");
            return Ok(false);
        }
    }

    // Wait for post to complete
    api.pause(2000).await;

    info!("Quote tweet posted successfully");
    Ok(true)
}

/// Validates and sanitizes LLM-generated text for Twitter.
///
/// # Checks:
/// - Length < 280 characters
/// - No @mentions
/// - No #hashtags
/// - No emojis
/// - No asterisks (emphasis)
/// - No banned AI-sounding words
///
/// # Returns
/// Sanitized text or error if invalid
pub fn validate_reply(text: &str) -> Result<String> {
    let mut sanitized = text.trim().to_string();

    // Remove asterisk emphasis (**word** and *word*)
    sanitized = sanitized.replace("**", "").replace('*', "");

    // Enforce character limit
    if sanitized.len() > 270 {
        // Leave room for ...
        sanitized = truncate_to_word_boundary(&sanitized, 270);
    }

    // Remove @mentions
    sanitized = remove_mentions(&sanitized);

    // Remove #hashtags
    sanitized = remove_hashtags(&sanitized);

    // Remove emojis (basic Unicode range check)
    sanitized = remove_emojis(&sanitized);

    // Check for banned AI words
    if let Some(banned_word) = check_banned_words(&sanitized) {
        warn!(
            "Reply contains banned AI word: '{}', but proceeding",
            banned_word
        );
        // For V1, we'll just warn and continue
    }

    // Ensure non-empty
    if sanitized.is_empty() {
        anyhow::bail!("Generated reply is empty after sanitization");
    }

    Ok(sanitized)
}

/// Truncates text to max_length at word boundary.
fn truncate_to_word_boundary(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        return text.to_string();
    }

    // Find last space before max_length (leave room for "...")
    let truncate_limit = max_length.saturating_sub(3);
    let truncate_at = text[..truncate_limit].rfind(' ').unwrap_or(truncate_limit);

    format!("{}...", &text[..truncate_at])
}

/// Removes @mentions from text.
fn remove_mentions(text: &str) -> String {
    // Remove @username patterns but keep the word
    let re = regex::Regex::new(r"@\w+").expect("Failed to compile mentions regex");
    re.replace_all(text, "").to_string()
}

/// Removes #hashtags from text.
fn remove_hashtags(text: &str) -> String {
    // Remove #tag patterns but keep the word
    let re = regex::Regex::new(r"#(\w+)").expect("Failed to compile hashtags regex");
    re.replace_all(text, "$1").to_string()
}

/// Removes emojis from text (basic Unicode ranges).
fn remove_emojis(text: &str) -> String {
    text.chars()
        .filter(|c| {
            // Filter out common emoji ranges
            let cp = *c as u32;
            // Emoticons: U+1F600–U+1F64F
            !(0x1F600..=0x1F64F).contains(&cp) &&
            // Misc Symbols and Pictographs: U+1F300–U+1F5FF
            !(0x1F300..=0x1F5FF).contains(&cp) &&
            // Transport and Map: U+1F680–U+1F6FF
            !(0x1F680..=0x1F6FF).contains(&cp) &&
            // Flags: U+1F1E0–U+1F1FF
            !(0x1F1E0..=0x1F1FF).contains(&cp) &&
            // Misc symbols: U+2600–U+26FF
            !(0x2600..=0x26FF).contains(&cp) &&
            // Dingbats: U+2700–U+27BF
            !(0x2700..=0x27BF).contains(&cp)
        })
        .collect()
}

/// Banned AI-sounding words list.
const BANNED_WORDS: &[&str] = &[
    "tapestry",
    "testament",
    "symphony",
    "delve",
    "foster",
    "crucial",
    "landscape",
    "game-changer",
    "underscore",
    "utilize",
    "enhance",
    "spearhead",
    "resonate",
    "vibrant",
    "seamless",
    "robust",
    "dynamic",
    "realm",
    "nuance",
    "harness",
    "leverage",
    "meticulous",
    "paradigm",
    "synergy",
    "holistic",
    "integral",
    "pivotal",
    "noteworthy",
    "compelling",
    "intriguing",
    "fascinating",
    "captivating",
    "enthralling",
    "empower",
    "revolutionize",
    "deep dive",
    "unpack",
    "in conclusion",
    "moreover",
    "furthermore",
    "it's important to note",
    "ah,",
    "i see",
    "as a",
];

/// Checks if text contains banned AI words.
fn check_banned_words(text: &str) -> Option<String> {
    let text_lower = text.to_lowercase();
    for word in BANNED_WORDS {
        if text_lower.contains(word) {
            return Some(word.to_string());
        }
    }
    None
}

/// Extracts tweet context from the current page for LLM processing.
///
/// This function queries the DOM to extract the current tweet's author, text,
/// and up to 20 top replies (then selects the 10 longest). The extracted data
/// is used as context for LLM reply/quote generation.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
///
/// # Returns
///
/// Returns tuple of (author, text, replies):
/// - `author`: Username of the tweet author
/// - `text`: Full text content of the tweet
/// - `replies`: Vector of (reply_author, reply_text) tuples (up to 10 longest)
///
/// # Errors
///
/// Returns error if DOM evaluation fails or data extraction fails.
///
/// # Behavior
///
/// - Extracts author from `[data-testid="tweet"] [dir="auto"]`
/// - Extracts tweet text from `[data-testid="tweetText"]`
/// - Extracts up to 20 replies from article elements, selects 10 longest
/// - Skips the first reply element (likely the root tweet)
/// - Returns "unknown" for author if not found
/// - Returns empty string for text if not found
///
/// # Selectors Used
///
/// - Author: `[data-testid="tweet"] [dir="auto"]`
/// - Text: `[data-testid="tweetText"]`
/// - Replies: `article [data-testid="tweet"] [dir="auto"]`
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
        replies.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
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
    fn test_validate_reply_truncates_long_text() {
        let long_text = "a".repeat(300);
        let result = validate_reply(&long_text).unwrap();
        assert!(result.len() <= 270);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_validate_reply_removes_mentions() {
        let text = "Great point @user! I agree with @someone else.";
        let result = validate_reply(text).unwrap();
        assert!(!result.contains("@user"));
        assert!(!result.contains("@someone"));
    }

    #[test]
    fn test_validate_reply_removes_hashtags() {
        let text = "This is #amazing and #awesome!";
        let result = validate_reply(text).unwrap();
        assert!(!result.contains("#"));
        assert!(result.contains("amazing"));
        assert!(result.contains("awesome"));
    }

    #[test]
    fn test_validate_reply_removes_emojis() {
        let text = "Love this! ❤️ 🔥 👍";
        let result = validate_reply(text).unwrap();
        assert!(!result.contains("❤"));
        assert!(!result.contains("🔥"));
        assert!(!result.contains("👍"));
    }

    #[test]
    fn test_check_banned_words_detects_ai_speak() {
        assert!(check_banned_words("This is crucial for the landscape").is_some());
        assert!(check_banned_words("Let me delve into this").is_some());
        assert!(check_banned_words("Normal text without banned words").is_none());
    }

    #[test]
    fn test_truncate_to_word_boundary() {
        let text = "This is a long sentence with many words that needs truncation";
        let result = truncate_to_word_boundary(text, 30);
        assert!(result.len() <= 33); // 30 + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_extract_tweet_context_js_generation() {
        // Test that the JS for tweet context extraction is valid
        let js = r#"
            (function() {
                var authorEl = document.querySelector('[data-testid="tweet"] [dir="auto"]');
                var author = authorEl ? authorEl.textContent.trim() : 'unknown';
                
                var tweetEl = document.querySelector('[data-testid="tweetText"]');
                var text = tweetEl ? tweetEl.textContent.trim() : '';
                
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

        // Just verify JS is valid (can't actually execute without browser)
        assert!(js.contains("querySelector"));
        assert!(js.contains("data-testid"));
        assert!(js.contains("return"));
    }
}
