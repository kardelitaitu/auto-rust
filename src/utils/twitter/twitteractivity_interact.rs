//! Interaction helpers for Twitter/X automation.
//! Like, retweet, follow, and reply operations with human-like timing.

use crate::prelude::TaskContext;
use anyhow::Result;

use super::twitteractivity_humanized::*;

/// Clicks the "like" (heart) button on the current tweet.
/// Uses proper filtering then Coords + api.click_at.
/// Returns true if the like action appears to have been successful.
pub async fn like_tweet(api: &TaskContext) -> Result<bool> {
    // Find like button with proper filtering, return coords
    let js = r#"
        (function() {
            var buttons = document.querySelectorAll('button[data-testid], a[data-testid]');
            for (var i = 0; i < buttons.length; i++) {
                var el = buttons[i];
                var testId = (el.getAttribute('data-testid') || '').toLowerCase();
                if (testId.includes('like') && !testId.includes('unlike')) {
                    var rect = el.getBoundingClientRect();
                    if (rect.width > 0 && rect.height > 0) {
                        return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
                    }
                }
            }
            return null;
        })()
    "#;
    let result = api.page().evaluate(js).await?;

    if let Some(obj) = result.value().and_then(|v| v.as_object()) {
        if let (Some(x), Some(y)) = (
            obj.get("x").and_then(|v| v.as_f64()),
            obj.get("y").and_then(|v| v.as_f64()),
        ) {
            api.move_mouse_to(x, y).await?;
            human_pause(api, 250).await;
            api.click_at(x, y).await?;
            human_pause(api, 600).await;
            return Ok(true);
        }
    }
    Ok(false)
}

/// Clicks the "retweet" button on the current tweet.
/// Uses proper filtering then Coords + api.click_at.
/// Note: Does not confirm the retweet in the modal; just opens the retweet menu.
pub async fn click_retweet_button(api: &TaskContext) -> Result<bool> {
    let js = r#"
        (function() {
            var buttons = document.querySelectorAll('button[data-testid], a[data-testid]');
            for (var i = 0; i < buttons.length; i++) {
                var el = buttons[i];
                var testId = (el.getAttribute('data-testid') || '').toLowerCase();
                if (testId.includes('retweet') && !testId.includes('unretweet')) {
                    var rect = el.getBoundingClientRect();
                    if (rect.width > 0 && rect.height > 0) {
                        return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
                    }
                }
            }
            return null;
        })()
    "#;
    let result = api.page().evaluate(js).await?;

    if let Some(obj) = result.value().and_then(|v| v.as_object()) {
        if let (Some(x), Some(y)) = (
            obj.get("x").and_then(|v| v.as_f64()),
            obj.get("y").and_then(|v| v.as_f64()),
        ) {
            api.move_mouse_to(x, y).await?;
            human_pause(api, 250).await;
            api.click_at(x, y).await?;
            human_pause(api, 600).await;
            return Ok(true);
        }
    }
    Ok(false)
}

/// Confirms a retweet from the retweet modal.
pub async fn confirm_retweet(api: &TaskContext) -> Result<bool> {
    let js = r#"
        (function() {
            var btn = document.querySelector('button[data-testid="retweetConfirm"]');
            if (!btn) return null;
            var rect = btn.getBoundingClientRect();
            return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
        })()
    "#;
    let result = api.page().evaluate(js).await?;

    if let Some(obj) = result.value().and_then(|v| v.as_object()) {
        if let (Some(x), Some(y)) = (
            obj.get("x").and_then(|v| v.as_f64()),
            obj.get("y").and_then(|v| v.as_f64()),
        ) {
            api.move_mouse_to(x, y).await?;
            human_pause(api, 200).await;
            api.click_at(x, y).await?;
            human_pause(api, 800).await;
            return Ok(true);
        }
    }
    Ok(false)
}

/// Full retweet action: click retweet then confirm.
pub async fn retweet_tweet(api: &TaskContext) -> Result<bool> {
    if click_retweet_button(api).await? {
        human_pause(api, 500).await;
        return confirm_retweet(api).await;
    }
    Ok(false)
}

