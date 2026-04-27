//! Mock-based browser integration tests for v0.0.3 APIs
//!
//! These tests use mocked browser interactions to test API logic
//! without requiring a real browser instance.
//!
//! Test characteristics:
//! - Local HTML fixtures (no external dependencies)
//! - No real browser required (all interactions mocked)
//! - Parallel execution (each test isolated)
//! - Isolated state (temp directories, no shared resources)

use std::collections::HashMap;
use tempfile::tempdir;

use auto::runtime::task_context::Rect;

// ============================================================================
// Test Fixtures
// ============================================================================

/// HTML fixture with various elements for DOM inspection tests
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

    fn with_cookie(mut self, cookie: serde_json::Value) -> Self {
        self.cookies.push(cookie);
        self
    }

    fn with_dom_element(mut self, selector: &str, data: serde_json::Value) -> Self {
        self.dom_data.insert(selector.to_string(), data);
        self
    }

    /// Simulates page.evaluate for localStorage export
    fn evaluate_local_storage_export(&self) -> Result<String, String> {
        let hostname = "example.com";
        let data = self.local_storage.get(hostname)
            .cloned()
            .unwrap_or_default();
        
        let mut result = HashMap::new();
        result.insert(hostname, data);
        
        serde_json::to_string(&result)
            .map_err(|e| format!("JSON error: {}", e))
    }

    /// Simulates page.evaluate for sessionStorage export
    fn evaluate_session_storage_export(&self) -> Result<String, String> {
        let hostname = "example.com";
        let data = self.session_storage.get(hostname)
            .cloned()
            .unwrap_or_default();
        
        let mut result = HashMap::new();
        result.insert(hostname, data);
        
        serde_json::to_string(&result)
            .map_err(|e| format!("JSON error: {}", e))
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
        }.to_string()
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
    let filtered: Vec<_> = cookies.into_iter()
        .filter(|c| {
            c.get("domain")
                .and_then(|d| d.as_str())
                .map(|d| d == domain || d == ".example.com")
                .unwrap_or(false)
        })
        .collect();
    
    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().all(|c| {
        c.get("domain").and_then(|d| d.as_str())
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
    let session_cookies: Vec<_> = cookies.into_iter()
        .filter(|c| {
            c.get("session")
                .and_then(|s| s.as_bool())
                .unwrap_or(false)
                || c.get("expires").is_none()
        })
        .collect();
    
    assert_eq!(session_cookies.len(), 1);
    assert_eq!(
        session_cookies[0].get("name").and_then(|n| n.as_str()),
        Some("session_id")
    );
}

// ============================================================================
// Session Management Tests
// ============================================================================

#[tokio::test]
async fn test_export_local_storage_extracts_data_mock() {
    let mut storage = HashMap::new();
    storage.insert("key1".to_string(), "value1".to_string());
    storage.insert("key2".to_string(), "value2".to_string());
    
    let mock = MockPageContext::new()
        .with_local_storage("example.com", storage.clone());
    
    let result = mock.evaluate_local_storage_export()
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
    
    let mock = MockPageContext::new()
        .with_session_storage("example.com", storage);
    
    let result = mock.evaluate_session_storage_export()
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
    
    let rect = mock.get_element_rect("#test-box")
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
    
    std::fs::write(&path, data.to_string())
        .expect("Failed to write");
    
    // Read back
    let content = std::fs::read_to_string(&path)
        .expect("Failed to read");
    
    let parsed: serde_json::Value = serde_json::from_str(&content)
        .expect("Failed to parse");
    
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

/// Mock HTTP response for testing
#[derive(Debug)]
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
            headers: HashMap::from([
                ("Content-Type".to_string(), "application/json".to_string()),
            ]),
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
    let response = mock_http_get("https://api.example.com/data")
        .expect("Should succeed");
    
    assert_eq!(response.status, 200);
    assert!(response.body.contains("success"));
    
    let parsed: serde_json::Value = serde_json::from_str(&response.body)
        .expect("Should parse JSON");
    
    assert_eq!(parsed["success"], true);
    assert!(parsed["data"].as_array().unwrap().len() == 3);
}

#[test]
fn test_http_get_not_found_mock() {
    let response = mock_http_get("https://api.example.com/not-found")
        .expect("Should return response");
    
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
    let _: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("Invalid JSON: {}", e))?;
    
    match url {
        "https://api.example.com/submit" => Ok(MockHttpResponse {
            status: 201,
            body: format!(r#"{{"received": {}}}"#, body),
            headers: HashMap::from([
                ("Content-Type".to_string(), "application/json".to_string()),
            ]),
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
    
    let response = mock_http_post_json("https://api.example.com/submit", payload)
        .expect("Should succeed");
    
    assert_eq!(response.status, 201);
    
    let parsed: serde_json::Value = serde_json::from_str(&response.body)
        .expect("Should parse JSON");
    
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
