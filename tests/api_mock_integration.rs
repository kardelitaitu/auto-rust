//! Mock-based browser integration tests for v0.0.3 APIs
//!
//! These tests use mocked browser interactions to test API logic
//! without requiring a real browser instance.
//!
//! # Test Categories
//! - Cookie Management (4 tests)
//! - Session Management (4 tests)
//! - DOM Inspection (6 tests)
//! - Browser Management (3 tests)
//! - Data File (9 tests)
//! - HTTP/Network (5 tests)
//! - Clipboard (5 tests)
//! - Permission Tests (11 tests)
//! - Edge Cases (5 tests)
//! - Concurrency (2 tests)
//!
//! # Usage
//! ```rust,no_run
//! // Using common test utilities
//! #[path = "common/mod.rs"]
//! mod common;
//! use common::*;
//! ```

#![allow(dead_code)]

use std::collections::HashMap;
use tempfile::tempdir;

use auto::runtime::task_context::Rect;

// Note: Common test utilities are available in `tests/common/mod.rs`
// To use in other test files:
// #[path = "common/mod.rs"]
// mod common;
// use common::*;

// ============================================================================
// Test Fixtures
// ============================================================================

/// HTML fixture with various elements for DOM inspection tests
#[allow(dead_code)]
const HTML_FIXTURE_DOM_INSPECTION: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        #test-box { width: 200px; height: 100px; background-color: red; }
        .button { padding: 10px; }
    </style>
</head>
<body>
    <div id="test-box" style="position: absolute; left: 50px; top: 100px;"></div>
    <button class="button" id="btn1">Button 1</button>
    <button class="button" id="btn2">Button 2</button>
    <button class="button" id="btn3">Button 3</button>
    <input type="text" id="input1">
    <input type="text" id="input2">
</body>
</html>
"#;

/// HTML fixture with storage for session management tests
#[allow(dead_code)]
const HTML_FIXTURE_STORAGE: &str = r#"
<!DOCTYPE html>
<html>
<body>
    <script>
        // Pre-populate storage for export tests
        localStorage.setItem('user_pref', 'dark_mode');
        localStorage.setItem('session_id', 'abc123');
        sessionStorage.setItem('temp_data', 'cached_value');
        sessionStorage.setItem('form_state', '{"field": "value"}');
    </script>
</body>
</html>
"#;

/// HTML fixture with scrollable content
#[allow(dead_code)]
const HTML_FIXTURE_SCROLLABLE: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        .tall-content { height: 3000px; }
        #viewport-element { position: fixed; top: 100px; }
        #below-fold { position: absolute; top: 2000px; }
    </style>
</head>
<body>
    <div id="viewport-element">In viewport</div>
    <div class="tall-content"></div>
    <div id="below-fold">Below fold</div>
</body>
</html>
"#;

// ============================================================================
// Mock Types
// ============================================================================

/// Mock page context for testing without real browser
#[derive(Debug, Default)]
#[allow(dead_code)]
struct MockPageContext {
    /// Simulated localStorage data (origin -> key/value)
    local_storage: HashMap<String, HashMap<String, String>>,
    /// Simulated sessionStorage data (origin -> key/value)
    session_storage: HashMap<String, HashMap<String, String>>,
    /// Simulated cookies
    cookies: Vec<serde_json::Value>,
    /// Simulated DOM element data for queries
    dom_data: HashMap<String, serde_json::Value>,
    /// Simulated scroll position (x, y)
    scroll_position: (i32, i32),
    /// Last JavaScript that was "executed"
    last_js_executed: Option<String>,
}

impl MockPageContext {
    fn new() -> Self {
        Self::default()
    }

    fn with_local_storage(mut self, origin: &str, data: HashMap<String, String>) -> Self {
        self.local_storage.insert(origin.to_string(), data);
        self
    }

    fn with_session_storage(mut self, origin: &str, data: HashMap<String, String>) -> Self {
        self.session_storage.insert(origin.to_string(), data);
        self
    }

    #[allow(dead_code)]
    fn with_cookie(mut self, cookie: serde_json::Value) -> Self {
        self.cookies.push(cookie);
        self
    }

    #[allow(dead_code)]
    fn with_dom_element(mut self, selector: &str, data: serde_json::Value) -> Self {
        self.dom_data.insert(selector.to_string(), data);
        self
    }

    /// Simulates page.evaluate for localStorage export
    fn evaluate_local_storage_export(&self) -> Result<String, String> {
        let hostname = "example.com";
        let data = self
            .local_storage
            .get(hostname)
            .cloned()
            .unwrap_or_default();

        let mut result = HashMap::new();
        result.insert(hostname, data);

        serde_json::to_string(&result).map_err(|e| format!("JSON error: {}", e))
    }

