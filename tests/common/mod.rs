//! Shared test utilities for the auto-rust project.
//!
//! This module provides common test helpers, mock types, and assertions
//! to reduce duplication across test files.
//!
//! # Usage
//!
//! ```rust,no_run
//! use auto::tests::test_utils::*;
//!
//! let mock = MockPageContext::new();
//! let temp = TempTestDir::new();
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Temporary Directory Helpers
// ============================================================================

/// Temporary test directory that auto-cleans on drop.
///
/// # Example
/// ```
/// let temp = TempTestDir::new();
/// let file_path = temp.create_file("test.txt", "content");
/// assert!(file_path.exists());
/// ```
pub struct TempTestDir {
    dir: TempDir,
}

impl TempTestDir {
    /// Create a new temporary test directory.
    pub fn new() -> Self {
        Self {
            dir: TempDir::new().expect("Failed to create temp dir"),
        }
    }

    /// Get the path to the temp directory.
    pub fn path(&self) -> &std::path::Path {
        self.dir.path()
    }

    /// Create a file in the temp directory with given content.
    /// Returns the full path to the created file.
    pub fn create_file(&self, name: &str, content: &str) -> PathBuf {
        let path = self.path().join(name);
        std::fs::write(&path, content).expect("Failed to write test file");
        path
    }

    /// Create a JSON file in the temp directory.
    pub fn create_json_file(&self, name: &str, value: serde_json::Value) -> PathBuf {
        let path = self.path().join(name);
        let content = serde_json::to_string_pretty(&value).expect("Failed to serialize JSON");
        std::fs::write(&path, content).expect("Failed to write JSON file");
        path
    }

    /// Create a subdirectory.
    pub fn create_subdir(&self, name: &str) -> PathBuf {
        let path = self.path().join(name);
        std::fs::create_dir_all(&path).expect("Failed to create subdir");
        path
    }
}

impl Default for TempTestDir {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Mock Types
// ============================================================================

/// Mock HTTP response for testing network operations.
#[derive(Debug, Clone)]
pub struct MockHttpResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}

impl MockHttpResponse {
    /// Create a successful JSON response.
    pub fn json(status: u16, value: serde_json::Value) -> Self {
        Self {
            status,
            body: value.to_string(),
            headers: HashMap::from([
                ("content-type".to_string(), "application/json".to_string()),
            ]),
        }
    }

    /// Create a successful text response.
    pub fn text(status: u16, body: &str) -> Self {
        Self {
            status,
            body: body.to_string(),
            headers: HashMap::new(),
        }
    }

    /// Create a 404 Not Found response.
    pub fn not_found() -> Self {
        Self {
            status: 404,
            body: "Not Found".to_string(),
            headers: HashMap::new(),
        }
    }

    /// Create a 500 Internal Server Error response.
    pub fn server_error() -> Self {
        Self {
            status: 500,
            body: "Internal Server Error".to_string(),
            headers: HashMap::new(),
        }
    }
}

/// Mock page context for testing browser interactions without a real browser.
#[derive(Debug, Default, Clone)]
pub struct MockPageContext {
    /// Simulated localStorage data (origin -> key/value)
    pub local_storage: HashMap<String, HashMap<String, String>>,
    /// Simulated sessionStorage data
    pub session_storage: HashMap<String, HashMap<String, String>>,
    /// Simulated cookies
    pub cookies: Vec<serde_json::Value>,
    /// Last JavaScript executed
    pub last_js: Option<String>,
}

impl MockPageContext {
    /// Create a new empty mock context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add localStorage data.
    pub fn with_local_storage(mut self, origin: &str, data: HashMap<String, String>) -> Self {
        self.local_storage.insert(origin.to_string(), data);
        self
    }

    /// Add sessionStorage data.
    pub fn with_session_storage(mut self, origin: &str, data: HashMap<String, String>) -> Self {
        self.session_storage.insert(origin.to_string(), data);
        self
    }

    /// Add a cookie.
    pub fn with_cookie(mut self, cookie: serde_json::Value) -> Self {
        self.cookies.push(cookie);
        self
    }

    /// Record executed JavaScript.
    pub fn record_js(&mut self, js: &str) {
        self.last_js = Some(js.to_string());
    }

    /// Get the last recorded JavaScript.
    pub fn get_last_js(&self) -> Option<&String> {
        self.last_js.as_ref()
    }

    /// Simulate exporting localStorage as JSON string.
    pub fn export_local_storage_json(&self, origin: &str) -> Result<String, String> {
        let data = self.local_storage.get(origin).cloned().unwrap_or_default();
        let mut result = HashMap::new();
        result.insert(origin.to_string(), data);
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }

    /// Simulate exporting sessionStorage as JSON string.
    pub fn export_session_storage_json(&self, origin: &str) -> Result<String, String> {
        let data = self.session_storage.get(origin).cloned().unwrap_or_default();
        let mut result = HashMap::new();
        result.insert(origin.to_string(), data);
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }

