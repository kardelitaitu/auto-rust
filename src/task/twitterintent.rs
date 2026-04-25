//! Twitter intent task.
//! Handles Twitter intent URLs for follow, like, post, quote, and retweet actions.
//!
//! # Intent URL Examples
//!
//! - Follow: `https://x.com/intent/follow?screen_name=username`
//! - Like: `https://x.com/intent/like?tweet_id=123456789`
//! - Post: `https://x.com/intent/tweet?text=Hello%20world`
//! - Quote: `https://x.com/intent/tweet?url=https://x.com/user/status/123&text=Great`
//! - Quote with reply: `https://x.com/intent/tweet?text=this+is+example+reply%0Athis+is+second+line&in_reply_to=2047854858305405321`
//! - Retweet: `https://x.com/intent/retweet?tweet_id=123456789`

use crate::prelude::TaskContext;
use crate::utils::math::random_in_range;
use anyhow::Result;
use log::{info, warn};
use serde_json::Value;

const DEFAULT_NAVIGATE_TIMEOUT_MS: u64 = 30_000;
const POST_NAVIGATE_WAIT_MS: u64 = 2000;

#[derive(Debug, Clone, Copy)]
enum IntentType {
    Follow,
    Like,
    Post,
    Quote,
    Retweet,
}

impl IntentType {
    fn from_url(url: &str) -> Result<Self> {
        if url.contains("/intent/follow") {
            Ok(IntentType::Follow)
        } else if url.contains("/intent/like") {
            Ok(IntentType::Like)
        } else if url.contains("/intent/tweet") {
            // Distinguish between post and quote by checking for url parameter
            if url.contains("url=") {
                Ok(IntentType::Quote)
            } else {
                Ok(IntentType::Post)
            }
        } else if url.contains("/intent/retweet") {
            Ok(IntentType::Retweet)
        } else {
            Err(anyhow::anyhow!("URL does not contain a valid intent path"))
        }
    }

    fn confirm_selector(&self) -> &'static str {
        match self {
            IntentType::Follow | IntentType::Like | IntentType::Retweet => {
                "[data-testid=\"confirmationSheetConfirm\"]"
            }
            IntentType::Post | IntentType::Quote => "[data-testid=\"tweetButton\"]",
        }
    }
}

#[derive(Debug)]
struct IntentInfo {
    description: String,
}

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let url = extract_url_from_payload(&payload)?;
    info!("[twitterintent] Intent URL: {}", url);

    let intent_type = IntentType::from_url(&url)?;
    info!("[twitterintent] Detected intent type: {:?}", intent_type);

    // Navigate to intent URL
    api.navigate(&url, DEFAULT_NAVIGATE_TIMEOUT_MS)
        .await?;
    api.pause(POST_NAVIGATE_WAIT_MS).await;

    // Click confirm button with verification
    let selector = intent_type.confirm_selector();
    info!("[twitterintent] Clicking button: {}", selector);
    
    let click_success = click_with_verification(api, selector, intent_type).await?;
    
    if click_success {
        let intent_info = parse_intent_info(&url, intent_type);
        info!("[twitterintent] SUCCESS {}", intent_info.description);
    } else {
        warn!("[twitterintent] Click verification failed - button may have already been clicked or action already performed");
    }

    // Random 5-15s pause after intent action (success or failed)
    let post_action_pause = random_in_range(5000, 15000);
    info!("[twitterintent] Pausing {}ms after intent action", post_action_pause);
    api.pause(post_action_pause).await;

    // Return to previous page using JavaScript
    info!("[twitterintent] Returning to previous page");
    api.page().evaluate("window.history.back()").await?;

    info!("[twitterintent] Task completed");
    Ok(())
}

fn extract_url_from_payload(payload: &Value) -> Result<String> {
    if let Some(value) = payload.get("url") {
        if let Some(url_str) = value.as_str() {
            return Ok(url_str.to_string());
        }
    }
    if let Some(value) = payload.get("value") {
        if let Some(url_str) = value.as_str() {
            return Ok(url_str.to_string());
        }
    }
    // Fallback: check any field that looks like a URL
    for (key, val) in payload
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("payload not an object"))?
    {
        if key != "url" && key != "value" {
            if let Some(v) = val.as_str() {
                if !v.is_empty() && (v.contains("x.com") || v.contains("twitter.com")) {
                    return Ok(v.to_string());
                }
            }
        }
    }
    Err(anyhow::anyhow!("No URL found in payload"))
}

async fn click_with_verification(
    api: &TaskContext,
    selector: &str,
    _intent_type: IntentType,
) -> Result<bool> {
    // Check if button is visible
    if !api.visible(selector).await? {
        warn!("[twitterintent] Button not visible - may already be clicked");
        return Ok(false);
    }

    click_and_verify(api, selector).await
}

