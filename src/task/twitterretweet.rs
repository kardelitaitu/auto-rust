//! Twitter retweet task.
//! Retweets a tweet with optional quote commentary.

use crate::llm::{ChatMessage, Llm};
use crate::prelude::TaskContext;
use crate::utils::timing::{
    duration_with_variance, run_with_timeout, DEFAULT_NAVIGATION_TIMEOUT_MS,
};
use crate::utils::twitter::reply_engine::reply_engine_system_prompt;
use crate::utils::twitter::twitteractivity_llm::validate_reply;
use crate::utils::twitter::{ComposerFlow, PostOutcome, StatusUrl};
use anyhow::Result;
use log::{info, warn};
use serde_json::Value;

const POST_WAIT_MS: u64 = 5000;
pub const DEFAULT_TWITTERRETWEET_TASK_DURATION_MS: u64 = 45_000;

fn task_duration_ms() -> u64 {
    duration_with_variance(DEFAULT_TWITTERRETWEET_TASK_DURATION_MS, 20)
}

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let duration_ms = task_duration_ms();
    run_with_timeout(duration_ms, "twitterretweet", run_inner(api, payload)).await
}

async fn run_inner(api: &TaskContext, payload: Value) -> Result<()> {
    let tweet_url = extract_url_from_payload(&payload)?;
    let quote_text = payload
        .get("quote_text")
        .and_then(|v| v.as_str())
        .map(std::string::ToString::to_string);
    let use_llm = payload
        .get("use_llm")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);

    info!("[twitterretweet] Task started - target: {tweet_url}");

    // Navigate to tweet
    info!("[twitterretweet] Navigating to tweet...");
    api.navigate(&tweet_url, DEFAULT_NAVIGATION_TIMEOUT_MS)
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
            if tweet_text.is_empty() || author == "unknown" {
                anyhow::bail!(
                    "[twitterretweet] Failed to extract valid tweet context: empty text or unknown author (author={author})"
                );
            }
            info!("[twitterretweet] Tweet by @{author}");

            let llm = Llm::new()?;
            let messages = build_quote_messages(&author, &tweet_text);

            match llm.chat(messages).await {
                Ok(text) => {
                    validate_reply(&text).unwrap_or_else(|_| "Interesting take!".to_string())
                }
                Err(e) => {
                    warn!("[twitterretweet] LLM failed: {e}, using fallback");
                    "Interesting take!".to_string()
                }
            }
        } else {
            "Interesting take!".to_string()
        };

        info!("[twitterretweet] Quote text: {commentary}");

        let mut flow = ComposerFlow::new();

        // Click quote button
        info!("[twitterretweet] Clicking quote button...");
        click_quote_button(api).await?;
        flow.record_composer_opened()?;
        api.pause(1500).await;

        // Type quote
        info!("[twitterretweet] Typing quote...");
        type_quote(api, &commentary).await?;
        flow.record_text_entered()?;
        api.pause(1000).await;

        // Post
        info!("[twitterretweet] Posting quote...");
        match post_quote_with_retry(api, 3).await? {
            PostOutcome::Posted => {
                flow.record_posted()?;
                info!("[twitterretweet] Quote posted successfully!");
            }
            PostOutcome::ComposerNotFound => warn!("[twitterretweet] Composer not found"),
            PostOutcome::Failed => warn!("[twitterretweet] Failed to post quote"),
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
        if clicked
            .value()
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
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
            if confirmed
                .value()
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
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

fn extract_url_from_payload(payload: &Value) -> Result<StatusUrl> {
    crate::utils::url::extract_url_from_payload(payload).map(StatusUrl::from_unchecked)
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

async fn post_quote(api: &TaskContext) -> Result<PostOutcome> {
    let outcome = api
        .click("[data-testid=\"retweetConfirm\"], [data-testid=\"tweetButton\"]")
        .await?;
    info!("[twitterretweet] Post: {}", outcome.summary());
    Ok(PostOutcome::Posted)
}

async fn post_quote_with_retry(api: &TaskContext, max_retries: u32) -> Result<PostOutcome> {
    let mut last_outcome = PostOutcome::Failed;
    for attempt in 1..=max_retries {
        match post_quote(api).await {
            Ok(PostOutcome::Posted) => return Ok(PostOutcome::Posted),
            Ok(other) => {
                warn!("[twitterretweet] Post failed (attempt {attempt}/{max_retries}): {other:?}");
                last_outcome = other;
            }
            Err(e) => {
                warn!("[twitterretweet] Post error (attempt {attempt}/{max_retries}): {e}");
            }
        }
        if attempt < max_retries {
            api.pause(2000).await;
        }
    }
    Ok(last_outcome)
}

fn build_quote_messages(tweet_author: &str, tweet_text: &str) -> Vec<ChatMessage> {
    let system = reply_engine_system_prompt();
    let user = format!(
        "Quote this tweet by @{tweet_author}:\n{tweet_text}\n\nGenerate a short, engaging quote commentary (max 280 chars):"
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
        assert!(result.as_str().contains("x.com"));
    }

    #[test]
    fn extract_url_from_payload_value() {
        let payload = json!({"value": "https://x.com/user/status/456"});
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
