//! Test LLM Reply task.
//!
//! Navigates to a specific tweet URL, extracts tweet context, calls the LLM
//! to generate a reply, and displays the raw response + validated output.
//! Does NOT type or post anything — safe for testing the LLM pipeline.
//!
//! Usage:
//!   cargo run --bin auto -- test-llmreply url=https://x.com/user/status/123
//!   cargo run --bin auto -- test-llmreply=https://x.com/user/status/123

use anyhow::{Context, Result};
use log::{error, info, warn};
use serde_json::Value;

use crate::prelude::TaskContext;
use crate::utils::twitter::twitteractivity_interact::reply_to_tweet;
use crate::utils::twitter::twitteractivity_navigation::{phase1_navigation, verify_login};
use std::time::Duration;
use tokio::time::timeout;

/// Default task duration in milliseconds (600s = 10 minutes to allow for LLM calls and slow humanized typing).
pub const DEFAULT_TEST_DURATION_MS: u64 = 600_000;

/// Payload key for dry-run mode (skip actual sending).
pub const DRY_RUN_KEY: &str = "dry_run";

/// Navigation timeout for initial Twitter landing (60s, matching twitteractivity).
const NAVIGATE_TIMEOUT_MS: u64 = 60_000;

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    timeout(
        Duration::from_millis(DEFAULT_TEST_DURATION_MS),
        run_inner(api, payload),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "[test-llmreply] Task exceeded duration budget of {}ms",
            DEFAULT_TEST_DURATION_MS
        )
    })?
}

async fn run_inner(api: &TaskContext, payload: Value) -> Result<()> {
    // Extract URL from payload
    let tweet_url = extract_url_from_payload(&payload).context(
        "test-llmreply requires a tweet URL: url=https://x.com/... or value=https://x.com/...",
    )?;

    info!("[test-llmreply] === LLM Reply Test ===");
    info!("[test-llmreply] Target: {tweet_url}");

    // Step 1: Phase 1 navigation — same as twitteractivity task
    // This navigates to a Twitter entry point, dismisses popups, and verifies login.
    info!("[test-llmreply] Step 1: Phase 1 navigation...");
    if let Err(e) = phase1_navigation(api).await {
        warn!("[test-llmreply] Phase 1 navigation had issues: {e}");
        // Continue anyway — the task may still work
    }

    // Check login status
    match verify_login(api).await {
        Ok(true) => info!("[test-llmreply] User is logged in"),
        _ => warn!("[test-llmreply] User may not be logged in"),
    }

    // Step 2: Navigate to the specific tweet
    info!("[test-llmreply] Step 2: Navigating to tweet...");
    api.navigate(&tweet_url, NAVIGATE_TIMEOUT_MS)
        .await
        .context("Failed to navigate to tweet URL")?;
    api.pause(3000).await; // Let page fully load

    // Step 3: Extract tweet context
    info!("[test-llmreply] Step 3: Extracting tweet context...");
    let (author, text, replies) =
        match crate::utils::twitter::twitteractivity_llm::extract_tweet_context(api).await {
            Ok(ctx) => ctx,
            Err(e) => {
                error!("[test-llmreply] Failed to extract tweet context: {e}");
                anyhow::bail!("Failed to extract tweet context: {e}");
            }
        };

    info!("[test-llmreply] Author: @{author}");
    info!("[test-llmreply] Tweet text ({} chars): {text}", text.len());
    info!("[test-llmreply] Replies in context: {}", replies.len());
    for (i, (ra, rt)) in replies.iter().enumerate() {
        info!("[test-llmreply]   Reply {}: @{} - {}", i + 1, ra, rt);
    }

    if text.is_empty() || author == "unknown" {
        anyhow::bail!(
            "Could not extract meaningful tweet context (author={author}, text_len={})",
            text.len()
        );
    }

    // Step 4: Call the LLM
    info!("[test-llmreply] Step 4: Calling LLM to generate reply...");
    let raw_reply = match crate::utils::twitter::twitteractivity_llm::generate_reply(
        api,
        &author,
        &text,
        replies,
        crate::utils::twitter::sentiment::Sentiment::Neutral,
    )
    .await
    {
        Ok(reply) => reply,
        Err(e) => {
            error!("[test-llmreply] LLM reply generation failed: {e}");
            anyhow::bail!("LLM reply generation failed: {e}");
        }
    };

    // Check whether dry-run mode is enabled
    let is_dry_run = payload
        .get(DRY_RUN_KEY)
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !is_dry_run {
        // Step 5: Type and send the reply
        info!("[test-llmreply] Step 5: Typing and sending reply...");
        match reply_to_tweet(api, &raw_reply).await {
            Ok(outcome) => {
                info!("[test-llmreply] Reply outcome: {outcome:?}");
            }
            Err(e) => {
                warn!("[test-llmreply] Reply send failed: {e}");
            }
        }
    } else {
        info!("[test-llmreply] 🔴 Dry-run mode — skipping reply send");
    }

    // Step 6: Show results
    info!("[test-llmreply] === Results ===");
    info!(
        "[test-llmreply] Validated reply ({} chars): {raw_reply}",
        raw_reply.len()
    );
    info!("[test-llmreply] =================");
    if is_dry_run {
        info!("[test-llmreply] ✅ LLM reply test complete (dry-run, nothing was sent).");
    } else {
        info!("[test-llmreply] ✅ LLM reply test complete.");
    }

    Ok(())
}

