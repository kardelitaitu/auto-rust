//! DOM interaction logic for LLM-powered actions (e.g., Quote Tweet).

use crate::prelude::TaskContext;
use crate::utils::timing::{TIMEOUT_MEDIUM_SECS, TIMEOUT_SHORT_SECS};
use anyhow::Result;
use log::{info, warn};
use std::time::Duration;
use tokio::time::timeout;

use super::twitteractivity_humanized::human_pause;
use super::twitteractivity_interact::click_retweet_button;

/// Timeout for finding quote tweet button - uses TIMEOUT_SHORT_SECS (5s)
/// Short pause after clicking quote button (milliseconds)
const QUOTE_CLICK_PAUSE_SHORT_MS: u64 = 300;
/// Long pause after clicking quote button (milliseconds)
const QUOTE_CLICK_PAUSE_LONG_MS: u64 = 600;
/// Wait time for composer to appear after button click (milliseconds)
const COMPOSER_WAIT_MS: u64 = 1000;

/// Performs a quote tweet with AI-generated commentary.
pub async fn quote_tweet(api: &TaskContext, commentary: &str) -> Result<bool> {
    info!("Executing quote tweet with {} chars", commentary.len());

    if !click_retweet_button(api).await? {
        warn!("Unable to open retweet menu before quote tweet");
        return Ok(false);
    }

    // Find quote tweet button coordinates
    let quote_btn_js = r#"
        (function() {
            function visible(el) {
                if (!el) return false;
                var rect = el.getBoundingClientRect();
                return rect.width > 0 && rect.height > 0;
            }
            var scopes = Array.prototype.slice.call(
                document.querySelectorAll('[role="menu"], div[role="dialog"], [data-testid="Dropdown"]')
            ).filter(visible);
            if (scopes.length === 0) scopes = [document.body];

            var buttons = [];
            for (var s = 0; s < scopes.length; s++) {
                var exact = scopes[s].querySelector('a[href="/compose/post"][role="menuitem"]');
                var exactText = exact ? (exact.textContent || exact.innerText || '').trim().toLowerCase() : '';
                if (visible(exact) && exactText.includes('quote')) {
                    return center(exact);
                }
                buttons = buttons.concat(Array.prototype.slice.call(scopes[s].querySelectorAll('[role="button"], [role="menuitem"]')));
            }
            for (var i = 0; i < buttons.length; i++) {
                var btn = buttons[i];
                var ariaLabel = btn.getAttribute('aria-label') || '';
                var text = btn.textContent || btn.innerText || '';
                var haystack = (ariaLabel + ' ' + text).toLowerCase();
                if (haystack.includes('quote')) {
                    if (visible(btn)) return center(btn);
                }
            }
            return null;

            function center(el) {
                var rect = el.getBoundingClientRect();
                return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
            }
        })()
    "#;

    let result = match timeout(
        Duration::from_secs(TIMEOUT_SHORT_SECS),
        api.page().evaluate(quote_btn_js.to_string()),
    )
    .await
    {
        Ok(r) => r?,
        Err(_) => {
            warn!("Timeout finding quote tweet button");
            return Ok(false);
        }
    };
    let coords = result.value().and_then(|v| v.as_object());

    let (x, y) = if let Some(obj) = coords {
        (
            obj.get("x").and_then(|v| v.as_f64()),
            obj.get("y").and_then(|v| v.as_f64()),
        )
    } else {
        (None, None)
    };

    let (x, y) = match (x, y) {
        (Some(x), Some(y)) => (x, y),
        _ => {
            warn!("Quote tweet button not found");
            return Ok(false);
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
    let composer_js = r#"
        (function() {
            var textboxes = document.querySelectorAll('[data-testid="tweetTextarea_0"][role="textbox"], [data-testid="tweetTextarea_0"], [role="textbox"][aria-label="Post text"]');
            for (var i = 0; i < textboxes.length; i++) {
                var textarea = textboxes[i];
                var rect = textarea.getBoundingClientRect();
                if (rect.width <= 0 || rect.height <= 0) continue;
                textarea.focus();
                return true;
            }
            return false;
        })()
    "#;

    let focused = match timeout(
        Duration::from_secs(TIMEOUT_SHORT_SECS),
        api.page().evaluate(composer_js.to_string()),
    )
    .await
    {
        Ok(r) => r?,
        Err(_) => {
            warn!("Timeout focusing composer textarea");
            return Ok(false);
        }
    };
    if !focused.value().and_then(|v| v.as_bool()).unwrap_or(false) {
        warn!("Composer textarea not found");
        return Ok(false);
    }

    api.pause(500).await;

    // Type the commentary
    match timeout(
        Duration::from_secs(TIMEOUT_MEDIUM_SECS),
        api.keyboard("[data-testid='tweetTextarea_0']", commentary),
    )
    .await
    {
        Ok(r) => r?,
        Err(_) => {
            warn!("Timeout typing commentary");
            return Ok(false);
        }
    }
    api.pause(COMPOSER_WAIT_MS).await;

    // Find Tweet button coordinates
    let tweet_btn_js = r#"
        (function() {
            var buttons = document.querySelectorAll('button[data-testid="tweetButton"]');
            for (var i = 0; i < buttons.length; i++) {
                var btn = buttons[i];
                var rect = btn.getBoundingClientRect();
                if (rect.width <= 0 || rect.height <= 0) continue;
                if (btn.disabled || btn.getAttribute('aria-disabled') === 'true') continue;
                var text = (btn.textContent || btn.innerText || '').trim().toLowerCase();
                if (text !== 'post') continue;
                return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
            }
            return null;
        })()
    "#;

    let button_result = match timeout(
        Duration::from_secs(TIMEOUT_SHORT_SECS),
        api.page().evaluate(tweet_btn_js.to_string()),
    )
    .await
    {
        Ok(r) => r?,
        Err(_) => {
            warn!("Timeout finding tweet button");
            return Ok(false);
        }
    };
    let coords = button_result.value().and_then(|v| v.as_object());

    let (tx, ty) = if let Some(obj) = coords {
        (
            obj.get("x").and_then(|v| v.as_f64()),
            obj.get("y").and_then(|v| v.as_f64()),
        )
    } else {
        (None, None)
    };

    let (tx, ty) = match (tx, ty) {
        (Some(tx), Some(ty)) => (tx, ty),
        _ => {
            warn!("Tweet button not found");
            return Ok(false);
        }
    };

    // Human-like cursor movement then click
    match timeout(
        Duration::from_secs(TIMEOUT_SHORT_SECS),
        api.move_mouse_to(tx, ty),
    )
    .await
    {
        Ok(_) => {}
        Err(_) => {
            warn!("Timeout moving mouse to tweet button");
            return Ok(false);
        }
    }
    human_pause(api, QUOTE_CLICK_PAUSE_SHORT_MS).await;
    match timeout(
        Duration::from_secs(TIMEOUT_SHORT_SECS),
        api.click_at(tx, ty),
    )
    .await
    {
        Ok(_) => {}
        Err(_) => {
            warn!("Timeout clicking tweet button");
            return Ok(false);
        }
    }

    // Wait for post to complete
    api.pause(2000).await;

    let verify_js = r#"
        (function() {
            var textarea = document.querySelector('[data-testid="tweetTextarea_0"]') ||
                           document.querySelector('[role="textbox"]');
            if (!textarea) return { posted: true, reason: 'composer closed' };
            var text = textarea.textContent || textarea.value || '';
            if (text.trim() === '') return { posted: true, reason: 'composer cleared' };
            return { posted: false, reason: 'composer still contains text' };
        })()
    "#;

    let verify_result = api.page().evaluate(verify_js).await?;
    if let Some(obj) = verify_result.value().and_then(|v| v.as_object()) {
        let posted = obj
            .get("posted")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let reason = obj
            .get("reason")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        if posted {
            info!("Quote tweet posted successfully ({})", reason);
        } else {
            warn!("Quote tweet verification failed: {}", reason);
        }
        return Ok(posted);
    }

    warn!("Quote tweet verification returned an unexpected result");
    Ok(false)
}
