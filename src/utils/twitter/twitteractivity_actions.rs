//! Action execution helpers extracted from `twitteractivity_engagement.rs`.
//! Contains per-action DOM interaction functions and text generation.

use crate::prelude::TaskContext;
use crate::utils::mouse::hover_before_click;
use crate::utils::twitter::{
    sentiment::Sentiment,
    twitteractivity_humanized::{click_post_pause, click_prep_pause},
    twitteractivity_selectors,
    twitteractivity_state::SentimentTemplates,
    EngagementOutcome,
};
use anyhow::Result;
use serde_json::Value;

/// Helper: extract tweet text from tweet object
#[must_use]
pub fn extract_tweet_text(tweet_obj: &Value) -> String {
    if let Some(text) = tweet_obj.get("text").or_else(|| tweet_obj.get("full_text")) {
        if let Some(text_str) = text.as_str() {
            return text_str.to_string();
        }
    }
    String::new()
}

/// Helper: extract a per-tweet button center from candidate payload.
#[must_use]
pub fn extract_tweet_button_position(tweet: &Value, button: &str) -> Option<(f64, f64)> {
    let button_obj = tweet
        .get("buttons")
        .and_then(|v| v.as_object())
        .and_then(|buttons| buttons.get(button))
        .and_then(|v| v.as_object())?;

    let x = button_obj.get("x").and_then(serde_json::Value::as_f64)?;
    let y = button_obj.get("y").and_then(serde_json::Value::as_f64)?;
    Some((x, y))
}

/// Helper: click like at a specific coordinate with profile-aware timing and hover
pub async fn like_at_position(api: &TaskContext, x: f64, y: f64) -> Result<EngagementOutcome> {
    let page = api.page();
    let element_type = "button";
    hover_before_click(page, x, y, element_type).await?;
    click_prep_pause(api).await;
    api.click_at(x, y).await?;
    click_post_pause(api).await;

    // Verify like was registered by checking if button state changed
    let verify_js = twitteractivity_selectors::js_verify_like(x, y);

    let result = page.evaluate(verify_js).await?;

    let value = result.value();
    if let Some(v) = value {
        if let Some(liked) = v.as_bool() {
            return if liked {
                Ok(EngagementOutcome::Completed)
            } else {
                Ok(EngagementOutcome::AlreadyDone)
            };
        }
    }

    // Verification failed - assume like was not registered
    Ok(EngagementOutcome::Failed)
}

/// Select a template string from a sentiment-indexed set.
/// Returns the template at `(idx % len)` position, or empty string if no templates.
#[must_use]
fn select_template(
    sentiment: Sentiment,
    idx: u32,
    positive: &[String],
    neutral: &[String],
    negative: &[String],
) -> String {
    let phrases = match sentiment {
        Sentiment::Positive => positive,
        Sentiment::Neutral => neutral,
        Sentiment::Negative => negative,
    };
    if phrases.is_empty() {
        return String::new();
    }
    phrases[(idx as usize) % phrases.len()].clone()
}

/// Generate a short reply string based on sentiment.
#[must_use]
pub fn generate_reply_text(
    sentiment: Sentiment,
    reply_idx: u32,
    templates: &SentimentTemplates,
) -> String {
    select_template(
        sentiment,
        reply_idx,
        &templates.reply_positive,
        &templates.reply_neutral,
        &templates.reply_negative,
    )
}

/// Generate a short quote commentary string based on sentiment.
#[must_use]
pub fn generate_quote_text(
    sentiment: Sentiment,
    quote_idx: u32,
    templates: &SentimentTemplates,
) -> String {
    select_template(
        sentiment,
        quote_idx,
        &templates.quote_positive,
        &templates.quote_neutral,
        &templates.quote_negative,
    )
}