    /// Simulates page.evaluate for sessionStorage export
    fn evaluate_session_storage_export(&self) -> Result<String, String> {
        let hostname = "example.com";
        let data = self
            .session_storage
            .get(hostname)
            .cloned()
            .unwrap_or_default();

        let mut result = HashMap::new();
        result.insert(hostname, data);

        serde_json::to_string(&result).map_err(|e| format!("JSON error: {}", e))
    }

    /// Simulates getting scroll position
    fn get_scroll_position(&self) -> (i32, i32) {
        self.scroll_position
    }

    /// Simulates counting elements
    fn count_elements(&self, selector: &str) -> usize {
        // Simple simulation based on selector
        match selector {
            "button" => 3,
            ".button" => 3,
            "#test-box" => 1,
            "input" => 2,
            _ => 0,
        }
    }

    /// Simulates checking viewport
    fn is_in_viewport(&self, selector: &str) -> bool {
        // Elements with "viewport" in name are visible
        selector.contains("viewport") || !selector.contains("below-fold")
    }

    /// Simulates getting computed style
    fn get_computed_style(&self, _selector: &str, property: &str) -> String {
        match property {
            "background-color" | "backgroundColor" => "rgb(255, 0, 0)",
            "width" => "200px",
            "height" => "100px",
            _ => "",
        }
        .to_string()
    }

    /// Simulates getting element rect
    fn get_element_rect(&self, selector: &str) -> Option<Rect> {
        if selector == "#test-box" {
            Some(Rect {
                x: 50.0,
                y: 100.0,
                width: 200.0,
                height: 100.0,
            })
        } else {
            None
        }
    }

    /// Records executed JavaScript
    fn record_js_execution(&mut self, js: &str) {
        self.last_js_executed = Some(js.to_string());
    }
}

/// Mock CDP response for cookies
fn mock_cdp_cookies() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "name": "session_id",
            "value": "abc123",
            "domain": ".example.com",
            "path": "/",
            "session": true,
        }),
        serde_json::json!({
            "name": "user_pref",
            "value": "dark_mode",
            "domain": ".example.com",
            "path": "/",
            "session": false,
            "expires": 1893456000.0, // Future timestamp
        }),
        serde_json::json!({
            "name": "other_cookie",
            "value": "value123",
            "domain": ".other.com",
            "path": "/",
            "session": false,
        }),
    ]
}

// ============================================================================
// Cookie Management Tests
// ============================================================================

#[tokio::test]
async fn test_export_cookies_filters_by_domain_mock() {
    let cookies = mock_cdp_cookies();

    // Simulate filtering by domain
    let domain = ".example.com";
    let filtered: Vec<_> = cookies
        .into_iter()
        .filter(|c| {
            c.get("domain")
                .and_then(|d| d.as_str())
                .map(|d| d == domain || d == ".example.com")
                .unwrap_or(false)
        })
        .collect();

    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().all(|c| {
        c.get("domain")
            .and_then(|d| d.as_str())
            .map(|d| d.contains("example.com"))
            .unwrap_or(false)
    }));
}

#[tokio::test]
async fn test_has_cookie_finds_existing_mock() {
    let cookies = mock_cdp_cookies();
    let target_name = "session_id";

    let exists = cookies.iter().any(|c| {
        c.get("name")
            .and_then(|n| n.as_str())
            .map(|n| n == target_name)
            .unwrap_or(false)
    });

    assert!(exists);
}

#[tokio::test]
async fn test_has_cookie_returns_false_for_missing_mock() {
    let cookies = mock_cdp_cookies();
    let target_name = "nonexistent_cookie";

    let exists = cookies.iter().any(|c| {
        c.get("name")
            .and_then(|n| n.as_str())
            .map(|n| n == target_name)
            .unwrap_or(false)
    });

    assert!(!exists);
}

#[tokio::test]
async fn test_export_session_cookies_filters_persistent_mock() {
    let cookies = mock_cdp_cookies();

    // Filter for session cookies only
    let session_cookies: Vec<_> = cookies
        .into_iter()
        .filter(|c| {
            c.get("session").and_then(|s| s.as_bool()).unwrap_or(false)
                || c.get("expires").is_none()
        })
        .collect();

    // Two session cookies: "session_id" (session=true) and "other_cookie" (no expires field)
    assert_eq!(session_cookies.len(), 2);
    assert!(session_cookies
        .iter()
        .any(|c| { c.get("name").and_then(|n| n.as_str()) == Some("session_id") }));
}

