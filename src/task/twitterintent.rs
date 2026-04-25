//! Twitter intent task.
//! Handles Twitter intent URLs for follow, like, post, quote, and retweet actions.

use crate::prelude::TaskContext;
use anyhow::Result;
use log::info;
use serde_json::Value;

const DEFAULT_NAVIGATE_TIMEOUT_MS: u64 = 30_000;
const POST_NAVIGATE_WAIT_MS: u64 = 2000;
const POST_CLICK_WAIT_MS: u64 = 3000;

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

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let url = extract_url_from_payload(&payload)?;
    info!("[twitterintent] Intent URL: {}", url);

    let intent_type = IntentType::from_url(&url)?;
    info!("[twitterintent] Detected intent type: {:?}", intent_type);

    // Navigate to intent URL
    api.navigate(&url, DEFAULT_NAVIGATE_TIMEOUT_MS)
        .await?;
    api.pause(POST_NAVIGATE_WAIT_MS).await;

    // Click confirm button
    let selector = intent_type.confirm_selector();
    info!("[twitterintent] Clicking button: {}", selector);
    
    let outcome = api.click(selector).await?;
    info!("[twitterintent] Click outcome: {}", outcome.summary());

    api.pause(POST_CLICK_WAIT_MS).await;

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
}
