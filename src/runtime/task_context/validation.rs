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
