//! Session data validation functions.

use crate::task::policy::SessionData;

/// Validate session data and return warnings for issues found.
/// Checks:
/// - Empty cookies and localStorage
/// - Cookie entries missing 'name' or 'value' fields
/// - localStorage size exceeding 1000 items
/// - Empty URL
#[must_use]
pub fn validate_session_data_impl(data: &SessionData) -> Vec<String> {
    let mut warnings = Vec::new();

    if data.cookies.is_empty() && data.local_storage.is_empty() {
        warnings.push("SessionData has no cookies and no localStorage".to_string());
    }

    for (i, cookie) in data.cookies.iter().enumerate() {
        if let Some(obj) = cookie.as_object() {
            if !obj.contains_key("name") {
                warnings.push(format!("Cookie[{i}] missing 'name' field"));
            }
            if !obj.contains_key("value") {
                warnings.push(format!("Cookie[{i}] missing 'value' field"));
            }
        } else {
            warnings.push(format!("Cookie[{i}] is not a JSON object"));
        }
    }

    if data.local_storage.len() > 1000 {
        warnings.push(format!(
            "localStorage has {} items (very large)",
            data.local_storage.len()
        ));
    }

    if data.url.is_empty() {
        warnings.push("SessionData url is empty".to_string());
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::policy::SessionData;
    use chrono::Utc;
    use serde_json::json;

    fn make_session_data(
        cookies: Vec<serde_json::Value>,
        local_storage: std::collections::HashMap<String, String>,
        url: &str,
    ) -> SessionData {
        SessionData {
            cookies,
            local_storage,
            exported_at: Utc::now(),
            url: url.to_string(),
        }
    }

    #[test]
    fn empty_cookies_and_storage_warns() {
        let data = make_session_data(vec![], std::collections::HashMap::new(), "https://x.com");
        let warnings = validate_session_data_impl(&data);
        assert!(warnings
            .iter()
            .any(|w| w.contains("no cookies and no localStorage")));
    }

    #[test]
    fn valid_cookie_no_warnings() {
        let cookie = json!({"name": "sid", "value": "abc123"});
        let data = make_session_data(
            vec![cookie],
            std::collections::HashMap::new(),
            "https://x.com",
        );
        let warnings = validate_session_data_impl(&data);
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    }

    #[test]
    fn cookie_missing_name_warns() {
        let cookie = json!({"value": "abc"});
        let data = make_session_data(
            vec![cookie],
            std::collections::HashMap::new(),
            "https://x.com",
        );
        let warnings = validate_session_data_impl(&data);
        assert!(warnings.iter().any(|w| w.contains("missing 'name'")));
    }

    #[test]
    fn cookie_missing_value_warns() {
        let cookie = json!({"name": "sid"});
        let data = make_session_data(
            vec![cookie],
            std::collections::HashMap::new(),
            "https://x.com",
        );
        let warnings = validate_session_data_impl(&data);
        assert!(warnings.iter().any(|w| w.contains("missing 'value'")));
    }

    #[test]
    fn cookie_not_object_warns() {
        let cookie = json!("just a string");
        let data = make_session_data(
            vec![cookie],
            std::collections::HashMap::new(),
            "https://x.com",
        );
        let warnings = validate_session_data_impl(&data);
        assert!(warnings.iter().any(|w| w.contains("not a JSON object")));
    }

    #[test]
    fn empty_url_warns() {
        let data = make_session_data(vec![], std::collections::HashMap::new(), "");
        let warnings = validate_session_data_impl(&data);
        assert!(warnings.iter().any(|w| w.contains("url is empty")));
    }

    #[test]
    fn large_local_storage_warns() {
        let mut ls = std::collections::HashMap::new();
        for i in 0..1500 {
            ls.insert(format!("key_{i}"), format!("val_{i}"));
        }
        let data = make_session_data(vec![], ls, "https://x.com");
        let warnings = validate_session_data_impl(&data);
        assert!(warnings.iter().any(|w| w.contains("very large")));
    }

    #[test]
    fn valid_data_no_warnings() {
        let cookie = json!({"name": "sid", "value": "abc"});
        let mut ls = std::collections::HashMap::new();
        ls.insert("theme".to_string(), "dark".to_string());
        let data = make_session_data(vec![cookie], ls, "https://x.com");
        let warnings = validate_session_data_impl(&data);
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    }

    #[test]
    fn multiple_cookies_all_checked() {
        let c1 = json!({"name": "a", "value": "1"});
        let c2 = json!({"name": "b"}); // missing value
        let c3 = json!({"value": "3"}); // missing name
        let data = make_session_data(
            vec![c1, c2, c3],
            std::collections::HashMap::new(),
            "https://x.com",
        );
        let warnings = validate_session_data_impl(&data);
        // Should have warnings for cookie[1] missing value and cookie[2] missing name
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn local_storage_with_cookies_no_empty_warning() {
        let cookie = json!({"name": "x", "value": "y"});
        let mut ls = std::collections::HashMap::new();
        ls.insert("k".to_string(), "v".to_string());
        let data = make_session_data(vec![cookie], ls, "https://x.com");
        let warnings = validate_session_data_impl(&data);
        // Should NOT warn about empty cookies+storage since both are present
        assert!(!warnings
            .iter()
            .any(|w| w.contains("no cookies and no localStorage")));
    }
}
