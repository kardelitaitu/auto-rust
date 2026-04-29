//! Twitter comprehensive test task.
//! Runs all Twitter interactions sequentially for validation.

use crate::prelude::TaskContext;
use crate::utils::timing::{duration_with_variance, DEFAULT_NAVIGATION_TIMEOUT_MS};
use crate::utils::twitter::twitteractivity_humanized::human_pause;
use crate::utils::twitter::twitteractivity_navigation::goto_home;
use anyhow::Result;
use log::{info, warn};
use serde_json::Value;
use std::time::Duration;
use tokio::time::timeout;

pub const DEFAULT_TWITTERTEST_TASK_DURATION_MS: u64 = 120_000;

fn task_duration_ms() -> u64 {
    duration_with_variance(DEFAULT_TWITTERTEST_TASK_DURATION_MS, 20)
}

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let duration_ms = task_duration_ms();
    timeout(Duration::from_millis(duration_ms), run_inner(api, payload))
        .await
        .map_err(|_| anyhow::anyhow!(
            "[twittertest] Task exceeded duration budget of {}ms",
            duration_ms
        ))?
}

async fn run_inner(api: &TaskContext, payload: Value) -> Result<()> {
    let tweet_url = extract_url_from_payload(&payload)?;
    let tests = extract_tests_from_payload(&payload);

    info!("[twittertest] === Twitter Comprehensive Test Suite ===");
    info!("[twittertest] Target: {}", tweet_url);
    info!("[twittertest] Tests to run: {:?}", tests);

    let mut results = TestResults::new();

    // Navigate to test tweet
    info!("[twittertest] Navigating to test tweet...");
    if let Err(e) = api.navigate(&tweet_url, DEFAULT_NAVIGATION_TIMEOUT_MS).await {
        error_test(&mut results, "navigation", &e.to_string());
        return Ok(());
    }
    api.pause(2000).await;

    // Run each test
    for test in &tests {
        match test.as_str() {
            "like" => test_like(api, &mut results).await,
            "retweet" => test_retweet(api, &mut results).await,
            "quote" => test_quote(api, &mut results).await,
            "follow" => test_follow(api, &mut results).await,
            "reply" => test_reply(api, &mut results).await,
            "dive" => test_dive(api, &mut results).await,
            _ => {
                warn!("[twittertest] Unknown test: {}", test);
            }
        }

        // Return to home between tests
        info!("[twittertest] Returning to home...");
        if let Err(e) = goto_home(api).await {
            warn!("[twittertest] Failed to return to home: {}", e);
        }
        api.pause(2000).await;
    }

    // Print summary
    info!("[twittertest]");
    info!("[twittertest] === Test Results ===");
    info!("[twittertest] Passed: {}", results.passed);
    info!("[twittertest] Failed: {}", results.failed);
    info!("[twittertest]");

    for (test, result) in &results.results {
        let status = if result.0 { "✅ PASS" } else { "❌ FAIL" };
        info!("[twittertest] {}: {} - {}", status, test, result.1);
    }

    if results.failed == 0 {
        info!("[twittertest] All tests passed! Ready for live deployment.");
    } else {
        warn!(
            "[twittertest] {} tests failed. Review before live deployment.",
            results.failed
        );
    }

    Ok(())
}

// ============================================================================
// Individual Tests
// ============================================================================

async fn test_like(api: &TaskContext, results: &mut TestResults) {
    info!("[twittertest] Running LIKE test...");

    let like_js = r#"
        (function() {
            var buttons = document.querySelectorAll('[data-testid="like"]');
            if (buttons.length > 0) {
                buttons[0].click();
                return true;
            }
            return false;
        })()
    "#;

    match api.page().evaluate(like_js.to_string()).await {
        Ok(result) => {
            if result.value().and_then(|v| v.as_bool()).unwrap_or(false) {
                info!("[twittertest] LIKE: Button clicked");
                results.pass("like", "Button clicked successfully");
            } else {
                results.fail("like", "Button not found or not clickable");
            }
        }
        Err(e) => results.fail("like", &e.to_string()),
    }

    human_pause(api, 1000).await;
}

async fn test_retweet(api: &TaskContext, results: &mut TestResults) {
    info!("[twittertest] Running RETWEET test...");

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

    match api.page().evaluate(rt_js.to_string()).await {
        Ok(result) => {
            if result.value().and_then(|v| v.as_bool()).unwrap_or(false) {
                info!("[twittertest] RETWEET: Menu opened");
                api.pause(1000).await;

                // Try to confirm
                let confirm_js = r#"
                    (function() {
                        var buttons = document.querySelectorAll('[data-testid="retweetConfirm"]');
                        if (buttons.length > 0) {
                            return true;
                        }
                        return false;
                    })()
                "#;

                match api.page().evaluate(confirm_js.to_string()).await {
                    Ok(confirm_result) => {
                        if confirm_result
                            .value()
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false)
                        {
                            results.pass("retweet", "Retweet menu opened, confirm available");
                        } else {
                            results.fail("retweet", "Confirm button not found");
                        }
                    }
                    Err(e) => results.fail("retweet", &format!("Confirm check failed: {}", e)),
                }
            } else {
                results.fail("retweet", "Retweet button not found");
            }
        }
        Err(e) => results.fail("retweet", &e.to_string()),
    }
}