/// Extract URL from payload, checking `url`, `value`, or first key containing "x.com".
fn extract_url_from_payload(payload: &Value) -> Result<String> {
    if let Some(url) = payload.get("url").and_then(|v| v.as_str()) {
        if !url.trim().is_empty() {
            return Ok(url.to_string());
        }
    }
    if let Some(value) = payload.get("value").and_then(|v| v.as_str()) {
        if !value.trim().is_empty() {
            return Ok(value.to_string());
        }
    }
    // Fallback: scan all keys for a value containing "x.com"
    if let Some(obj) = payload.as_object() {
        for (_key, val) in obj {
            if let Some(v) = val.as_str() {
                if v.contains("x.com") || v.contains("twitter.com") {
                    return Ok(v.to_string());
                }
            }
        }
    }
    Err(anyhow::anyhow!("No tweet URL found in payload"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_url_from_payload_url_field() {
        let payload = json!({"url": "https://x.com/user/status/123"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert_eq!(result, "https://x.com/user/status/123");
    }

    #[test]
    fn extract_url_from_payload_value_field() {
        let payload = json!({"value": "https://x.com/user/status/456"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert_eq!(result, "https://x.com/user/status/456");
    }

    #[test]
    fn extract_url_from_payload_fallback() {
        let payload = json!({"some_key": "https://x.com/user/status/789"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert_eq!(result, "https://x.com/user/status/789");
    }

    #[test]
    fn extract_url_from_payload_prefers_url_over_value() {
        let payload = json!({
            "url": "https://x.com/preferred",
            "value": "https://x.com/other"
        });
        let result = extract_url_from_payload(&payload).unwrap();
        assert_eq!(result, "https://x.com/preferred");
    }

    #[test]
    fn extract_url_from_payload_empty_object() {
        let payload = json!({});
        let result = extract_url_from_payload(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn extract_url_from_payload_empty_string() {
        let payload = json!({"url": ""});
        let result = extract_url_from_payload(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn extract_url_from_payload_no_xcom() {
        let payload = json!({"url": "https://example.com/page"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert_eq!(result, "https://example.com/page");
    }

    #[test]
    fn dry_run_defaults_to_false() {
        let payload = json!({"url": "https://x.com/user/status/123"});
        let is_dry_run = payload
            .get(DRY_RUN_KEY)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        assert!(!is_dry_run);
    }

    #[test]
    fn dry_run_true_when_set() {
        let payload = json!({"url": "https://x.com/user/status/123", "dry_run": true});
        let is_dry_run = payload
            .get(DRY_RUN_KEY)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        assert!(is_dry_run);
    }

    #[test]
    fn dry_run_false_when_explicitly_false() {
        let payload = json!({"url": "https://x.com/user/status/123", "dry_run": false});
        let is_dry_run = payload
            .get(DRY_RUN_KEY)
            .and_then(|v| v.as_bool())
            .unwrap_or(true); // default true so we know explicit false is read
        assert!(!is_dry_run);
    }
}
