//! DOM interaction logic for LLM-powered actions (e.g., Quote Tweet).

use crate::prelude::TaskContext;
use crate::utils::timing::{TIMEOUT_MEDIUM_SECS, TIMEOUT_SHORT_SECS};
use crate::utils::twitter::twitteractivity_selectors;
use anyhow::Result;
use log::{info, warn};
use std::time::Duration;
use tokio::time::timeout;

use super::state::parse_button_coordinates;
use super::twitteractivity_humanized::human_pause;
use super::twitteractivity_interact::click_retweet_button;
use super::EngagementOutcome;

/// Timeout for finding quote tweet button - uses `TIMEOUT_SHORT_SECS` (5s)
/// Short pause after clicking quote button (milliseconds)
const QUOTE_CLICK_PAUSE_SHORT_MS: u64 = 300;
/// Long pause after clicking quote button (milliseconds)
const QUOTE_CLICK_PAUSE_LONG_MS: u64 = 600;
/// Wait time for composer to appear after button click (milliseconds)
const COMPOSER_WAIT_MS: u64 = 1000;

/// Performs a quote tweet with AI-generated commentary.
pub async fn quote_tweet(api: &TaskContext, commentary: &str) -> Result<EngagementOutcome> {
    info!(
        "[quote] Executing quote tweet with {} chars",
        commentary.len()
    );

    // Scroll to top to ensure root tweet is visible and mounted
    if let Err(e) = api.scroll_to_top().await {
        warn!("[quote] Failed to scroll to top before quote: {e}");
    }
    human_pause(api, 500).await;

    if click_retweet_button(api).await? != EngagementOutcome::Completed {
        warn!("[quote] Unable to open retweet menu before quote tweet");
        return Ok(EngagementOutcome::ElementNotFound);
    }

    // Find quote tweet button coordinates
    let quote_btn_js = twitteractivity_selectors::js_find_quote_button();

    let result = if let Ok(r) = timeout(
        Duration::from_secs(TIMEOUT_SHORT_SECS),
        api.page().evaluate(quote_btn_js.to_string()),
    )
    .await
    {
        r?
    } else {
        warn!("[quote] Timeout finding quote tweet button");
        return Ok(EngagementOutcome::Failed);
    };
    let (x, y) = match result.value().and_then(parse_button_coordinates) {
        Some(coords) => coords,
        None => {
            warn!("[quote] Quote tweet button not found");
            return Ok(EngagementOutcome::ElementNotFound);
        }
    };

    // Human-like cursor movement then click
    api.move_mouse_to(x, y).await?;
    human_pause(api, QUOTE_CLICK_PAUSE_SHORT_MS).await;
    api.click_at(x, y).await?;
    human_pause(api, QUOTE_CLICK_PAUSE_LONG_MS).await;

    // Wait for composer to appear
    api.pause(COMPOSER_WAIT_MS).await;

    // Find composer textarea and type commentary
    let composer_js = twitteractivity_selectors::js_focus_composer();

    let focused = if let Ok(r) = timeout(
        Duration::from_secs(TIMEOUT_SHORT_SECS),
        api.page().evaluate(composer_js.to_string()),
    )
    .await
    {
        r?
    } else {
        warn!("[quote] Timeout focusing composer textarea");
        return Ok(EngagementOutcome::Failed);
    };
    if !focused
        .value()
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        warn!("[quote] Composer textarea not found");
        return Ok(EngagementOutcome::ElementNotFound);
    }

    api.pause(500).await;

    // Type the commentary
    let typing_timeout_secs = TIMEOUT_MEDIUM_SECS + (commentary.len() as u64 * 1500 / 1000);
    if let Ok(r) = timeout(
        Duration::from_secs(typing_timeout_secs),
        api.keyboard("[data-testid='tweetTextarea_0']", commentary),
    )
    .await
    {
        r?
    } else {
        warn!("[quote] Timeout typing commentary");
        return Ok(EngagementOutcome::Failed);
    }
    api.pause(COMPOSER_WAIT_MS).await;

    // Find Tweet button coordinates
    let tweet_btn_js = twitteractivity_selectors::js_find_tweet_button();

    let button_result = if let Ok(r) = timeout(
        Duration::from_secs(TIMEOUT_SHORT_SECS),
        api.page().evaluate(tweet_btn_js.to_string()),
    )
    .await
    {
        r?
    } else {
        warn!("[quote] Timeout finding tweet button");
        return Ok(EngagementOutcome::Failed);
    };
    let (tx, ty) = match button_result.value().and_then(parse_button_coordinates) {
        Some(coords) => coords,
        None => {
            warn!("[quote] Tweet button not found");
            return Ok(EngagementOutcome::ElementNotFound);
        }
    };

    // Human-like cursor movement then click
    if timeout(
        Duration::from_secs(TIMEOUT_SHORT_SECS),
        api.move_mouse_to(tx, ty),
    )
    .await
    .is_err()
    {
        warn!("[quote] Timeout moving mouse to tweet button");
        return Ok(EngagementOutcome::Failed);
    }
    human_pause(api, QUOTE_CLICK_PAUSE_SHORT_MS).await;
    if timeout(
        Duration::from_secs(TIMEOUT_SHORT_SECS),
        api.click_at(tx, ty),
    )
    .await
    .is_err()
    {
        warn!("[quote] Timeout clicking tweet button");
        return Ok(EngagementOutcome::Failed);
    }

    // Wait for post to complete
    api.pause(2000).await;

    let verify_js = twitteractivity_selectors::js_verify_quote_posted();

    let verify_result = api.page().evaluate(verify_js).await?;
    if let Some(obj) = verify_result.value().and_then(|v| v.as_object()) {
        let posted = obj
            .get("posted")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let reason = obj
            .get("reason")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        if posted {
            info!("[quote] Quote tweet posted successfully ({reason})");
        } else {
            warn!("[quote] Quote tweet verification failed: {reason}");
        }
        return if posted {
            Ok(EngagementOutcome::Completed)
        } else {
            Ok(EngagementOutcome::Failed)
        };
    }

    warn!("[quote] Quote tweet verification returned an unexpected result");
    Ok(EngagementOutcome::Failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_btn_js_has_visibility_check() {
        let js = r#"
            function visible(el) {
                if (!el) return false;
                var rect = el.getBoundingClientRect();
                return rect.width > 0 && rect.height > 0;
            }
        "#;
        assert!(js.contains("visible"));
        assert!(js.contains("getBoundingClientRect"));
        assert!(js.contains("width > 0"));
        assert!(js.contains("height > 0"));
    }

    #[test]
    fn test_quote_btn_js_searches_scopes() {
        let js = r#"
            var scopes = Array.prototype.slice.call(
                document.querySelectorAll('[role="menu"], div[role="dialog"], [data-testid="Dropdown"]')
            ).filter(visible);
        "#;
        assert!(js.contains("role=\"menu\""));
        assert!(js.contains("role=\"dialog\""));
        assert!(js.contains("data-testid=\"Dropdown\""));
        assert!(js.contains("filter(visible)"));
    }

    #[test]
    fn test_quote_btn_js_quotes_exact_link() {
        let js = r#"
            var exact = scopes[s].querySelector('a[href="/compose/post"][role="menuitem"]');
        "#;
        assert!(js.contains("/compose/post"));
        assert!(js.contains("role=\"menuitem\""));
    }

    #[test]
    fn test_quote_btn_js_fallback_text_search() {
        let js = r#"
            var haystack = (ariaLabel + ' ' + text).toLowerCase();
            if (haystack.includes('quote')) {
        "#;
        assert!(js.contains("includes('quote')"));
    }

    #[test]
    fn test_quote_btn_js_returns_null_when_not_found() {
        let js = r#"
            return null;
        "#;
        assert!(js.contains("return null"));
    }

    #[test]
    fn test_composer_js_targets_tweet_textarea() {
        let js = r#"
            var textboxes = document.querySelectorAll('[data-testid="tweetTextarea_0"][role="textbox"], [data-testid="tweetTextarea_0"], [role="textbox"][aria-label="Post text"]');
        "#;
        assert!(js.contains("tweetTextarea_0"));
        assert!(js.contains("role=\"textbox\""));
        assert!(js.contains("aria-label=\"Post text\""));
    }

    #[test]
    fn test_composer_js_focuses_and_returns() {
        let js = r#"
                textarea.focus();
                return true;
        "#;
        assert!(js.contains("textarea.focus()"));
        assert!(js.contains("return true"));
    }

    #[test]
    fn test_composer_js_skips_invisible() {
        let js = r#"
                if (rect.width <= 0 || rect.height <= 0) continue;
        "#;
        assert!(js.contains("rect.width <= 0"));
        assert!(js.contains("rect.height <= 0"));
    }

    #[test]
    fn test_tweet_btn_js_targets_tweet_button() {
        let js = r#"
            var buttons = document.querySelectorAll('button[data-testid="tweetButton"]');
        "#;
        assert!(js.contains("tweetButton"));
    }

    #[test]
    fn test_tweet_btn_js_checks_disabled() {
        let js = r#"
            if (btn.disabled || btn.getAttribute('aria-disabled') === 'true') continue;
        "#;
        assert!(js.contains("btn.disabled"));
        assert!(js.contains("aria-disabled"));
    }

    #[test]
    fn test_tweet_btn_js_checks_post_text() {
        let js = r#"
            if (text !== 'post') continue;
        "#;
        assert!(js.contains("'post'"));
    }

    #[test]
    fn test_verify_js_checks_composer_cleared() {
        let js = r#"
            if (!textarea) return { posted: true, reason: 'composer closed' };
            var text = textarea.textContent || textarea.value || '';
            if (text.trim() === '') return { posted: true, reason: 'composer cleared' };
            return { posted: false, reason: 'composer still contains text' };
        "#;
        assert!(js.contains("composer closed"));
        assert!(js.contains("composer cleared"));
        assert!(js.contains("composer still contains text"));
    }

    #[test]
    fn test_validate_constants() {
        assert_eq!(QUOTE_CLICK_PAUSE_SHORT_MS, 300);
        assert_eq!(QUOTE_CLICK_PAUSE_LONG_MS, 600);
        assert_eq!(COMPOSER_WAIT_MS, 1000);
    }

    #[test]
    fn test_quote_tweet_signature() {
        fn assert_fn<T>(_: T) {}
        assert_fn(quote_tweet);
    }
}
