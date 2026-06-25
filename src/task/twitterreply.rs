use crate::internal::text::{preview_chars, truncate_with_ellipsis};
use crate::llm::unified_processor::UnifiedLLMProcessor;
use crate::prelude::TaskContext;
use crate::utils::timing::{
    duration_with_variance, run_with_timeout, DEFAULT_NAVIGATION_TIMEOUT_MS,
};
use crate::utils::twitter::{ComposerFlow, PostOutcome, StatusUrl};
use anyhow::Result;
use log::{info, warn};
use rand::Rng;
use serde_json::Value;

const CONTEXT_REPLIES: u32 = 5;
const POST_WAIT_MS: u64 = 5000;
pub const DEFAULT_TWITTERREPLY_TASK_DURATION_MS: u64 = 45_000;

fn task_duration_ms() -> u64 {
    duration_with_variance(DEFAULT_TWITTERREPLY_TASK_DURATION_MS, 20)
}

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let duration_ms = task_duration_ms();
    run_with_timeout(duration_ms, "twitterreply", run_inner(api, payload)).await
}

async fn run_inner(api: &TaskContext, payload: Value) -> Result<()> {
    let tweet_url = extract_url_from_payload(&payload)?;
    info!("Task started - target: {tweet_url}");

    info!("Trampoline navigation...");
    api.navigate(&tweet_url, DEFAULT_NAVIGATION_TIMEOUT_MS)
        .await?;
    api.pause(2000).await;

    info!("Scrolling down (10-20s)...");
    scroll_down_random(api).await?;

    info!("Scrolling up (5-10s)...");
    scroll_up_faster(api).await?;

    info!("Extracting tweet and context...");
    let (author, tweet_text) = extract_main_tweet(api).await?;
    info!("Tweet by @{}: {}", author, preview_chars(&tweet_text, 50));

    let replies = extract_replies(api, CONTEXT_REPLIES).await?;
    info!("Extracted {} replies for context", replies.len());

    info!("Generating AI reply using unified batch processor...");
    let processor = UnifiedLLMProcessor::new()?;

    // Convert replies to format expected by unified processor
    let reply_tuples: Vec<(&str, &str)> = replies
        .iter()
        .map(|(a, t)| (a.as_str(), t.as_str()))
        .collect();

    let reply_texts: Vec<crate::llm::processor::UnifiedReplyResponse> = processor
        .process_replies_batch(&tweet_text, &author, &reply_tuples)
        .await
        .map_err(|e| {
            warn!("Unified processor failed: {e}, using fallback");
            e
        })?;

    // Use first reply (most relevant to the tweet)
    let reply_text = if let Some(first_reply) = reply_texts.first() {
        sanitize_reply(&first_reply.content)
    } else {
        warn!("No replies generated, using fallback");
        "Interesting perspective! Thanks for sharing.".to_string()
    };

    info!("AI Reply: {reply_text}");

    let mut flow = ComposerFlow::new();

    info!("Clicking reply button...");
    click_reply_button(api).await?;
    flow.record_composer_opened()?;

    api.pause(1500).await;

    info!("Typing reply...");
    type_reply(api, &reply_text).await?;
    flow.record_text_entered()?;

    api.pause(1000).await;

    info!("Posting reply...");
    match post_reply_with_retry(api, 3).await? {
        PostOutcome::Posted => {
            flow.record_posted()?;
            info!("📝 [SUCCESS] Reply posted successfully!");
        }
        PostOutcome::ComposerNotFound => warn!("Composer not found"),
        PostOutcome::Failed => warn!("Failed to post reply"),
    }

    api.pause(POST_WAIT_MS).await;
    info!("Task completed");
    Ok(())
}

fn extract_url_from_payload(payload: &Value) -> Result<StatusUrl> {
    crate::utils::url::extract_url_from_payload(payload).map(StatusUrl::from_unchecked)
}

async fn extract_main_tweet(api: &TaskContext) -> Result<(String, String)> {
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

    Err(anyhow::anyhow!("Could not extract tweet"))
}