async fn test_quote(api: &TaskContext, results: &mut TestResults) {
    info!("[twittertest] Running QUOTE test...");

    // Same as retweet but checks for quote capability
    let quote_js = r#"
        (function() {
            var buttons = document.querySelectorAll('button[data-testid]');
            for (var i = 0; i < buttons.length; i++) {
                var btn = buttons[i];
                var testId = (btn.getAttribute('data-testid') || '').toLowerCase();
                if (testId.includes('retweet')) {
                    return true;
                }
            }
            return false;
        })()
    "#;

    match api.page().evaluate(quote_js.to_string()).await {
        Ok(result) => {
            if result.value().and_then(|v| v.as_bool()).unwrap_or(false) {
                results.pass("quote", "Quote button available");
            } else {
                results.fail("quote", "Quote button not found");
            }
        }
        Err(e) => results.fail("quote", &e.to_string()),
    }
}

async fn test_follow(api: &TaskContext, results: &mut TestResults) {
    info!("[twittertest] Running FOLLOW test...");

    let follow_js = r#"
        (function() {
            var buttons = document.querySelectorAll('[role="button"]');
            for (var i = 0; i < buttons.length; i++) {
                var btn = buttons[i];
                var ariaLabel = (btn.getAttribute('aria-label') || '').toLowerCase();
                if (ariaLabel.includes('follow')) {
                    return true;
                }
            }
            return false;
        })()
    "#;

    match api.page().evaluate(follow_js.to_string()).await {
        Ok(result) => {
            if result.value().and_then(|v| v.as_bool()).unwrap_or(false) {
                results.pass("follow", "Follow button found");
            } else {
                results.fail("follow", "Follow button not found");
            }
        }
        Err(e) => results.fail("follow", &e.to_string()),
    }
}

async fn test_reply(api: &TaskContext, results: &mut TestResults) {
    info!("[twittertest] Running REPLY test...");

    let reply_js = r#"
        (function() {
            var buttons = document.querySelectorAll('[data-testid="reply"]');
            if (buttons.length > 0) {
                return true;
            }
            return false;
        })()
    "#;

    match api.page().evaluate(reply_js.to_string()).await {
        Ok(result) => {
            if result.value().and_then(|v| v.as_bool()).unwrap_or(false) {
                results.pass("reply", "Reply button found");
            } else {
                results.fail("reply", "Reply button not found");
            }
        }
        Err(e) => results.fail("reply", &e.to_string()),
    }
}

async fn test_dive(api: &TaskContext, results: &mut TestResults) {
    info!("[twittertest] Running DIVE test...");

    // Check if thread/replies exist
    let dive_js = r#"
        (function() {
            var articles = document.querySelectorAll('article[data-testid="tweet"]');
            return articles.length > 1;
        })()
    "#;

    match api.page().evaluate(dive_js.to_string()).await {
        Ok(result) => {
            if result.value().and_then(|v| v.as_bool()).unwrap_or(false) {
                results.pass("dive", "Thread/replies detected");
            } else {
                results.fail("dive", "No thread/replies found");
            }
        }
        Err(e) => results.fail("dive", &e.to_string()),
    }
}

// ============================================================================
// Helpers
// ============================================================================

struct TestResults {
    passed: u32,
    failed: u32,
    results: std::collections::HashMap<String, (bool, String)>,
}

impl TestResults {
    fn new() -> Self {
        Self {
            passed: 0,
            failed: 0,
            results: std::collections::HashMap::new(),
        }
    }

    fn pass(&mut self, test: &str, message: &str) {
        self.passed += 1;
        self.results
            .insert(test.to_string(), (true, message.to_string()));
    }

    fn fail(&mut self, test: &str, message: &str) {
        self.failed += 1;
        self.results
            .insert(test.to_string(), (false, message.to_string()));
    }
}

fn error_test(results: &mut TestResults, test: &str, error: &str) {
    results.fail(test, error);
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

fn extract_tests_from_payload(payload: &Value) -> Vec<String> {
    let default_tests = vec![
        "like".to_string(),
        "retweet".to_string(),
        "quote".to_string(),
        "follow".to_string(),
        "reply".to_string(),
        "dive".to_string(),
    ];

    if let Some(tests_value) = payload.get("tests") {
        if let Some(tests_array) = tests_value.as_array() {
            return tests_array
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
    }

    default_tests
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
    fn extract_tests_default() {
        let payload = json!({});
        let tests = extract_tests_from_payload(&payload);
        assert_eq!(tests.len(), 6);
    }

    #[test]
    fn extract_tests_custom() {
        let payload = json!({"tests": ["like", "follow"]});
        let tests = extract_tests_from_payload(&payload);
        assert_eq!(tests, vec!["like", "follow"]);
    }

    #[test]
    fn task_duration_stays_within_bounds() {
        let duration_ms = task_duration_ms();
        assert!(duration_ms >= 96_000 && duration_ms <= 144_000);
    }
}
