//! Twitter intent task.
//! Navigates to a Twitter/X URL for viewing or interaction.

use crate::prelude::TaskContext;
use anyhow::Result;
use log::info;
use serde_json::Value;

const DEFAULT_NAVIGATE_TIMEOUT_MS: u64 = 30_000;
const POST_NAVIGATE_WAIT_MS: u64 = 3000;

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let tweet_url = extract_url_from_payload(&payload)?;
    
    info!("[twitterintent] Task started - target: {}", tweet_url);

    // Navigate to tweet
    info!("[twitterintent] Navigating to tweet...");
    api.navigate(&tweet_url, DEFAULT_NAVIGATE_TIMEOUT_MS)
        .await?;
    
    // Wait for page to load
    api.pause(POST_NAVIGATE_WAIT_MS).await;
    
    // Extract basic tweet info
    let (author, tweet_text) = extract_tweet_info(api).await?;
    info!(
        "[twitterintent] Tweet by @{}: {}",
        author,
        truncate_text(&tweet_text, 80)
    );
    
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
    // Check for default_url in payload
    if let Some(default_url) = payload.get("default_url") {
        if let Some(url_str) = default_url.as_str() {
            return Ok(url_str.to_string());
        }
    }
    // Fallback: check any field that looks like a URL
    for (key, val) in payload
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("payload not an object"))?
    {
        if key != "url" && key != "value" && key != "default_url" {
            if let Some(v) = val.as_str() {
                if !v.is_empty() && (v.contains("x.com") || v.contains("twitter.com")) {
                    return Ok(v.to_string());
                }
            }
        }
    }
    Err(anyhow::anyhow!("No URL found in payload"))
}

async fn extract_tweet_info(api: &TaskContext) -> Result<(String, String)> {
    let page = api.page();

    let result = page
        .evaluate(
            r#"
        (function() {
            var tweet = document.querySelector('article[data-testid="tweet"]');
            if (!tweet) return null;
            
            var nameLink = tweet.querySelector('a[href^="/"]');
            var username = nameLink ? nameLink.getAttribute('href').split('/')[1] : 'unknown';
            var textContent = tweet.querySelectorAll('[data-testid="tweetText"]');
            var text = textContent.length > 0 ? textContent[textContent.length - 1].innerText : '';
            
            return { username: username, text: text };
        })()
    "#,
        )
        .await?;

    let value = result.value();
    if let Some(v) = value {
        if let Some(obj) = v.as_object() {
            let username = obj
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let text = obj.get("text").and_then(|v| v.as_str()).unwrap_or("");
            return Ok((username.to_string(), text.to_string()));
        }
    }

    Err(anyhow::anyhow!("Could not extract tweet info"))
}

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_url_from_payload_url() {
        let payload = json!({"url": "https://x.com/user/status/123"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert!(result.contains("x.com"));
    }

    #[test]
    fn extract_url_from_payload_value() {
        let payload = json!({"value": "https://x.com/user/status/456"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert!(result.contains("x.com"));
    }

    #[test]
    fn extract_url_from_payload_fallback() {
        let payload = json!({"tweet": "https://x.com/user/status/789"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert!(result.contains("x.com"));
    }

    #[test]
    fn extract_url_missing() {
        let payload = json!({});
        assert!(extract_url_from_payload(&payload).is_err());
    }

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("hello", 10), "hello");
        assert_eq!(truncate_text("hello world", 5), "hello...");
    }
}
