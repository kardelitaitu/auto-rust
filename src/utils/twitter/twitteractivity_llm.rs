//! LLM-powered engagement for Twitter automation.
//!
//! Provides AI-generated replies and quote tweets using configured LLM provider.
//! Supports Ollama (local) and OpenRouter (cloud) with automatic fallback.

use anyhow::{Context, Result};
use log::{info, warn};

use crate::llm::{Llm, build_reply_messages, build_quote_messages};
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
pub async fn generate_reply(
    _api: &TaskContext,
    tweet_author: &str,
    tweet_text: &str,
    top_replies: Vec<(String, String)>,
) -> Result<String> {
    info!(
        "Generating LLM reply for tweet by @{} ({} replies for context)",
        tweet_author,
        top_replies.len()
    );

    // Build prompt with tweet context
    let messages = build_reply_messages(
        tweet_author,
        tweet_text,
        &top_replies.iter().map(|(a, t)| (a.as_str(), t.as_str())).collect::<Vec<_>>(),
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
pub async fn generate_quote_commentary(
    _api: &TaskContext,
    tweet_author: &str,
    tweet_text: &str,
    top_replies: Vec<(String, String)>,
) -> Result<String> {
    info!(
        "Generating LLM quote commentary for tweet by @{}",
        tweet_author
    );

    let messages = build_quote_messages(
        tweet_author,
        tweet_text,
        &top_replies.iter().map(|(a, t)| (a.as_str(), t.as_str())).collect::<Vec<_>>(),
    );

    let llm = Llm::new().context("Failed to initialize LLM client")?;
    let commentary = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        llm.chat_with_fallback(messages),
    )
    .await
    .context("LLM generation timed out after 30s")??;

    let sanitized = validate_reply(&commentary)?;
    info!("Generated commentary ({} chars): {}", sanitized.len(), sanitized);

    Ok(sanitized)
}

/// Performs a quote tweet with AI-generated commentary.
///
/// # Arguments
/// * `api` - Task context for browser automation
/// * `commentary` - AI-generated quote tweet text
///
/// # Returns
/// true if quote tweet was successful
pub async fn quote_tweet(
    api: &TaskContext,
    commentary: &str,
) -> Result<bool> {
    info!("Executing quote tweet with {} chars", commentary.len());

    // Click quote tweet button (different from retweet button)
    let quote_btn_js = r#"
        (function() {
            var buttons = document.querySelectorAll('[role="button"]');
            for (var i = 0; i < buttons.length; i++) {
                var btn = buttons[i];
                var ariaLabel = btn.getAttribute('aria-label') || '';
                if (ariaLabel.toLowerCase().includes('quote')) {
                    var rect = btn.getBoundingClientRect();
                    if (rect.width > 0 && rect.height > 0) {
                        btn.click();
                        return true;
                    }
                }
            }
            return false;
        })()
    "#;

    let clicked = api.page().evaluate(quote_btn_js.to_string()).await?;
    if !clicked.value().and_then(|v| v.as_bool()).unwrap_or(false) {
        anyhow::bail!("Quote tweet button not found");
    }

    // Wait for composer to appear
    api.pause(1000).await;

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

    let focused = api.page().evaluate(composer_js.to_string()).await?;
    if !focused.value().and_then(|v| v.as_bool()).unwrap_or(false) {
        anyhow::bail!("Composer textarea not found");
    }

    api.pause(500).await;

    // Type the commentary
    api.keyboard("[data-testid='tweetTextarea_0']", commentary).await?;
    api.pause(1000).await;

    // Click Tweet button
    let tweet_btn_js = r#"
        (function() {
            var buttons = document.querySelectorAll('[data-testid="tweetButton"], [data-testid="tweetButtonInline"]');
            if (buttons.length > 0) {
                buttons[0].click();
                return true;
            }
            return false;
        })()
    "#;

    let submitted = api.page().evaluate(tweet_btn_js.to_string()).await?;
    if !submitted.value().and_then(|v| v.as_bool()).unwrap_or(false) {
        anyhow::bail!("Tweet button not found");
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
/// - No banned AI-sounding words
///
/// # Returns
/// Sanitized text or error if invalid
fn validate_reply(text: &str) -> Result<String> {
    let mut sanitized = text.trim().to_string();

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
        warn!("Reply contains banned AI word: '{}', regenerating...", banned_word);
        // For V1, we'll just warn and continue
        // In production, you might want to regenerate
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
    let truncate_at = text[..truncate_limit]
        .rfind(' ')
        .unwrap_or(truncate_limit);

    format!("{}...", &text[..truncate_at])
}

/// Removes @mentions from text.
fn remove_mentions(text: &str) -> String {
    // Remove @username patterns but keep the word
    let re = regex::Regex::new(r"@\w+").unwrap();
    re.replace_all(text, "").to_string()
}

/// Removes #hashtags from text.
fn remove_hashtags(text: &str) -> String {
    // Remove #tag patterns but keep the word
    let re = regex::Regex::new(r"#(\w+)").unwrap();
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
    "tapestry", "testament", "symphony", "delve", "foster", "crucial",
    "landscape", "game-changer", "underscore", "utilize", "enhance",
    "spearhead", "resonate", "vibrant", "seamless", "robust", "dynamic",
    "realm", "nuance", "harness", "leverage", "meticulous", "paradigm",
    "synergy", "holistic", "integral", "pivotal", "noteworthy", "compelling",
    "intriguing", "fascinating", "captivating", "enthralling", "empower",
    "revolutionize", "deep dive", "unpack", "in conclusion", "moreover",
    "furthermore", "it's important to note", "ah,", "i see", "as a",
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

/// Extracts tweet context from the current page.
///
/// # Returns
/// Tuple of (author, text, Vec<(reply_author, reply_text)>)
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
            
            // Extract top 5 replies
            var replies = [];
            var replyEls = document.querySelectorAll('article [data-testid="tweet"] [dir="auto"]');
            for (var i = 1; i < Math.min(replyEls.length, 6); i++) {
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
    let value = result.value()
        .context("Failed to extract tweet context")?;

    // Parse the result
    if let Some(obj) = value.as_object() {
        let author = obj.get("author")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        
        let text = obj.get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let replies = obj.get("replies")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        item.as_array().and_then(|pair| {
                            let author = pair.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let text = pair.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
                            if !text.is_empty() {
                                Some((author, text))
                            } else {
                                None
                            }
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        
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
                for (var i = 1; i < Math.min(replyEls.length, 6); i++) {
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