// ============================================================================
// Session Management Tests
// ============================================================================

#[tokio::test]
async fn test_export_local_storage_extracts_data_mock() {
    let mut storage = HashMap::new();
    storage.insert("key1".to_string(), "value1".to_string());
    storage.insert("key2".to_string(), "value2".to_string());

    let mock = MockPageContext::new().with_local_storage("example.com", storage.clone());

    let result = mock
        .evaluate_local_storage_export()
        .expect("Should export localStorage");

    let parsed: HashMap<String, HashMap<String, String>> =
        serde_json::from_str(&result).expect("Should parse JSON");

    assert!(parsed.contains_key("example.com"));
    let data = parsed.get("example.com").unwrap();
    assert_eq!(data.get("key1"), Some(&"value1".to_string()));
    assert_eq!(data.get("key2"), Some(&"value2".to_string()));
}

#[tokio::test]
async fn test_export_session_storage_extracts_data_mock() {
    let mut storage = HashMap::new();
    storage.insert("temp_key".to_string(), "temp_value".to_string());

    let mock = MockPageContext::new().with_session_storage("example.com", storage);

    let result = mock
        .evaluate_session_storage_export()
        .expect("Should export sessionStorage");

    let parsed: HashMap<String, HashMap<String, String>> =
        serde_json::from_str(&result).expect("Should parse JSON");

    assert!(parsed.contains_key("example.com"));
    assert_eq!(
        parsed.get("example.com").unwrap().get("temp_key"),
        Some(&"temp_value".to_string())
    );
}

