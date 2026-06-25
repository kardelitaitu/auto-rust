//! Action execution helpers extracted from `twitteractivity_engagement.rs`.
//! Contains per-action DOM interaction functions and text generation.

use crate::internal::text::truncate_chars;
use crate::prelude::TaskContext;
use crate::utils::mouse::hover_before_click;
use crate::utils::twitter::{
    sentiment::Sentiment,
    twitteractivity_humanized::{click_post_pause, click_prep_pause, human_pause},
    twitteractivity_selectors,
    twitteractivity_state::SentimentTemplates,
    EngagementOutcome,
};
use anyhow::Result;
use rand::Rng;
use serde_json::Value;

/// Helper: extract tweet text from tweet object
///
/// Checks `text` → `full_text` → `retweeted_status.text` → truncated JSON fallback (280 chars).
#[must_use]
pub fn extract_tweet_text(tweet_obj: &Value) -> String {
    if let Some(text) = tweet_obj.get("text").and_then(|v| v.as_str()) {
        return text.to_string();
    }
    if let Some(full) = tweet_obj.get("full_text").and_then(|v| v.as_str()) {
        return full.to_string();
    }
    if let Some(obj) = tweet_obj.as_object() {
        if let Some(rt) = obj.get("retweeted_status") {
            return extract_tweet_text(rt);
        }
    }
    truncate_chars(&tweet_obj.to_string(), 280)
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

/// Click retweet at a specific coordinate, then confirm in the popup.
///
/// This is the position-based equivalent of `retweet_tweet()` — clicks the retweet
/// button at the scraped coordinates, waits for the confirm dialog, then clicks confirm.
/// Works from the feed without needing a thread dive.
pub async fn retweet_at_position(api: &TaskContext, x: f64, y: f64) -> Result<EngagementOutcome> {
    let page = api.page();
    let element_type = "button";

    // Step 1: Click retweet button at position
    hover_before_click(page, x, y, element_type).await?;
    click_prep_pause(api).await;
    api.click_at(x, y).await?;
    click_post_pause(api).await;

    // Step 2: Random pause 1-2s before confirming (matches retweet_tweet behavior)
    let pause_ms = rand::thread_rng().gen_range(1000..2000);
    human_pause(api, pause_ms).await;

    // Step 3: Find and click the retweet confirm button
    let confirm_js = twitteractivity_selectors::js_find_retweet_confirm_button();
    let result = page.evaluate(confirm_js).await?;

    if let Some((cx, cy)) = result.value().and_then(parse_button_coords) {
        api.move_mouse_to(cx, cy).await?;
        human_pause(api, 250).await;
        api.click_at(cx, cy).await?;
        human_pause(api, 800).await;
        return Ok(EngagementOutcome::Completed);
    }

    Ok(EngagementOutcome::Failed)
}

/// Click follow at a specific coordinate.
///
/// Position-based equivalent of `follow_from_tweet()`. Clicks the follow button
/// at the scraped coordinates. Works from the feed without a thread dive.
pub async fn follow_at_position(api: &TaskContext, x: f64, y: f64) -> Result<EngagementOutcome> {
    let page = api.page();
    let element_type = "button";

    hover_before_click(page, x, y, element_type).await?;
    click_prep_pause(api).await;
    api.click_at(x, y).await?;
    click_post_pause(api).await;

    Ok(EngagementOutcome::Completed)
}

/// Click bookmark at a specific coordinate.
///
/// Position-based equivalent of `bookmark_tweet()`. Clicks the bookmark button
/// at the scraped coordinates. Works from the feed without a thread dive.
pub async fn bookmark_at_position(api: &TaskContext, x: f64, y: f64) -> Result<EngagementOutcome> {
    let page = api.page();
    let element_type = "button";

    hover_before_click(page, x, y, element_type).await?;
    click_prep_pause(api).await;
    api.click_at(x, y).await?;
    click_post_pause(api).await;

    Ok(EngagementOutcome::Completed)
}

/// Helper to parse `{x, y}` coordinates from a JS evaluation result.
fn parse_button_coords(value: &serde_json::Value) -> Option<(f64, f64)> {
    let obj = value.as_object()?;
    let x = obj.get("x").and_then(|v| v.as_f64())?;
    let y = obj.get("y").and_then(|v| v.as_f64())?;
    Some((x, y))
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ====================================================================
    // extract_tweet_text
    // ====================================================================

    #[test]
    fn extract_text_from_text_field() {
        let tweet = json!({"text": "Hello world"});
        assert_eq!(extract_tweet_text(&tweet), "Hello world");
    }

    #[test]
    fn extract_text_from_full_text_field() {
        let tweet = json!({"full_text": "A longer tweet with full text"});
        assert_eq!(extract_tweet_text(&tweet), "A longer tweet with full text");
    }

    #[test]
    fn extract_text_text_field_takes_priority_over_full_text() {
        // When both are present, text field is checked first
        let tweet = json!({
            "text": "short text",
            "full_text": "the full and complete text"
        });
        assert_eq!(extract_tweet_text(&tweet), "short text");
    }

    #[test]
    fn extract_text_recurses_into_retweeted_status() {
        let tweet = json!({
            "retweeted_status": {
                "text": "original tweet content"
            }
        });
        assert_eq!(extract_tweet_text(&tweet), "original tweet content");
    }

    #[test]
    fn extract_text_retweeted_status_full_text() {
        let tweet = json!({
            "retweeted_status": {
                "full_text": "original long form content here"
            }
        });
        assert_eq!(
            extract_tweet_text(&tweet),
            "original long form content here"
        );
    }

    #[test]
    fn extract_text_empty_object_uses_truncation_fallback() {
        let tweet = json!({});
        let result = extract_tweet_text(&tweet);
        // Should not panic and should return something
        assert!(
            !result.is_empty(),
            "Fallback should produce non-empty result"
        );
        // The fallback is truncation of the JSON representation
        assert_eq!(result, "{}");
    }

    #[test]
    fn extract_text_non_object_value_uses_truncation() {
        let tweet = json!("just a string");
        let result = extract_tweet_text(&tweet);
        assert!(!result.is_empty());
        assert!(result.len() <= 280, "Result should be truncated to 280");
    }

    #[test]
    fn extract_text_deeply_nested_retweet() {
        let tweet = json!({
            "retweeted_status": {
                "retweeted_status": {
                    "text": "deeply nested original"
                }
            }
        });
        assert_eq!(extract_tweet_text(&tweet), "deeply nested original");
    }

    #[test]
    fn extract_text_retweeted_status_without_text_falls_to_truncation() {
        let tweet = json!({
            "retweeted_status": {
                "id": 12345
            }
        });
        let result = extract_tweet_text(&tweet);
        // retweeted_status has no text/full_text, should truncate the outer object
        assert!(!result.is_empty());
        assert!(result.len() <= 280);
    }

    #[test]
    fn extract_text_null_text_field_skips_to_fallback() {
        let tweet = json!({"text": null});
        let result = extract_tweet_text(&tweet);
        // text is null → as_str() returns None → falls through
        assert!(!result.is_empty());
    }

    #[test]
    fn extract_text_numeric_text_field_skips_to_fallback() {
        let tweet = json!({"text": 123});
        let result = extract_tweet_text(&tweet);
        // text is number → as_str() returns None → falls through
        assert!(!result.is_empty());
    }

    #[test]
    fn extract_text_long_text_is_not_truncated() {
        let long_text = "a".repeat(1000);
        let tweet = json!({"text": long_text});
        let result = extract_tweet_text(&tweet);
        // text field returns the value as-is (truncation only applies to fallback)
        assert_eq!(result.len(), 1000);
    }

    #[test]
    fn extract_text_empty_string_text() {
        let tweet = json!({"text": ""});
        let result = extract_tweet_text(&tweet);
        assert_eq!(result, "");
    }

    #[test]
    fn extract_text_empty_string_full_text() {
        let tweet = json!({"full_text": ""});
        let result = extract_tweet_text(&tweet);
        assert_eq!(result, "");
    }

    #[test]
    fn extract_text_retweeted_status_non_object_skips_to_fallback() {
        // retweeted_status must be an object for as_object() to return Some
        let tweet = json!({"retweeted_status": "not an object"});
        let result = extract_tweet_text(&tweet);
        // Falls through to truncation of the outer object
        assert!(!result.is_empty());
        assert!(result.len() <= 280);
    }

    #[test]
    fn extract_text_retweeted_status_null_skips_to_fallback() {
        let tweet = json!({"retweeted_status": null});
        let result = extract_tweet_text(&tweet);
        assert!(!result.is_empty());
        assert!(result.len() <= 280);
    }

    #[test]
    fn extract_text_text_array_skips_to_fallback() {
        let tweet = json!({"text": [1, 2, 3]});
        let result = extract_tweet_text(&tweet);
        // as_str() returns None for arrays, falls through
        assert!(!result.is_empty());
    }

    #[test]
    fn extract_text_outer_text_wins_over_retweeted_status() {
        // When outer object has text, retweeted_status should not be consulted
        let tweet = json!({
            "text": "outer text",
            "retweeted_status": {"text": "inner text"}
        });
        assert_eq!(extract_tweet_text(&tweet), "outer text");
    }

    #[test]
    fn extract_text_large_fallback_truncates_to_280() {
        let large_json = serde_json::json!({
            "data": "x".repeat(500),
            "nested": {"deep": "y".repeat(500)}
        });
        let result = extract_tweet_text(&large_json);
        // Fallback path: truncate_chars at 280
        assert!(result.len() <= 280);
    }

    // ====================================================================
    // extract_tweet_button_position
    // ====================================================================

    #[test]
    fn extract_button_position_valid() {
        let tweet = json!({
            "buttons": {
                "like": {"x": 100.5, "y": 200.3}
            }
        });
        let pos = extract_tweet_button_position(&tweet, "like");
        assert_eq!(pos, Some((100.5, 200.3)));
    }

    #[test]
    fn extract_button_position_missing_button() {
        let tweet = json!({
            "buttons": {
                "like": {"x": 100.5, "y": 200.3}
            }
        });
        let pos = extract_tweet_button_position(&tweet, "retweet");
        assert_eq!(pos, None);
    }

    #[test]
    fn extract_button_position_missing_buttons() {
        let tweet = json!({});
        let pos = extract_tweet_button_position(&tweet, "like");
        assert_eq!(pos, None);
    }

    #[test]
    fn extract_button_position_non_object_buttons() {
        let tweet = json!({"buttons": "not an object"});
        let pos = extract_tweet_button_position(&tweet, "like");
        assert_eq!(pos, None);
    }

    #[test]
    fn extract_button_position_missing_coordinate() {
        let tweet = json!({
            "buttons": {
                "like": {"x": 100.5}
            }
        });
        let pos = extract_tweet_button_position(&tweet, "like");
        assert_eq!(pos, None);
    }

    #[test]
    fn extract_button_position_null_coordinates() {
        let tweet = json!({
            "buttons": {
                "like": {"x": null, "y": 200.3}
            }
        });
        let pos = extract_tweet_button_position(&tweet, "like");
        assert_eq!(pos, None);
    }

    // ====================================================================
    // select_template / generate_reply_text / generate_quote_text
    // ====================================================================

    #[test]
    fn select_template_positive_uses_positive_list() {
        let templates = SentimentTemplates::default();
        let result = generate_reply_text(Sentiment::Positive, 0, &templates);
        assert_eq!(result, templates.reply_positive[0]);
    }

    #[test]
    fn select_template_neutral_uses_neutral_list() {
        let templates = SentimentTemplates::default();
        let result = generate_reply_text(Sentiment::Neutral, 0, &templates);
        assert_eq!(result, templates.reply_neutral[0]);
    }

    #[test]
    fn select_template_negative_uses_negative_list() {
        let templates = SentimentTemplates::default();
        let result = generate_reply_text(Sentiment::Negative, 0, &templates);
        assert_eq!(result, templates.reply_negative[0]);
    }

    #[test]
    fn select_template_wraps_around_with_modulo() {
        let templates = SentimentTemplates::default();
        // idx = len should wrap to index 0
        let len = templates.reply_positive.len() as u32;
        let result = generate_reply_text(Sentiment::Positive, len, &templates);
        assert_eq!(result, templates.reply_positive[0]);
    }

    #[test]
    fn select_template_empty_list_returns_empty() {
        let templates = SentimentTemplates {
            reply_positive: vec![],
            reply_neutral: vec![],
            reply_negative: vec![],
            ..SentimentTemplates::default()
        };
        let result = generate_reply_text(Sentiment::Positive, 0, &templates);
        assert_eq!(result, "");
    }

    #[test]
    fn generate_quote_text_uses_quote_templates() {
        let templates = SentimentTemplates::default();
        let result = generate_quote_text(Sentiment::Positive, 0, &templates);
        assert_eq!(result, templates.quote_positive[0]);
    }

    #[test]
    fn generate_quote_text_neutral_uses_neutral_list() {
        let templates = SentimentTemplates::default();
        let result = generate_quote_text(Sentiment::Neutral, 0, &templates);
        assert_eq!(result, templates.quote_neutral[0]);
    }

    #[test]
    fn generate_quote_text_negative_uses_negative_list() {
        let templates = SentimentTemplates::default();
        let result = generate_quote_text(Sentiment::Negative, 0, &templates);
        assert_eq!(result, templates.quote_negative[0]);
    }

    #[test]
    fn generate_quote_text_wraps_around_with_modulo() {
        let templates = SentimentTemplates::default();
        let len = templates.quote_positive.len() as u32;
        let result = generate_quote_text(Sentiment::Positive, len, &templates);
        assert_eq!(result, templates.quote_positive[0]);
    }

    #[test]
    fn generate_quote_text_empty_list_returns_empty() {
        let templates = SentimentTemplates {
            quote_positive: vec![],
            quote_neutral: vec![],
            quote_negative: vec![],
            ..SentimentTemplates::default()
        };
        let result = generate_quote_text(Sentiment::Positive, 0, &templates);
        assert_eq!(result, "");
    }
}

#[cfg(test)]
mod fuzz_tests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::Value;

    /// Convert an arbitrary string to a Value: parses as JSON if valid, falls back to Value::String.
    fn val(s: &str) -> Value {
        serde_json::from_str(s).unwrap_or(Value::String(s.to_string()))
    }

    proptest! {
        /// extract_tweet_text must never panic on any string -> Value conversion.
        #[test]
        fn fuzz_extract_tweet_text(s: String) {
            let value = val(&s);
            let _ = extract_tweet_text(&value);
        }

        /// extract_tweet_text with object containing arbitrary text/full_text values.
        #[test]
        fn fuzz_extract_tweet_text_fields(text: String, full_text: String) {
            let obj = serde_json::json!({"text": val(&text), "full_text": val(&full_text)});
            let _ = extract_tweet_text(&obj);
        }

        /// extract_tweet_button_position must never panic on any string + button name.
        #[test]
        fn fuzz_extract_tweet_button_position(json: String, button: String) {
            let value = val(&json);
            let _ = extract_tweet_button_position(&value, &button);
        }

        /// like verification pattern: value.as_bool().unwrap_or(false) — never panic.
        #[test]
        fn fuzz_like_verify_value(s: String) {
            let value = val(&s);
            let _ = value.as_bool().unwrap_or(false);
        }

        /// nested value chain pattern: get → as_object → get → as_str/as_f64.
        #[test]
        fn fuzz_nested_value_chain(json: String, key1: String, key2: String) {
            let value = val(&json);
            let _ = value.get(&key1)
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get(&key2))
                .and_then(|v| v.as_str());
            let _ = value.get(&key1)
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get(&key2))
                .and_then(|v| v.as_f64());
        }
    }
}