/// Clicks the "reply" button on the current tweet.
/// Uses proper filtering then Coords + api.click_at.
pub async fn click_reply_button(api: &TaskContext) -> Result<bool> {
    let js = r#"
        (function() {
            var buttons = document.querySelectorAll('button[data-testid], a[data-testid]');
            for (var i = 0; i < buttons.length; i++) {
                var el = buttons[i];
                var testId = (el.getAttribute('data-testid') || '').toLowerCase();
                if (testId.includes('reply') || testId.includes('comment')) {
                    var rect = el.getBoundingClientRect();
                    if (rect.width > 0 && rect.height > 0) {
                        return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
                    }
                }
            }
            return null;
        })()
    "#;
    let result = api.page().evaluate(js).await?;

    if let Some(obj) = result.value().and_then(|v| v.as_object()) {
        if let (Some(x), Some(y)) = (
            obj.get("x").and_then(|v| v.as_f64()),
            obj.get("y").and_then(|v| v.as_f64()),
        ) {
            api.move_mouse_to(x, y).await?;
            human_pause(api, 250).await;
            api.click_at(x, y).await?;
            human_pause(api, 500).await;
            return Ok(true);
        }
    }
    Ok(false)
}

/// Types text into the currently focused reply composer and sends it.
pub async fn send_reply(api: &TaskContext, reply_text: &str) -> Result<bool> {
    let textarea_js = r#"
        (function() {
            var ta = document.querySelector('div[role="textbox"]') ||
                     document.querySelector('textarea') ||
                     document.querySelector('[contenteditable="true"]');
            if (ta) {
                ta.focus();
                return { found: true };
            }
            return { found: false };
        })()
    "#;
    let textarea_result = api.page().evaluate(textarea_js).await?;
    let found = textarea_result
        .value()
        .and_then(|v| {
            v.as_object()
                .and_then(|o| o.get("found"))
                .and_then(|fv| fv.as_bool())
        })
        .unwrap_or(false);

    if !found {
        return Ok(false);
    }

    human_pause(api, 300).await;
    api.type_text(reply_text).await?;
    human_pause(api, 400).await;
    api.press("Enter").await?;
    human_pause(api, 1000).await;

    Ok(true)
}

/// Full reply flow: open composer, type text, send.
pub async fn reply_to_tweet(api: &TaskContext, reply_text: &str) -> Result<bool> {
    if !click_reply_button(api).await? {
        return Ok(false);
    }
    send_reply(api, reply_text).await
}

/// Clicks the "follow" button with proper filtering + Coords + api.click_at.
pub async fn follow_from_tweet(api: &TaskContext) -> Result<bool> {
    let js = r#"
        (function() {
            var buttons = document.querySelectorAll('[role="button"]');
            for (var i = 0; i < buttons.length; i++) {
                var btn = buttons[i];
                var ariaLabel = btn.getAttribute('aria-label') || '';
                if (!ariaLabel.toLowerCase().startsWith('follow @')) continue;
                
                var spans = btn.querySelectorAll('span');
                for (var j = 0; j < spans.length; j++) {
                    var txt = (spans[j].textContent || spans[j].innerText || '').toLowerCase().trim();
                    if (txt === 'follow') {
                        var rect = btn.getBoundingClientRect();
                        if (rect.width > 0 && rect.height > 0) {
                            return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
                        }
                    }
                }
            }
            return null;
        })()
    "#;
    let result = api.page().evaluate(js).await?;

    if let Some(obj) = result.value().and_then(|v| v.as_object()) {
        if let (Some(x), Some(y)) = (
            obj.get("x").and_then(|v| v.as_f64()),
            obj.get("y").and_then(|v| v.as_f64()),
        ) {
            api.move_mouse_to(x, y).await?;
            human_pause(api, 250).await;
            api.click_at(x, y).await?;
            human_pause(api, 800).await;
            return Ok(true);
        }
    }
    Ok(false)
}

/// Clicks the "bookmark" button on the current tweet.
/// V1 stub - disabled by default.
pub async fn bookmark_tweet(_api: &TaskContext) -> Result<bool> {
    log::warn!("bookmark_tweet() called but disabled in V1");
    Ok(false)
}