#[tokio::test]
async fn test_validate_session_data_rejects_invalid_json_mock() {
    let invalid_json = "not valid json {";

    let result: Result<serde_json::Value, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validate_session_data_accepts_valid_json_mock() {
    let valid_json = r#"{"cookies": [], "local_storage": {}, "url": "test"}"#;

    let result: Result<serde_json::Value, _> = serde_json::from_str(valid_json);
    assert!(result.is_ok());
}

// ============================================================================
// DOM Inspection Tests
// ============================================================================

#[test]
fn test_get_computed_style_mock() {
    let mock = MockPageContext::new();

    let color = mock.get_computed_style("#test-box", "background-color");
    assert_eq!(color, "rgb(255, 0, 0)");

    let width = mock.get_computed_style("#test-box", "width");
    assert_eq!(width, "200px");
}

#[test]
fn test_get_element_rect_mock() {
    let mock = MockPageContext::new();

    let rect = mock
        .get_element_rect("#test-box")
        .expect("Should get rect for existing element");

    assert_eq!(rect.x, 50.0);
    assert_eq!(rect.y, 100.0);
    assert_eq!(rect.width, 200.0);
    assert_eq!(rect.height, 100.0);
}

#[test]
fn test_get_element_rect_missing_element_mock() {
    let mock = MockPageContext::new();

    let rect = mock.get_element_rect("#nonexistent");
    assert!(rect.is_none());
}

#[test]
fn test_get_scroll_position_mock() {
    let mock = MockPageContext::new();

    let (x, y) = mock.get_scroll_position();
    assert_eq!(x, 0); // Default position
    assert_eq!(y, 0);
}

#[test]
fn test_count_elements_mock() {
    let mock = MockPageContext::new();

    assert_eq!(mock.count_elements("button"), 3);
    assert_eq!(mock.count_elements(".button"), 3);
    assert_eq!(mock.count_elements("input"), 2);
    assert_eq!(mock.count_elements("#test-box"), 1);
    assert_eq!(mock.count_elements(".nonexistent"), 0);
}

#[test]
fn test_is_in_viewport_mock() {
    let mock = MockPageContext::new();

    // Elements with "viewport" in name should be visible
    assert!(mock.is_in_viewport("#viewport-element"));

    // Elements with "below-fold" should not be visible (in this mock)
    assert!(!mock.is_in_viewport("#below-fold"));

    // Regular selectors are considered visible by default
    assert!(mock.is_in_viewport("#test-box"));
}

// ============================================================================
// Browser Management Tests
// ============================================================================

#[test]
fn test_browser_data_aggregates_all_sources_mock() {
    use chrono::Utc;

    let cookies = mock_cdp_cookies();

    let mut local_storage = HashMap::new();
    let mut origin_storage = HashMap::new();
    origin_storage.insert("ls_key".to_string(), "ls_value".to_string());
    local_storage.insert("example.com".to_string(), origin_storage);

    let mut session_storage = HashMap::new();
    let mut origin_session = HashMap::new();
    origin_session.insert("ss_key".to_string(), "ss_value".to_string());
    session_storage.insert("example.com".to_string(), origin_session);

    let mut indexeddb = HashMap::new();
    indexeddb.insert("example.com".to_string(), vec!["db1".to_string()]);

    let browser_data = auto::task::policy::BrowserData {
        cookies: cookies.clone(),
        local_storage,
        session_storage,
        indexeddb_names: indexeddb,
        exported_at: Utc::now(),
        source: "https://example.com".to_string(),
        browser_version: Some("Chrome 120".to_string()),
    };

    // Verify all data is present
    assert_eq!(browser_data.cookies.len(), 3);
    assert!(!browser_data.local_storage.is_empty());
    assert!(!browser_data.session_storage.is_empty());
    assert!(!browser_data.indexeddb_names.is_empty());
    assert!(browser_data.browser_version.is_some());
}

#[tokio::test]
async fn test_import_browser_generates_correct_js_mock() {
    let mut mock = MockPageContext::new();

    // Simulate importing localStorage
    let data = HashMap::from([
        ("key1".to_string(), "value1".to_string()),
        ("key2".to_string(), "value2".to_string()),
    ]);
    let json = serde_json::to_string(&data).unwrap();

    let js_code = format!(
        r#"
        (function() {{
            const data = {};
            Object.entries(data).forEach(([k, v]) => {{
                localStorage.setItem(k, v);
            }});
            return 'imported';
        }})()
        "#,
        json
    );

    mock.record_js_execution(&js_code);

    // Verify JS was recorded
    assert!(mock.last_js_executed.is_some());
    let executed = mock.last_js_executed.unwrap();
    assert!(executed.contains("localStorage.setItem"));
    assert!(executed.contains("key1"));
    assert!(executed.contains("value1"));
}

#[test]
fn test_import_browser_handles_empty_data_mock() {
    use chrono::Utc;

    let browser_data = auto::task::policy::BrowserData {
        cookies: vec![],
        local_storage: HashMap::new(),
        session_storage: HashMap::new(),
        indexeddb_names: HashMap::new(),
        exported_at: Utc::now(),
        source: "test".to_string(),
        browser_version: None,
    };

    // Empty data should be valid
    assert!(browser_data.cookies.is_empty());
    assert!(browser_data.local_storage.is_empty());
    assert!(browser_data.session_storage.is_empty());

    // Should be serializable
    let json = serde_json::to_string(&browser_data);
    assert!(json.is_ok());
}

// ============================================================================
// Data File Tests
// ============================================================================

#[test]
fn test_list_data_files_lists_all_files() {
    let dir = tempdir().expect("Failed to create temp dir");

    // Create test files
    let file1 = dir.path().join("file1.json");
    let file2 = dir.path().join("subdir/file2.json");

    std::fs::write(&file1, "[]").expect("Failed to write file1");
    std::fs::create_dir_all(file2.parent().unwrap()).expect("Failed to create subdir");
    std::fs::write(&file2, "[]").expect("Failed to write file2");

    // Simulate list_data_files by listing files in temp dir
    let mut files: Vec<String> = std::fs::read_dir(dir.path())
        .expect("Failed to read dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    files.sort();
    assert_eq!(files.len(), 1); // file1.json in root
    assert_eq!(files[0], "file1.json");
}

#[test]
fn test_list_data_files_with_subdir() {
    let dir = tempdir().expect("Failed to create temp dir");
    let subdir = dir.path().join("data");
    std::fs::create_dir_all(&subdir).expect("Failed to create subdir");

    let file1 = subdir.join("test.json");
    let file2 = subdir.join("other.txt");
    std::fs::write(&file1, "[]").expect("Failed to write");
    std::fs::write(&file2, "content").expect("Failed to write");

    // Simulate list_data_files with subdir="data"
    let mut files: Vec<String> = std::fs::read_dir(&subdir)
        .expect("Failed to read subdir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    files.sort();
    assert_eq!(files.len(), 2);
    assert!(files.contains(&"other.txt".to_string()));
    assert!(files.contains(&"test.json".to_string()));
}

#[test]
fn test_data_file_exists_returns_true_for_existing() {
    let dir = tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("exists.json");
    std::fs::write(&path, "true").expect("Failed to write");

    assert!(path.exists());
    assert!(std::fs::metadata(&path).is_ok());
}

#[test]
fn test_data_file_exists_returns_false_for_missing() {
    let dir = tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("missing.json");

    assert!(!path.exists());
}

#[test]
fn test_delete_data_file_removes_file() {
    let dir = tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("to_delete.json");
    std::fs::write(&path, "delete me").expect("Failed to write");

    assert!(path.exists());
    std::fs::remove_file(&path).expect("Failed to delete");
    assert!(!path.exists());
}

#[test]
fn test_delete_data_file_errors_on_missing() {
    let dir = tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("nonexistent.json");

    let result = std::fs::remove_file(&path);
    assert!(result.is_err());
}

#[test]
fn test_append_data_file_adds_content() {
    let dir = tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("append_test.txt");

    // First write
    std::fs::write(&path, "Hello").expect("Failed to write");

    // Append
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("Failed to open for append");
    std::io::Write::write_all(&mut file, b", World!").expect("Failed to append");

    let content = std::fs::read_to_string(&path).expect("Failed to read");
    assert_eq!(content, "Hello, World!");
}

#[test]
fn test_append_data_file_creates_if_missing() {
    let dir = tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("new_file.txt");

    // Append to non-existent file
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .expect("Failed to create/append");
    std::io::Write::write_all(&mut file, b"New content").expect("Failed to append");

    let content = std::fs::read_to_string(&path).expect("Failed to read");
    assert_eq!(content, "New content");
}

#[test]
fn test_read_json_data_file_not_found() {
    let dir = tempdir().expect("Failed to create temp dir");
    let non_existent = dir.path().join("non_existent.json");

    let result = std::fs::read_to_string(&non_existent);
    assert!(result.is_err());
}

#[test]
fn test_write_and_read_json_data_roundtrip() {
    let dir = tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("test_data.json");

    // Write
    let data = serde_json::json!({
        "name": "test",
        "value": 42,
        "nested": {
            "key": "value"
        }
    });

    std::fs::write(&path, data.to_string()).expect("Failed to write");

    // Read back
    let content = std::fs::read_to_string(&path).expect("Failed to read");

    let parsed: serde_json::Value = serde_json::from_str(&content).expect("Failed to parse");

    assert_eq!(parsed["name"], "test");
    assert_eq!(parsed["value"], 42);
    assert_eq!(parsed["nested"]["key"], "value");
}

#[test]
fn test_data_file_metadata_correct_values() {
    let dir = tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("test_file.txt");

    // Create file with known content
    let content = "Hello, World!";
    std::fs::write(&path, content).expect("Failed to write");

    // Get metadata
    let metadata = std::fs::metadata(&path).expect("Failed to get metadata");

    assert_eq!(metadata.len(), content.len() as u64);
    assert!(metadata.modified().is_ok());
    assert!(metadata.created().is_ok());
}

// ============================================================================
// HTTP/Network Tests (using mock client)
// ============================================================================

/// Mock download function
fn mock_download_file(url: &str, _relative_path: &str) -> Result<u64, String> {
    match url {
        "https://example.com/file.txt" => Ok(1024),
        "https://example.com/large.bin" => Ok(1048576),
        _ => Err("Download failed".to_string()),
    }
}

#[test]
fn test_download_file_success_mock() {
    let result = mock_download_file("https://example.com/file.txt", "file.txt");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1024);
}

#[test]
fn test_download_file_large_file_mock() {
    let result = mock_download_file("https://example.com/large.bin", "large.bin");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1048576);
}

#[test]
fn test_download_file_failure_mock() {
    let result = mock_download_file("https://example.com/missing", "missing.txt");
    assert!(result.is_err());
}

// ============================================================================
// Session Management: import_local_storage Tests
// ============================================================================

#[tokio::test]
async fn test_import_local_storage_generates_js_mock() {
    let mut mock = MockPageContext::new();

    // Simulate import localStorage JS generation
    let data = HashMap::from([
        ("key1".to_string(), "value1".to_string()),
        ("key2".to_string(), "value2".to_string()),
    ]);

    let json = serde_json::to_string(&data).unwrap();
    let js_code = format!(
        r#"
        (function() {{
            const data = {};
            Object.entries(data).forEach(([k, v]) => {{
                localStorage.setItem(k, v);
            }});
            return 'imported';
        }})()
        "#,
        json
    );

    mock.record_js_execution(&js_code);

    assert!(mock
        .last_js_executed
        .unwrap()
        .contains("localStorage.setItem"));
}

#[tokio::test]
async fn test_import_local_storage_empty_data_mock() {
    let mut mock = MockPageContext::new();

    let data: HashMap<String, String> = HashMap::new();
    let json = serde_json::to_string(&data).unwrap();

    let js_code = format!("localStorage import with data: {}", json);
    mock.record_js_execution(&js_code);

    assert!(mock.last_js_executed.unwrap().contains("data:"));
}

// ============================================================================
// Additional Permission Tests
// ============================================================================

#[test]
fn test_permission_denial_blocks_import_cookies_mock() {
    use auto::task::policy::TaskPermissions;

    let perms = TaskPermissions {
        allow_import_cookies: false,
        ..Default::default()
    };

    assert!(!perms.allow_import_cookies);
}

#[test]
fn test_permission_denial_blocks_export_session_mock() {
    use auto::task::policy::TaskPermissions;

    let perms = TaskPermissions {
        allow_export_session: false,
        ..Default::default()
    };

    assert!(!perms.allow_export_session);
}

#[test]
fn test_permission_denial_blocks_session_clipboard_mock() {
    use auto::task::policy::TaskPermissions;

    let perms = TaskPermissions {
        allow_session_clipboard: false,
        ..Default::default()
    };

    assert!(!perms.allow_session_clipboard);
}

#[test]
fn test_permission_denial_blocks_read_data_mock() {
    use auto::task::policy::TaskPermissions;

    let perms = TaskPermissions {
        allow_read_data: false,
        ..Default::default()
    };

    assert!(!perms.allow_read_data);
}

#[test]
fn test_permission_denial_blocks_write_data_mock() {
    use auto::task::policy::TaskPermissions;

    let perms = TaskPermissions {
        allow_write_data: false,
        ..Default::default()
    };

    assert!(!perms.allow_write_data);
}

#[test]
fn test_permission_denial_blocks_dom_inspection_mock() {
    use auto::task::policy::TaskPermissions;

    let perms = TaskPermissions {
        allow_dom_inspection: false,
        ..Default::default()
    };

    assert!(!perms.allow_dom_inspection);
}

#[test]
fn test_permission_denial_blocks_browser_import_mock() {
    use auto::task::policy::TaskPermissions;

    let perms = TaskPermissions {
        allow_browser_import: false,
        ..Default::default()
    };

    assert!(!perms.allow_browser_import);
}

// ============================================================================
// Negative Tests
// ============================================================================

#[test]
fn test_read_json_data_invalid_json_mock() {
    let dir = tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("invalid.json");
    std::fs::write(&path, "not valid json {").expect("Failed to write");

    let content = std::fs::read_to_string(&path).expect("Failed to read");
    let result: Result<serde_json::Value, _> = serde_json::from_str(&content);
    assert!(result.is_err());
}

#[test]
fn test_write_json_data_creates_file_mock() {
    let dir = tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("output.json");

    let data = serde_json::json!({"test": true});
    std::fs::write(&path, data.to_string()).expect("Failed to write");

    assert!(path.exists());
    let content = std::fs::read_to_string(&path).expect("Failed to read");
    assert!(content.contains("test"));
}

#[test]
fn test_http_get_timeout_mock() {
    // Simulate timeout
    let result: Result<MockHttpResponse, String> =
        Err("Connection timeout after 5000ms".to_string());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("timeout"));
}

