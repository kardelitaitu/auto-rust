//! Twitter quote task.
//! Quotes a tweet with LLM-generated commentary.

use crate::internal::text::{preview_chars, truncate_with_ellipsis};
use crate::prelude::TaskContext;
use crate::utils::timing::{
    duration_with_variance, run_with_timeout, DEFAULT_NAVIGATION_TIMEOUT_MS,
};
use crate::utils::twitter::unified_processor::UnifiedLLMProcessor;
use crate::utils::twitter::PostOutcome;
use crate::utils::twitter::StatusUrl;
use anyhow::Result;
use log::{info, warn};
use serde_json::Value;

const POST_WAIT_MS: u64 = 5000;
pub const DEFAULT_TWITTERQUOTE_TASK_DURATION_MS: u64 = 45_000;

fn task_duration_ms() -> u64 {
    duration_with_variance(DEFAULT_TWITTERQUOTE_TASK_DURATION_MS, 20)
}

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let duration_ms = task_duration_ms();
    run_with_timeout(duration_ms, "twitterquote", run_inner(api, payload)).await
}

async fn run_inner(api: &TaskContext, payload: Value) -> Result<()> {
    let tweet_url = extract_url_from_payload(&payload)?;
    let custom_quote = payload
        .get("quote_text")
        .and_then(|v| v.as_str())
        .map(std::string::ToString::to_string);

    info!("[twitterquote] Task started - target: {tweet_url}");

    // Navigate to tweet
    info!("[twitterquote] Navigating to tweet...");
    api.navigate(&tweet_url, DEFAULT_NAVIGATION_TIMEOUT_MS)
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
    info!(
        "[twitterquote] Extracted {} replies for context",
        replies.len()
    );

    // Generate or use provided quote
    let quote_text = if let Some(text) = custom_quote {
        info!("[twitterquote] Using provided quote text");
        text
    } else {
        info!("[twitterquote] Generating LLM quote using unified batch processor...");
        let processor = UnifiedLLMProcessor::new();

        // Convert replies to format expected by unified processor
        let reply_tuples: Vec<(&str, &str)> = replies
            .iter()
            .map(|(a, t)| (a.as_str(), t.as_str()))
            .collect();

        let reply_texts: crate::utils::twitter::unified_processor::UnifiedQuoteResponse = processor
            .process_quote_with_sentiment(&tweet_text, &reply_tuples)
            .await
            .map_err(|e| {
                warn!("[twitterquote] Unified processor failed: {e}, using fallback");
                e
            })?;

        // Use the quote content
        reply_texts.content
    };

    let quote_text = truncate_with_ellipsis(&quote_text, 280);
    info!(
        "[twitterquote] Quote text: {}",
        preview_chars(&quote_text, 60)
    );

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
    match post_quote_with_retry(api, 3).await? {
        PostOutcome::Posted => info!("[twitterquote] Quote posted successfully!"),
        PostOutcome::ComposerNotFound => warn!("[twitterquote] Composer not found"),
        PostOutcome::Failed => warn!("[twitterquote] Failed to post quote"),
    }

    api.pause(POST_WAIT_MS).await;
    info!("[twitterquote] Task completed");
    Ok(())
}

fn extract_url_from_payload(payload: &Value) -> Result<StatusUrl> {
    crate::utils::url::extract_url_from_payload(payload).map(StatusUrl::from_unchecked)
}

async fn extract_tweet_context(
    api: &TaskContext,
) -> Result<(String, String, Vec<(String, String)>)> {
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
                                if t.is_empty() {
                                    None
                                } else {
                                    Some((u.to_string(), t.to_string()))
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
    let js = r"
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
    ";
    let result = api.page().evaluate(js).await?;
    let value = result.value();
    if let Some(v) = value {
        if let Some(obj) = v.as_object() {
            if let Some(true) = obj.get("success").and_then(serde_json::Value::as_bool) {
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

async fn post_quote(api: &TaskContext) -> Result<PostOutcome> {
    let outcome = api
        .click("[data-testid=\"retweetConfirm\"], [data-testid=\"tweetButton\"]")
        .await?;
    info!("[twitterquote] Post: {}", outcome.summary());
    Ok(PostOutcome::Posted)
}

async fn post_quote_with_retry(api: &TaskContext, max_retries: u32) -> Result<PostOutcome> {
    let mut last_outcome = PostOutcome::Failed;
    for attempt in 1..=max_retries {
        match post_quote(api).await {
            Ok(PostOutcome::Posted) => return Ok(PostOutcome::Posted),
            Ok(other) => {
                warn!("[twitterquote] Post failed (attempt {attempt}/{max_retries}): {other:?}");
                last_outcome = other;
            }
            Err(e) => {
                warn!("[twitterquote] Post error (attempt {attempt}/{max_retries}): {e}");
            }
        }
        if attempt < max_retries {
            api.pause(2000).await;
        }
    }
    Ok(last_outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_url_from_payload_url() {
        let payload = json!({"url": "https://x.com/user/status/123"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert!(result.as_str().contains("x.com"));
    }

    #[test]
    fn extract_url_from_payload_value() {
        let payload = json!({"value": "https://x.com/user/status/456"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert!(result.as_str().contains("x.com"));
    }

    #[test]
    fn extract_url_from_payload_fallback() {
        let payload = json!({"tweet": "https://x.com/user/status/789"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert!(result.as_str().contains("x.com"));
    }

    #[test]
    fn extract_url_missing() {
        let payload = json!({});
        assert!(extract_url_from_payload(&payload).is_err());
    }

    #[test]
    fn task_duration_stays_within_bounds() {
        let duration_ms = task_duration_ms();
        assert!((36_000..=54_000).contains(&duration_ms));
    }
}