async fn click_and_verify(api: &TaskContext, selector: &str) -> Result<bool> {
    // Random 4-8s pause before clicking (100ms interval)
    let pause_ms = random_in_range(4000, 8000);
    info!("[twitterintent] Waiting {}ms before clicking", pause_ms);
    api.pause(pause_ms).await;

    // Attempt click
    let outcome = api.click(selector).await?;
    info!("[twitterintent] Click outcome: {}", outcome.summary());

    // Wait for action to process
    api.pause(1000).await;

    // Verify success: confirm button should disappear for all intents
    let button_gone = !api.visible(selector).await?;
    Ok(button_gone)
}

fn parse_intent_info(url: &str, intent_type: IntentType) -> IntentInfo {
    match intent_type {
        IntentType::Follow => {
            let username = extract_param(url, "screen_name").unwrap_or_else(|| "unknown".to_string());
            IntentInfo {
                description: format!("Followed @{}", username),
            }
        }
        IntentType::Like => {
            let tweet_id = extract_param(url, "tweet_id").unwrap_or_else(|| "unknown".to_string());
            IntentInfo {
                description: format!("Liked tweet {}", tweet_id),
            }
        }
        IntentType::Post => {
            let text = extract_param(url, "text")
                .unwrap_or_else(|| "empty text".to_string());
            IntentInfo {
                description: format!("Posted '{}'", text),
            }
        }
        IntentType::Quote => {
            let url_param = extract_param(url, "url").unwrap_or_else(|| "unknown".to_string());
            IntentInfo {
                description: format!("Quoted {}", url_param),
            }
        }
        IntentType::Retweet => {
            let tweet_id = extract_param(url, "tweet_id").unwrap_or_else(|| "unknown".to_string());
            IntentInfo {
                description: format!("Retweeted tweet {}", tweet_id),
            }
        }
    }
}