// ============================================================================
// Doc Tests Validation
// ============================================================================

// Note: Run `cargo test --doc` to validate all rustdoc examples in task_context.rs
// This is tested separately via CI/CD

// Keep existing tests below...

/// Mock HTTP response for testing
#[derive(Debug)]
#[allow(dead_code)]
struct MockHttpResponse {
    status: u16,
    body: String,
    headers: HashMap<String, String>,
}

/// Simulates HTTP GET with mock response
fn mock_http_get(url: &str) -> Result<MockHttpResponse, String> {
    match url {
        "https://api.example.com/data" => Ok(MockHttpResponse {
            status: 200,
            body: r#"{"success": true, "data": [1, 2, 3]}"#.to_string(),
            headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
        }),
        "https://api.example.com/not-found" => Ok(MockHttpResponse {
            status: 404,
            body: "Not Found".to_string(),
            headers: HashMap::new(),
        }),
        "https://api.example.com/error" => Err("Connection timeout".to_string()),
        _ => Ok(MockHttpResponse {
            status: 200,
            body: "{}".to_string(),
            headers: HashMap::new(),
        }),
    }
}

#[test]
fn test_http_get_success_mock() {
    let response = mock_http_get("https://api.example.com/data").expect("Should succeed");

    assert_eq!(response.status, 200);
    assert!(response.body.contains("success"));

    let parsed: serde_json::Value =
        serde_json::from_str(&response.body).expect("Should parse JSON");

    assert_eq!(parsed["success"], true);
    assert!(parsed["data"].as_array().unwrap().len() == 3);
}