    /// Count elements by selector (mock implementation).
    pub fn count_elements(&self, selector: &str) -> usize {
        match selector {
            "button" | ".button" => 3,
            "#test-box" => 1,
            "input" => 2,
            _ => 0,
        }
    }

    /// Check if element is in viewport (mock implementation).
    pub fn is_in_viewport(&self, selector: &str) -> bool {
        selector.contains("viewport") || !selector.contains("below-fold")
    }
}

// ============================================================================
// Assertion Helpers
// ============================================================================

/// Assert that a Result is Ok and return the value.
#[macro_export]
macro_rules! assert_ok {
    ($expr:expr) => {{
        match $expr {
            Ok(val) => val,
            Err(e) => panic!("Expected Ok, got Err: {:?}", e),
        }
    }};
    ($expr:expr, $msg:literal) => {{
        match $expr {
            Ok(val) => val,
            Err(e) => panic!("{}: {:?}", $msg, e),
        }
    }};
}

/// Assert that a Result is Err.
#[macro_export]
macro_rules! assert_err {
    ($expr:expr) => {{
        match $expr {
            Ok(val) => panic!("Expected Err, got Ok: {:?}", val),
            Err(_) => (),
        }
    }};
    ($expr:expr, $msg:literal) => {{
        match $expr {
            Ok(val) => panic!("{}: {:?}", $msg, val),
            Err(_) => (),
        }
    }};
}

/// Assert that a string contains a substring.
#[macro_export]
macro_rules! assert_contains {
    ($haystack:expr, $needle:expr) => {{
        let haystack = &$haystack;
        let needle = &$needle;
        assert!(
            haystack.contains(needle),
            "Expected '{}' to contain '{}'",
            haystack,
            needle
        );
    }};
}

/// Assert that two f64 values are approximately equal.
#[macro_export]
macro_rules! assert_approx_eq {
    ($left:expr, $right:expr, $tolerance:expr) => {{
        let left = $left as f64;
        let right = $right as f64;
        let tolerance = $tolerance as f64;
        assert!(
            (left - right).abs() < tolerance,
            "Expected {} ≈ {}, but difference is {}",
            left,
            right,
            (left - right).abs()
        );
    }};
}

// ============================================================================
// Test Data Builders
// ============================================================================

/// Build a mock cookie JSON value.
pub fn build_mock_cookie(name: &str, value: &str, domain: &str) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "value": value,
        "domain": domain,
        "path": "/",
        "session": false,
    })
}

/// Build a HashMap of key-value pairs.
pub fn build_string_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// Build test localStorage data.
pub fn build_local_storage_data() -> HashMap<String, String> {
    build_string_map(&[
        ("user_pref", "dark_mode"),
        ("session_id", "abc123"),
        ("theme", "auto"),
    ])
}

/// Build test sessionStorage data.
pub fn build_session_storage_data() -> HashMap<String, String> {
    build_string_map(&[
        ("temp_data", "cached_value"),
        ("form_state", r#"{"field": "value"}"#),
    ])
}

// ============================================================================
// Async Test Helpers
// ============================================================================

/// Run an async test with a timeout.
#[cfg(test)]
pub async fn run_with_timeout<F, T>(future: F, timeout_ms: u64) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), future)
        .await
        .expect("Test timed out")
}

// ============================================================================
// Tests for Test Utilities
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_test_dir_creates_and_cleans_up() {
        let temp = TempTestDir::new();
        let path = temp.create_file("test.txt", "hello");
        assert!(path.exists());
        // Temp dir will be cleaned up on drop
    }

    #[test]
    fn temp_test_dir_json_file() {
        let temp = TempTestDir::new();
        let value = serde_json::json!({"key": "value"});
        let path = temp.create_json_file("data.json", value.clone());
        let content = std::fs::read_to_string(&path).expect("Failed to read");
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("Failed to parse");
        assert_eq!(parsed, value);
    }

    #[test]
    fn mock_http_response_builders() {
        let json_resp = MockHttpResponse::json(200, serde_json::json!({"ok": true}));
        assert_eq!(json_resp.status, 200);
        assert!(json_resp.body.contains("ok"));

        let not_found = MockHttpResponse::not_found();
        assert_eq!(not_found.status, 404);

        let error = MockHttpResponse::server_error();
        assert_eq!(error.status, 500);
    }

    #[test]
    fn mock_page_context_operations() {
        let mut ctx = MockPageContext::new();
        ctx.record_js("test();");
        assert!(ctx.get_last_js().is_some());
        assert!(ctx.get_last_js().unwrap().contains("test"));

        let data = build_local_storage_data();
        ctx = ctx.with_local_storage("example.com", data);
        let json = ctx.export_local_storage_json("example.com").unwrap();
        assert!(json.contains("user_pref"));
    }

    #[test]
    fn test_builders() {
        let cookie = build_mock_cookie("session", "abc", "example.com");
        assert_eq!(cookie["name"], "session");
        assert_eq!(cookie["domain"], "example.com");

        let map = build_string_map(&[("a", "1"), ("b", "2")]);
        assert_eq!(map.len(), 2);
    }
}
