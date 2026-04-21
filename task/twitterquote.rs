//! Twitter quote task.
//! Quotes a tweet with LLM-generated commentary.

use crate::internal::text::{preview_chars, truncate_with_ellipsis};
use crate::llm::{reply_engine_system_prompt, ChatMessage, Llm};
use crate::prelude::TaskContext;
use anyhow::Result;
use log::{info, warn};
use serde_json::Value;

const DEFAULT_NAVIGATE_TIMEOUT_MS: u64 = 30_000;
const POST_WAIT_MS: u64 = 5000;

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let tweet_url = extract_url_from_payload(&payload)?;
    let custom_quote = payload
        .get("quote_text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    info!("[twitterquote] Task started - target: {}", tweet_url);

    // Navigate to tweet
    info!("[twitterquote] Navigating to tweet...");
    api.navigate(&tweet_url, DEFAULT_NAVIGATE_TIMEOUT_MS)
        .await?;
    api.pause(2000).await;

    // Extract tweet context
    info!("[twitterquote] Extracting tweet context...");
    let (author, tweet_text, replies) = extract_tweet_context(api).await?;
    info!(
        "[twitterquote] Tweet by @{}: {}",
        author,
        preview_chars(&tweet_text, 50)
    );
    info!("[twitterquote] Extracted {} replies for context", replies.len());

    // Generate or use provided quote
    let quote_text = if let Some(text) = custom_quote {
        info!("[twitterquote] Using provided quote text");
        text
    } else {
        info!("[twitterquote] Generating LLM quote...");
        let llm = Llm::new()?;
        let messages = build_quote_messages(&author, &tweet_text, &replies);

        match llm.chat(messages).await {
            Ok(text) => validate_reply(&text),
            Err(e) => {
                warn!("[twitterquote] LLM failed: {}, using fallback", e);
                "Interesting take!".to_string()
            }
        }
    };

    let quote_text = truncate_with_ellipsis(&quote_text, 280);
    info!("[twitterquote] Quote text: {}", preview_chars(&quote_text, 60));

    // Click quote button
    info!("[twitterquote] Clicking quote button...");
    click_quote_button(api).await?;

    api.pause(1500).await;

    // Type quote text
    info!("[twitterquote] Typing quote...");
    type_quote(api, &quote_text).await?;

    api.pause(1000).await;

    // Post
    info!("[twitterquote] Posting quote...");
    let posted = post_quote_with_retry(api, 3).await?;

    if posted {
        info!("[twitterquote] Quote posted successfully!");
    } else {
        warn!("[twitterquote] Failed to post quote");
    }

    api.pause(POST_WAIT_MS).await;
    info!("[twitterquote] Task completed");
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
    for (key, val) in payload
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("payload not an object"))?
    {
        if key != "url" && key != "value" {
            if let Some(v) = val.as_str() {
                if !v.is_empty() && v.contains("x.com") {
                    return Ok(v.to_string());
                }
            }
        }
    }
    Err(anyhow::anyhow!("No URL found in payload"))
}