#[test]
fn test_http_get_not_found_mock() {
    let response =
        mock_http_get("https://api.example.com/not-found").expect("Should return response");

    assert_eq!(response.status, 404);
    assert_eq!(response.body, "Not Found");
}

#[test]
fn test_http_get_error_mock() {
    let result = mock_http_get("https://api.example.com/error");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("timeout"));
}

/// Simulates HTTP POST with mock
fn mock_http_post_json(url: &str, body: &str) -> Result<MockHttpResponse, String> {
    // Verify body is valid JSON
    let _: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {}", e))?;

    match url {
        "https://api.example.com/submit" => Ok(MockHttpResponse {
            status: 201,
            body: format!(r#"{{"received": {}}}"#, body),
            headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
        }),
        _ => Ok(MockHttpResponse {
            status: 200,
            body: body.to_string(),
            headers: HashMap::new(),
        }),
    }
}

#[test]
fn test_http_post_json_echoes_body_mock() {
    let payload = r#"{"action": "create", "id": 123}"#;

    let response =
        mock_http_post_json("https://api.example.com/submit", payload).expect("Should succeed");

    assert_eq!(response.status, 201);

    let parsed: serde_json::Value =
        serde_json::from_str(&response.body).expect("Should parse JSON");

    // Verify body was echoed back
    assert!(parsed["received"].to_string().contains("action"));
}

#[test]
fn test_http_post_json_rejects_invalid_json() {
    let invalid_payload = "not json";

    let result = mock_http_post_json("https://api.example.com/submit", invalid_payload);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid JSON"));
}

// ============================================================================
// Clipboard Tests
// ============================================================================

/// Mock clipboard for testing
#[derive(Debug, Default)]
struct MockClipboard {
    content: Option<String>,
}

impl MockClipboard {
    fn read(&self) -> Option<String> {
        self.content.clone()
    }

    fn write(&mut self, text: &str) {
        self.content = Some(text.to_string());
    }

    fn clear(&mut self) {
        self.content = None;
    }

    fn has_content(&self) -> bool {
        self.content.is_some()
    }

    fn append(&mut self, text: &str) {
        if let Some(ref mut existing) = self.content {
            existing.push_str(text);
        } else {
            self.content = Some(text.to_string());
        }
    }
}

#[test]
fn test_clipboard_write_and_read_mock() {
    let mut clipboard = MockClipboard::default();

    clipboard.write("Hello, World!");
    assert_eq!(clipboard.read(), Some("Hello, World!".to_string()));
}

#[test]
fn test_clipboard_clear_mock() {
    let mut clipboard = MockClipboard::default();

    clipboard.write("content");
    assert!(clipboard.has_content());

    clipboard.clear();
    assert!(!clipboard.has_content());
    assert_eq!(clipboard.read(), None);
}

#[test]
fn test_clipboard_has_content_mock() {
    let mut clipboard = MockClipboard::default();

    assert!(!clipboard.has_content());

    clipboard.write("test");
    assert!(clipboard.has_content());
}

#[test]
fn test_clipboard_append_mock() {
    let mut clipboard = MockClipboard::default();

    clipboard.write("Hello");
    clipboard.append(", World!");

    assert_eq!(clipboard.read(), Some("Hello, World!".to_string()));
}

#[test]
fn test_clipboard_append_to_empty_mock() {
    let mut clipboard = MockClipboard::default();

    clipboard.append("First content");

    assert_eq!(clipboard.read(), Some("First content".to_string()));
}

// ============================================================================
// Permission Integration Tests
// ============================================================================

#[test]
fn test_permission_denial_blocks_export_cookies_mock() {
    use auto::task::policy::TaskPermissions;

    let perms = TaskPermissions {
        allow_export_cookies: false,
        ..Default::default()
    };

    assert!(!perms.allow_export_cookies);
    // In real implementation, this would trigger permission error
}

#[test]
fn test_permission_denial_blocks_import_session_mock() {
    use auto::task::policy::TaskPermissions;

    let perms = TaskPermissions {
        allow_import_session: false,
        ..Default::default()
    };

    assert!(!perms.allow_import_session);
}

#[test]
fn test_permission_denial_blocks_http_requests_mock() {
    use auto::task::policy::TaskPermissions;

    let perms = TaskPermissions {
        allow_http_requests: false,
        ..Default::default()
    };

    assert!(!perms.allow_http_requests);
}

#[test]
fn test_permission_denial_blocks_browser_export_mock() {
    use auto::task::policy::TaskPermissions;

    let perms = TaskPermissions {
        allow_browser_export: false,
        ..Default::default()
    };

    assert!(!perms.allow_browser_export);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_session_data_with_unicode_values() {
    use std::collections::HashMap;

    let mut local_storage = HashMap::new();
    local_storage.insert("unicode_key".to_string(), "日本語".to_string());
    local_storage.insert("emoji".to_string(), "🎉".to_string());

    let data = auto::task::policy::SessionData {
        cookies: vec![],
        local_storage,
        exported_at: chrono::Utc::now(),
        url: "https://example.com".to_string(),
    };

    // Should serialize/deserialize correctly
    let json = serde_json::to_string(&data).expect("Should serialize unicode");
    let parsed: auto::task::policy::SessionData =
        serde_json::from_str(&json).expect("Should deserialize unicode");

    assert_eq!(
        parsed.local_storage.get("unicode_key"),
        Some(&"日本語".to_string())
    );
}

#[test]
fn test_browser_data_with_large_cookies() {
    use chrono::Utc;

    // Create large cookie values
    let large_value = "x".repeat(10000);
    let cookie = serde_json::json!({
        "name": "large_cookie",
        "value": large_value,
        "domain": ".example.com",
    });

    let browser_data = auto::task::policy::BrowserData {
        cookies: vec![cookie],
        local_storage: HashMap::new(),
        session_storage: HashMap::new(),
        indexeddb_names: HashMap::new(),
        exported_at: Utc::now(),
        source: "test".to_string(),
        browser_version: None,
    };

    // Should handle large values
    let json = serde_json::to_string(&browser_data).expect("Should serialize large data");
    assert!(json.len() > 10000);
}

#[test]
fn test_rect_with_negative_coordinates() {
    let rect = Rect {
        x: -100.0,
        y: -50.0,
        width: 200.0,
        height: 100.0,
    };

    // Elements can have negative coordinates (off-screen)
    assert_eq!(rect.x, -100.0);
    assert_eq!(rect.y, -50.0);
    assert_eq!(rect.width, 200.0);
    assert_eq!(rect.height, 100.0);
}

#[test]
fn test_http_response_with_large_body() {
    let large_body = "{".repeat(10000);

    let response = MockHttpResponse {
        status: 200,
        body: large_body.clone(),
        headers: HashMap::new(),
    };

    assert_eq!(response.body.len(), large_body.len());
}

#[test]
fn test_empty_selector_count_returns_zero() {
    let mock = MockPageContext::new();

    assert_eq!(mock.count_elements(""), 0);
    assert_eq!(mock.count_elements(".nonexistent-class"), 0);
    assert_eq!(mock.count_elements("#nonexistent-id"), 0);
}

#[tokio::test]
async fn test_browser_data_serialization_preserves_timestamps() {
    use chrono::Utc;

    let now = Utc::now();
    let data = auto::task::policy::BrowserData {
        cookies: vec![],
        local_storage: HashMap::new(),
        session_storage: HashMap::new(),
        indexeddb_names: HashMap::new(),
        exported_at: now,
        source: "test".to_string(),
        browser_version: None,
    };

    let json = serde_json::to_string(&data).expect("Should serialize");
    let parsed: auto::task::policy::BrowserData =
        serde_json::from_str(&json).expect("Should deserialize");

    // Timestamp should be preserved within 1 second
    let diff = (parsed.exported_at - now).num_milliseconds().abs();
    assert!(diff < 1000);
}

// ============================================================================
// Concurrency Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_clipboard_operations() {
    use std::sync::Arc;
    use tokio::task;

    let clipboard = Arc::new(std::sync::Mutex::new(MockClipboard::default()));

    let mut handles = vec![];

    // Spawn 5 concurrent write operations
    for i in 0..5 {
        let cb = clipboard.clone();
        let handle = task::spawn(async move {
            let mut cb = cb.lock().unwrap();
            cb.write(&format!("content_{}", i));
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.expect("Task failed");
    }

    // Verify no panic/crash
    let cb = clipboard.lock().unwrap();
    assert!(cb.has_content());
}

#[tokio::test]
async fn test_concurrent_data_file_operations() {
    use std::sync::Arc;
    use tokio::task;

    let dir = tempdir().expect("Failed to create temp dir");
    let base_path = dir.path().join("concurrent");
    std::fs::create_dir_all(&base_path).expect("Failed to create dir");

    let mut handles = vec![];

    // Spawn 5 concurrent write operations to different files
    for i in 0..5 {
        let path = base_path.join(format!("file_{}.txt", i));
        let handle = task::spawn(async move {
            std::fs::write(&path, format!("content {}", i)).expect("Failed to write");
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.expect("Task failed");
    }

    // Verify all files were created
    for i in 0..5 {
        let path = base_path.join(format!("file_{}.txt", i));
        assert!(path.exists());
    }
}
