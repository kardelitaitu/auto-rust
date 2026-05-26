//! Shared URL extraction utilities for JSON task payloads.

use anyhow::{anyhow, Result};
use serde_json::Value;

/// Extract a URL from a JSON task payload, checking standard fields in priority order.
///
/// # Priority
/// 1. `url` field
/// 2. `value` field
/// 3. `default_url` field
/// 4. Any other field containing an "x.com" or "twitter.com" URL
///
/// # Returns
/// - `Ok(url)` if a URL is found
/// - `Err` if no URL is found in any field
pub fn extract_url_from_payload(payload: &Value) -> Result<String> {
    // Standard fields in priority order
    for key in &["url", "value", "default_url"] {
        if let Some(value) = payload.get(*key) {
            if let Some(url_str) = value.as_str() {
                if !url_str.is_empty() {
                    return Ok(url_str.to_string());
                }
            }
        }
    }

    // Fallback: search all remaining fields for x.com or twitter.com URLs
    if let Some(obj) = payload.as_object() {
        for (key, val) in obj {
            if key != "url" && key != "value" && key != "default_url" {
                if let Some(v) = val.as_str() {
                    if !v.is_empty() && (v.contains("x.com") || v.contains("twitter.com")) {
                        return Ok(v.to_string());
                    }
                }
            }
        }
    }

    Err(anyhow!("No URL found in payload"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_url_field() {
        let payload = json!({"url": "https://x.com/user/status/123"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert!(result.contains("x.com"));
    }

    #[test]
    fn extract_value_field() {
        let payload = json!({"value": "https://x.com/user/status/456"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert!(result.contains("x.com"));
    }

    #[test]
    fn extract_default_url_field() {
        let payload = json!({"default_url": "https://x.com/user/status/789"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert!(result.contains("x.com"));
    }

    #[test]
    fn extract_fallback_x_com() {
        let payload = json!({"tweet": "https://x.com/user/status/abc"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert!(result.contains("x.com"));
    }

    #[test]
    fn extract_fallback_twitter_com() {
        let payload = json!({"tweet": "https://twitter.com/user/status/abc"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert!(result.contains("twitter.com"));
    }

    #[test]
    fn extract_url_priority_over_value() {
        let payload = json!({
            "url": "https://x.com/from_url",
            "value": "https://x.com/from_value"
        });
        let result = extract_url_from_payload(&payload).unwrap();
        assert!(result.contains("from_url"));
    }

    #[test]
    fn extract_url_missing() {
        let payload = json!({});
        assert!(extract_url_from_payload(&payload).is_err());
    }

    #[test]
    fn extract_url_not_an_object() {
        let payload = json!("not an object");
        assert!(extract_url_from_payload(&payload).is_err());
    }

    #[test]
    fn extract_url_empty_string_fields() {
        let payload = json!({"url": "", "value": "", "default_url": ""});
        assert!(extract_url_from_payload(&payload).is_err());
    }
}