async fn extract_tweet_context(api: &TaskContext) -> Result<(String, String, Vec<(String, String)>)> {
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
            
            // Extract replies for context
            var replies = [];
            var articles = document.querySelectorAll('article[data-testid="tweet"]');
            for (var i = 1; i < Math.min(articles.length, 6); i++) {
                var article = articles[i];
                var link = article.querySelector('a[href^="/"]');
                var replyUser = link ? link.getAttribute('href').split('/')[1] : 'unknown';
                var replyTextEls = article.querySelectorAll('[data-testid="tweetText"]');
                var replyText = replyTextEls.length > 0 ? replyTextEls[replyTextEls.length - 1].innerText : '';
                if (replyText) replies.push([replyUser, replyText]);
            }
            
            return { username: username, text: text, replies: replies };
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

            let replies: Vec<(String, String)> = obj
                .get("replies")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| {
                            item.as_array().and_then(|pair| {
                                let u = pair.first().and_then(|v| v.as_str()).unwrap_or("unknown");
                                let t = pair.get(1).and_then(|v| v.as_str()).unwrap_or("");
                                if !t.is_empty() {
                                    Some((u.to_string(), t.to_string()))
                                } else {
                                    None
                                }
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            return Ok((username.to_string(), text.to_string(), replies));
        }
    }

    Err(anyhow::anyhow!("Could not extract tweet context"))
}

async fn click_quote_button(api: &TaskContext) -> Result<()> {
    // Quote button - try multiple selectors
    let js = r#"
        (function() {
            var buttons = document.querySelectorAll('button[data-testid]');
            for (var i = 0; i < buttons.length; i++) {
                var btn = buttons[i];
                var testId = (btn.getAttribute('data-testid') || '').toLowerCase();
                if (testId.includes('retweet') && !testId.includes('unretweet')) {
                    var rect = btn.getBoundingClientRect();
                    if (rect.width > 0 && rect.height > 0) {
                        btn.click();
                        return { success: true };
                    }
                }
            }
            return { success: false };
        })()
    "#;
    let result = api.page().evaluate(js).await?;
    let value = result.value();
    if let Some(v) = value {
        if let Some(obj) = v.as_object() {
            if let Some(true) = obj.get("success").and_then(|v| v.as_bool()) {
                return Ok(());
            }
        }
    }
    // Fallback
    let outcome = api.click("[data-testid=\"retweet\"]").await?;
    info!("[twitterquote] Quote button: {}", outcome.summary());
    Ok(())
}

async fn type_quote(api: &TaskContext, text: &str) -> Result<()> {
    let page = api.page();

    let result = page
        .evaluate(
            r#"
        (function() {
            var composer = document.querySelector('[data-testid="tweetTextarea"]') || 
                        document.querySelector('[contenteditable="true"]');
            if (composer) return true;
            return false;
        })()
    "#,
        )
        .await?;

    let value = result.value();
    if let Some(v) = value {
        if let Some(true) = v.as_bool() {
            api.type_text(text).await?;
            return Ok(());
        }
    }

    Err(anyhow::anyhow!("Composer not found"))
}

async fn post_quote(api: &TaskContext) -> Result<bool> {
    let outcome = api
        .click("[data-testid=\"retweetConfirm\"], [data-testid=\"tweetButton\"]")
        .await?;
    info!("[twitterquote] Post: {}", outcome.summary());
    Ok(true)
}

async fn post_quote_with_retry(api: &TaskContext, max_retries: u32) -> Result<bool> {
    let mut last_error: Option<anyhow::Error> = None;
    for attempt in 1..=max_retries {
        match post_quote(api).await {
            Ok(true) => return Ok(true),
            Ok(false) => {
                warn!("[twitterquote] Post failed (attempt {}/{})", attempt, max_retries);
                last_error = Some(anyhow::anyhow!("Post returned false"));
            }
            Err(e) => {
                warn!("[twitterquote] Post error (attempt {}/{}): {}", attempt, max_retries, e);
                last_error = Some(e);
            }
        }
        if attempt < max_retries {
            api.pause(2000).await;
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Post failed after {} retries", max_retries)))
}

fn validate_reply(text: &str) -> String {
    let mut result = text.trim().to_string();

    result = result.trim_start_matches('"').to_string();
    result = result.trim_end_matches('"').to_string();

    if result.ends_with('.') {
        result.pop();
    }

    result
}

fn build_quote_messages(
    tweet_author: &str,
    tweet_text: &str,
    replies: &[(String, String)],
) -> Vec<ChatMessage> {
    let system = reply_engine_system_prompt();

    let mut user = format!("Quote this tweet by @{}:\n{}", tweet_author, tweet_text);

    if !replies.is_empty() {
        user.push_str("\n\nTop replies for context:\n");
        for (author, text) in replies.iter().take(3) {
            user.push_str(&format!("@{}: {}\n", author, text));
        }
    }

    user.push_str("\n\nGenerate a short, engaging quote commentary (max 280 chars):");

    vec![ChatMessage::system(system), ChatMessage::user(user)]
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
    fn validate_reply_removes_quotes() {
        let text = "\"Hello world\"";
        let result = validate_reply(text);
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn validate_reply_trims() {
        let text = "  Some text  ";
        let result = validate_reply(text);
        assert_eq!(result, "Some text");
    }
}