async fn extract_replies(api: &TaskContext, limit: u32) -> Result<Vec<(String, String)>> {
    let page = api.page();

    let js = format!(
        r#"
        (function() {{
            var articles = document.querySelectorAll('article[data-testid="tweet"]');
            var replies = [];
            for (var i = 1; i < Math.min(articles.length, {}); i++) {{
                var article = articles[i];
                var nameLink = article.querySelector('a[href^="/"]');
                var username = nameLink ? nameLink.getAttribute('href').split('/')[1] : 'unknown';
                var textContent = article.querySelectorAll('[data-testid="tweetText"]');
                var text = textContent.length > 0 ? textContent[textContent.length - 1].innerText : '';
                if (text) replies.push([username, text]);
            }}
            return replies;
        }})()
"#,
        limit + 1
    );

    let result = page.evaluate(js).await?;

    let value = result.value();
    let mut replies = Vec::new();

    if let Some(v) = value {
        if let Some(arr) = v.as_array() {
            for item in arr {
                if let Some(obj) = item.as_array() {
                    if obj.len() >= 2 {
                        let username = obj[0].as_str().unwrap_or("unknown");
                        let text = obj[1].as_str().unwrap_or("");
                        replies.push((username.to_string(), text.to_string()));
                    }
                }
            }
        }
    }

    Ok(replies)
}

async fn click_reply_button(api: &TaskContext) -> Result<()> {
    let outcome = api.click("[data-testid=\"reply\"]").await?;
    info!("Reply {}", outcome.summary());
    Ok(())
}

async fn type_reply(api: &TaskContext, text: &str) -> Result<()> {
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

async fn post_reply(api: &TaskContext) -> Result<PostOutcome> {
    let outcome = api
        .click("[data-testid=\"tweetButton\"], [data-testid=\"tweetButtonInline\"]")
        .await?;
    info!("Post {}", outcome.summary());
    Ok(PostOutcome::Posted)
}

async fn post_reply_with_retry(api: &TaskContext, max_retries: u32) -> Result<PostOutcome> {
    let mut last_outcome = PostOutcome::Failed;
    for attempt in 1..=max_retries {
        match post_reply(api).await {
            Ok(PostOutcome::Posted) => return Ok(PostOutcome::Posted),
            Ok(other) => {
                warn!("Post failed (attempt {attempt}/{max_retries}): {other:?}");
                last_outcome = other;
            }
            Err(e) => {
                warn!("Post error (attempt {attempt}/{max_retries}): {e}");
            }
        }
        if attempt < max_retries {
            api.pause(2000).await;
        }
    }
    Ok(last_outcome)
}

fn sanitize_reply(text: &str) -> String {
    let mut result = text.trim().to_string();

    result = result.trim_start_matches('"').to_string();
    result = result.trim_end_matches('"').to_string();

    if result.ends_with('.') {
        result.pop();
    }

    result = truncate_with_ellipsis(&result, 280);

    result
}

async fn scroll_down_random(api: &TaskContext) -> Result<()> {
    let duration_ms = rand::thread_rng().gen_range(10000..=20000);
    let page = api.page();
    let start = std::time::Instant::now();

    while start.elapsed().as_millis() < duration_ms {
        let scroll_amount = rand::thread_rng().gen_range(200..=500);
        let js = format!("window.scrollBy(0, {scroll_amount})");
        page.evaluate(js).await?;
        api.pause(200).await;
    }

    Ok(())
}

async fn scroll_up_faster(api: &TaskContext) -> Result<()> {
    let duration_ms = rand::thread_rng().gen_range(5000..=10000);
    let page = api.page();
    let start = std::time::Instant::now();

    while start.elapsed().as_millis() < duration_ms {
        let scroll_amount = rand::thread_rng().gen_range(400..=800);
        let js = format!("window.scrollBy(0, -{scroll_amount})");
        page.evaluate(js).await?;
        api.pause(100).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{sanitize_reply, task_duration_ms};
    use crate::internal::text::preview_chars;

    #[test]
    fn preview_text_is_char_safe() {
        assert_eq!(preview_chars("naïve🙂text", 6), "naïve🙂");
    }

    #[test]
    fn sanitize_reply_truncates_by_chars() {
        let text = "🙂".repeat(300);
        let out = sanitize_reply(&text);
        assert_eq!(out.chars().count(), 280);
        assert!(out.ends_with("..."));
    }

    #[test]
    fn task_duration_stays_within_bounds() {
        let duration_ms = task_duration_ms();
        assert!((36_000..=54_000).contains(&duration_ms));
    }
}
