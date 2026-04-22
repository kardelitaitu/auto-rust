//! Twitter retweet task.
//! Retweets a tweet with optional quote commentary.

use crate::llm::{reply_engine_system_prompt, ChatMessage, Llm};
use crate::prelude::TaskContext;
use crate::utils::twitter::twitteractivity_llm::validate_reply;
use anyhow::Result;
use log::{info, warn};
use serde_json::Value;

const DEFAULT_NAVIGATE_TIMEOUT_MS: u64 = 30_000;
const POST_WAIT_MS: u64 = 5000;

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let tweet_url = extract_url_from_payload(&payload)?;
    let quote_text = payload
        .get("quote_text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let use_llm = payload
        .get("use_llm")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    info!("[twitterretweet] Task started - target: {}", tweet_url);

    // Navigate to tweet
    info!("[twitterretweet] Navigating to tweet...");
    api.navigate(&tweet_url, DEFAULT_NAVIGATE_TIMEOUT_MS)
        .await?;
    api.pause(2000).await;

    // Determine if we should quote or native retweet
    let do_quote = quote_text.is_some() || use_llm;

    if do_quote {
        // Quote tweet flow
        let commentary = if let Some(text) = quote_text {
            info!("[twitterretweet] Using provided quote text");
            text
        } else if use_llm {
            info!("[twitterretweet] Generating LLM quote...");
            let (author, tweet_text) = extract_tweet_context(api).await?;
            info!("[twitterretweet] Tweet by @{}", author);

            let llm = Llm::new()?;
            let messages = build_quote_messages(&author, &tweet_text);

            match llm.chat(messages).await {
                Ok(text) => {
                    validate_reply(&text).unwrap_or_else(|_| "Interesting take!".to_string())
                }
                Err(e) => {
                    warn!("[twitterretweet] LLM failed: {}, using fallback", e);
                    "Interesting take!".to_string()
                }
            }
        } else {
            "Interesting take!".to_string()
        };

        info!("[twitterretweet] Quote text: {}", commentary);

        // Click quote button
        info!("[twitterretweet] Clicking quote button...");
        click_quote_button(api).await?;
        api.pause(1500).await;

        // Type quote
        info!("[twitterretweet] Typing quote...");
        type_quote(api, &commentary).await?;
        api.pause(1000).await;

        // Post
        info!("[twitterretweet] Posting quote...");
        let posted = post_quote_with_retry(api, 3).await?;
        if posted {
            info!("[twitterretweet] Quote posted successfully!");
        } else {
            warn!("[twitterretweet] Failed to post quote");
        }
    } else {
        // Native retweet flow
        info!("[twitterretweet] Performing native retweet...");

        let rt_js = r#"
            (function() {
                var buttons = document.querySelectorAll('[data-testid="retweet"]');
                if (buttons.length > 0) {
                    buttons[0].click();
                    return true;
                }
                return false;
            })()
        "#;

        let clicked = api.page().evaluate(rt_js.to_string()).await?;
        if clicked.value().and_then(|v| v.as_bool()).unwrap_or(false) {
            info!("[twitterretweet] Retweet menu opened");
            api.pause(1000).await;

            // Confirm retweet
            info!("[twitterretweet] Confirming retweet...");
            let confirm_js = r#"
                (function() {
                    var buttons = document.querySelectorAll('[data-testid="retweetConfirm"]');
                    if (buttons.length > 0) {
                        buttons[0].click();
                        return true;
                    }
                    return false;
                })()
            "#;
            let confirmed = api.page().evaluate(confirm_js.to_string()).await?;
            if confirmed.value().and_then(|v| v.as_bool()).unwrap_or(false) {
                info!("[twitterretweet] Retweet confirmed!");
            } else {
                warn!("[twitterretweet] Failed to confirm retweet");
            }
        } else {
            warn!("[twitterretweet] Failed to click retweet button");
        }
    }

    api.pause(POST_WAIT_MS).await;
    info!("[twitterretweet] Task completed");
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
    for (key, val) in payload
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("payload not an object"))?
    {
        if key != "url" && key != "value" && key != "default_url" {
            if let Some(v) = val.as_str() {
                if !v.is_empty() && v.contains("x.com") {
                    return Ok(v.to_string());
                }
            }
        }
    }
    Err(anyhow::anyhow!("No URL found in payload"))
}

async fn extract_tweet_context(api: &TaskContext) -> Result<(String, String)> {
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

    Err(anyhow::anyhow!("Could not extract tweet context"))
}

async fn click_quote_button(api: &TaskContext) -> Result<()> {
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
    let _ = api.click("[data-testid=\"retweet\"]").await?;
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
    info!("[twitterretweet] Post: {}", outcome.summary());
    Ok(true)
}

async fn post_quote_with_retry(api: &TaskContext, max_retries: u32) -> Result<bool> {
    let mut last_error: Option<anyhow::Error> = None;
    for attempt in 1..=max_retries {
        match post_quote(api).await {
            Ok(true) => return Ok(true),
            Ok(false) => {
                warn!(
                    "[twitterretweet] Post failed (attempt {}/{})",
                    attempt, max_retries
                );
                last_error = Some(anyhow::anyhow!("Post returned false"));
            }
            Err(e) => {
                warn!(
                    "[twitterretweet] Post error (attempt {}/{}): {}",
                    attempt, max_retries, e
                );
                last_error = Some(e);
            }
        }
        if attempt < max_retries {
            api.pause(2000).await;
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Post failed after {} retries", max_retries)))
}

fn build_quote_messages(tweet_author: &str, tweet_text: &str) -> Vec<ChatMessage> {
    let system = reply_engine_system_prompt();
    let user = format!(
        "Quote this tweet by @{}:\n{}\n\nGenerate a short, engaging quote commentary (max 280 chars):",
        tweet_author, tweet_text
    );
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
    fn extract_url_missing() {
        let payload = json!({});
        assert!(extract_url_from_payload(&payload).is_err());
    }
}