fn extract_param(url: &str, param: &str) -> Option<String> {
    let pattern = format!("{}=", param);
    if let Some(start) = url.find(&pattern) {
        let start = start + pattern.len();
        let end = url[start..].find('&').map(|e| start + e).unwrap_or(url.len());
        Some(url[start..end].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_intent_type_from_url_follow() {
        let url = "https://x.com/intent/follow?screen_name=elonmusk";
        assert!(matches!(IntentType::from_url(url), Ok(IntentType::Follow)));
    }

    #[test]
    fn test_intent_type_from_url_like() {
        let url = "https://x.com/intent/like?tweet_id=123456789";
        assert!(matches!(IntentType::from_url(url), Ok(IntentType::Like)));
    }

    #[test]
    fn test_intent_type_from_url_post() {
        let url = "https://x.com/intent/tweet?text=Hello";
        assert!(matches!(IntentType::from_url(url), Ok(IntentType::Post)));
    }

    #[test]
    fn test_intent_type_from_url_quote() {
        let url = "https://x.com/intent/tweet?url=https://x.com/user/status/123&text=Great";
        assert!(matches!(IntentType::from_url(url), Ok(IntentType::Quote)));
    }

    #[test]
    fn test_intent_type_from_url_retweet() {
        let url = "https://x.com/intent/retweet?tweet_id=123456789";
        assert!(matches!(IntentType::from_url(url), Ok(IntentType::Retweet)));
    }

    #[test]
    fn test_intent_type_invalid() {
        let url = "https://x.com/elonmusk";
        assert!(IntentType::from_url(url).is_err());
    }

    #[test]
    fn test_confirm_selector() {
        assert_eq!(
            IntentType::Follow.confirm_selector(),
            "[data-testid=\"confirmationSheetConfirm\"]"
        );
        assert_eq!(
            IntentType::Like.confirm_selector(),
            "[data-testid=\"confirmationSheetConfirm\"]"
        );
        assert_eq!(
            IntentType::Retweet.confirm_selector(),
            "[data-testid=\"confirmationSheetConfirm\"]"
        );
        assert_eq!(
            IntentType::Post.confirm_selector(),
            "[data-testid=\"tweetButton\"]"
        );
        assert_eq!(
            IntentType::Quote.confirm_selector(),
            "[data-testid=\"tweetButton\"]"
        );
    }

    #[test]
    fn extract_url_from_payload_url() {
        let payload = json!({"url": "https://x.com/intent/follow?screen_name=elonmusk"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert!(result.contains("intent/follow"));
    }

    #[test]
    fn extract_url_from_payload_value() {
        let payload = json!({"value": "https://x.com/intent/like?tweet_id=123"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert!(result.contains("intent/like"));
    }

    #[test]
    fn extract_url_missing() {
        let payload = json!({});
        assert!(extract_url_from_payload(&payload).is_err());
    }

    #[test]
    fn test_extract_param_screen_name() {
        let url = "https://x.com/intent/follow?screen_name=elonmusk";
        assert_eq!(extract_param(url, "screen_name"), Some("elonmusk".to_string()));
    }

    #[test]
    fn test_extract_param_tweet_id() {
        let url = "https://x.com/intent/like?tweet_id=123456789";
        assert_eq!(extract_param(url, "tweet_id"), Some("123456789".to_string()));
    }

    #[test]
    fn test_extract_param_text() {
        let url = "https://x.com/intent/tweet?text=Hello%20world";
        assert_eq!(extract_param(url, "text"), Some("Hello%20world".to_string()));
    }

    #[test]
    fn test_extract_param_url() {
        let url = "https://x.com/intent/tweet?url=https://x.com/user/status/123&text=Great";
        assert_eq!(extract_param(url, "url"), Some("https://x.com/user/status/123".to_string()));
    }

    #[test]
    fn test_extract_param_missing() {
        let url = "https://x.com/intent/follow?screen_name=elonmusk";
        assert_eq!(extract_param(url, "tweet_id"), None);
    }

    #[test]
    fn test_parse_intent_info_follow() {
        let url = "https://x.com/intent/follow?screen_name=elonmusk";
        let info = parse_intent_info(url, IntentType::Follow);
        assert_eq!(info.description, "Followed @elonmusk");
    }

    #[test]
    fn test_parse_intent_info_like() {
        let url = "https://x.com/intent/like?tweet_id=123456789";
        let info = parse_intent_info(url, IntentType::Like);
        assert_eq!(info.description, "Liked tweet 123456789");
    }

    #[test]
    fn test_parse_intent_info_post() {
        let url = "https://x.com/intent/tweet?text=Hello%20world";
        let info = parse_intent_info(url, IntentType::Post);
        assert_eq!(info.description, "Posted 'Hello%20world'");
    }

    #[test]
    fn test_parse_intent_info_quote() {
        let url = "https://x.com/intent/tweet?url=https://x.com/user/status/123&text=Great";
        let info = parse_intent_info(url, IntentType::Quote);
        assert_eq!(info.description, "Quoted https://x.com/user/status/123");
    }

    #[test]
    fn test_parse_intent_info_retweet() {
        let url = "https://x.com/intent/retweet?tweet_id=123456789";
        let info = parse_intent_info(url, IntentType::Retweet);
        assert_eq!(info.description, "Retweeted tweet 123456789");
    }

    #[test]
    fn test_parse_intent_info_follow_unknown() {
        let url = "https://x.com/intent/follow";
        let info = parse_intent_info(url, IntentType::Follow);
        assert_eq!(info.description, "Followed @unknown");
    }

    #[test]
    fn test_parse_intent_info_post_empty() {
        let url = "https://x.com/intent/tweet";
        let info = parse_intent_info(url, IntentType::Post);
        assert_eq!(info.description, "Posted 'empty text'");
    }

    #[test]
    fn test_intent_type_reply_vs_quote_both_have_url() {
        // Both Reply and Quote have url= parameter, both detected as Quote
        let reply_url = "https://x.com/intent/tweet?url=https://x.com/user/status/123&text=Reply";
        let quote_url = "https://x.com/intent/tweet?url=https://x.com/user/status/123&text=Quote";
        
        assert!(matches!(IntentType::from_url(reply_url), Ok(IntentType::Quote)));
        assert!(matches!(IntentType::from_url(quote_url), Ok(IntentType::Quote)));
    }

    #[test]
    fn test_extract_param_multiline_text() {
        let url = "https://x.com/intent/tweet?text=this+is+example+reply%0Athis+is+second+line";
        assert_eq!(extract_param(url, "text"), Some("this+is+example+reply%0Athis+is+second+line".to_string()));
    }

    #[test]
    fn test_extract_param_complex_url() {
        let url = "https://x.com/intent/tweet?text=Hello&url=https://x.com/user/status/123&in_reply_to=456";
        assert_eq!(extract_param(url, "text"), Some("Hello".to_string()));
        assert_eq!(extract_param(url, "url"), Some("https://x.com/user/status/123".to_string()));
        assert_eq!(extract_param(url, "in_reply_to"), Some("456".to_string()));
    }

    #[test]
    fn test_parse_intent_info_multiline() {
        let url = "https://x.com/intent/tweet?text=this+is+example+reply%0Athis+is+second+line&url=https://x.com/user/status/123";
        let info = parse_intent_info(url, IntentType::Quote);
        assert_eq!(info.description, "Quoted https://x.com/user/status/123");
    }
}
