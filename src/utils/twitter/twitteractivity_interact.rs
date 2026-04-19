//! Interaction helpers for Twitter/X automation.
//! Like, retweet, follow, and reply operations with human-like timing.

use crate::prelude::TaskContext;
use anyhow::Result;
use serde_json::Value;

use super::twitteractivity_feed::get_tweet_engagement_buttons;
use super::{twitteractivity_selectors::*, twitteractivity_humanized::*};

/// Clicks the "like" (heart) button on the current tweet (assumes tweet is in view).
/// Returns true if the like action appears to have been successful (state toggled).
pub async fn like_tweet(api: &TaskContext) -> Result<bool> {
    let buttons = get_tweet_engagement_buttons(api).await?;
    if let Some(like_obj) = buttons.get("like").and_then(|v: &Value| v.as_object()) {
        if let (Some(x), Some(y)) = (
            like_obj.get("x").and_then(|v: &serde_json::Value| v.as_f64()),
            like_obj.get("y").and_then(|v: &serde_json::Value| v.as_f64()),
        ) {
            api.move_mouse_to(x, y).await?;
            human_pause(api, 300).await;
            api.click_at(x, y).await?;
            human_pause(api, 600).await; // wait for animation
            return Ok(true);
        }
    }
    Ok(false)
}

/// Clicks the "retweet" button on the current tweet.
/// Note: Does not confirm the retweet in the modal; just opens the retweet menu.
/// Returns true if retweet button was found and clicked.
pub async fn click_retweet_button(api: &TaskContext) -> Result<bool> {
    let buttons = get_tweet_engagement_buttons(api).await?;
    if let Some(rt_obj) = buttons.get("retweet").and_then(|v: &Value| v.as_object()) {
        if let (Some(x), Some(y)) = (
            rt_obj.get("x").and_then(|v: &serde_json::Value| v.as_f64()),
            rt_obj.get("y").and_then(|v: &serde_json::Value| v.as_f64()),
        ) {
            api.move_mouse_to(x, y).await?;
            human_pause(api, 300).await;
            api.click_at(x, y).await?;
            human_pause(api, 600).await;
            return Ok(true);
        }
    }
    Ok(false)
}

/// Confirms a retweet from the retweet modal that appears after clicking retweet.
/// Returns true if confirmation succeeded.
pub async fn confirm_retweet(api: &TaskContext) -> Result<bool> {
    // Modal typically has a confirm button with data-testid="retweetConfirm"
    let js = r#"
        (function() {
            var btn = document.querySelector('button[data-testid="retweetConfirm"]');
            if (!btn) return null;
            var rect = btn.getBoundingClientRect();
            return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
        })()
    "#;
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value();
    if let Some(obj) = value.and_then(|v: &Value| v.as_object()) {
        if let (Some(x), Some(y)) = (
            obj.get("x").and_then(|v: &serde_json::Value| v.as_f64()),
            obj.get("y").and_then(|v: &serde_json::Value| v.as_f64()),
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
/// Returns true if retweet was confirmed.
pub async fn retweet_tweet(api: &TaskContext) -> Result<bool> {
    if click_retweet_button(api).await? {
        human_pause(api, 500).await;
        return confirm_retweet(api).await;
    }
    Ok(false)
}

/// Clicks the "reply" button on the current tweet.
/// This opens the reply composer.
pub async fn click_reply_button(api: &TaskContext) -> Result<bool> {
    let buttons = get_tweet_engagement_buttons(api).await?;
    if let Some(reply_obj) = buttons.get("reply").and_then(|v: &Value| v.as_object()) {
        if let (Some(x), Some(y)) = (
            reply_obj.get("x").and_then(|v: &serde_json::Value| v.as_f64()),
            reply_obj.get("y").and_then(|v: &serde_json::Value| v.as_f64()),
        ) {
            api.move_mouse_to(x, y).await?;
            human_pause(api, 300).await;
            api.click_at(x, y).await?;
            human_pause(api, 500).await; // composer opens
            return Ok(true);
        }
    }
     Ok(false)
 }
 
 /// Types text into the currently focused reply composer and sends it.
/// Note: Assumes `click_reply_button` was called first and composer is open.
pub async fn send_reply(api: &TaskContext, reply_text: &str) -> Result<bool> {
    // Try to focus the reply textarea
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
    let textarea_result = api.page().evaluate(textarea_js.to_string()).await?;
    let found = textarea_result
        .value()
        .and_then(|v: &Value| v.as_object().and_then(|o| o.get("found")).and_then(|fv: &Value| fv.as_bool()))
        .unwrap_or(false);

    if !found {
        return Ok(false);
    }

    human_pause(api, 300).await;

    // Type the reply text
    api.type_text(reply_text).await?;
    human_pause(api, 400).await;

    // Send (usually Enter or a "Reply" button)
    // Try pressing Enter first
    api.press("Enter").await?;
    human_pause(api, 1000).await;

    // Check if reply was sent (optional)
    Ok(true)
}

/// Full reply flow: open composer, type text, send.
/// Returns true if reply succeeded.
pub async fn reply_to_tweet(api: &TaskContext, reply_text: &str) -> Result<bool> {
    if !click_reply_button(api).await? {
        return Ok(false);
    }
    send_reply(api, reply_text).await
}

/// Clicks the "follow" button for the currently viewed user/tweet.
/// Returns true if follow button was clicked.
pub async fn follow_from_tweet(api: &TaskContext) -> Result<bool> {
    let js = selector_follow_button();
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value();

    if let Some(obj) = value.and_then(|v: &Value| v.as_object()) {
        if let (Some(x), Some(y)) = (
            obj.get("x").and_then(|v: &serde_json::Value| v.as_f64()),
            obj.get("y").and_then(|v: &serde_json::Value| v.as_f64()),
        ) {
            api.move_mouse_to(x, y).await?;
            human_pause(api, 300).await;
            api.click_at(x, y).await?;
            human_pause(api, 800).await; // wait for follow to register
            return Ok(true);
        }
    }

    Ok(false)
}


