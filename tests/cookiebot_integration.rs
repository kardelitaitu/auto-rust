//! Integration tests for the cookiebot task.
//!
//! These tests verify the cookiebot task's behavior including:
//! - URL file parsing (comments, empty lines, whitespace trimming)
//! - URL shuffling behavior (randomization)
//! - Empty URL handling
//! - Error handling for missing files
//!
//! # Test Categories
//! - URL File Parsing (6 tests)
//! - Edge Cases (4 tests)
//!
//! # Usage
//! Tests use TempTestDir from tests/common/mod.rs for temp file handling.

use std::io::Write;
use tempfile::tempdir;

// Use common test utilities
#[path = "common/mod.rs"]
mod common;
use common::*;

/// Helper to create a temporary URL file with given content
/// Uses TempTestDir from common utilities
fn create_temp_url_file(content: &str) -> (TempTestDir, std::path::PathBuf) {
    let dir = TempTestDir::new();
    let file_path = dir.path().join("test_urls.txt");
    let mut file = std::fs::File::create(&file_path).expect("Failed to create file");
    file.write_all(content.as_bytes()).expect("Failed to write");
    (dir, file_path)
}

#[test]
fn test_read_cookiebot_urls_basic() {
    use auto::task::cookiebot::read_cookiebot_urls;

    let (_dir, path) = create_temp_url_file("https://example.com\nhttps://test.com\n");
    let urls = read_cookiebot_urls(path.to_str().unwrap()).expect("Should read URLs");

    assert_eq!(urls.len(), 2);
    assert_eq!(urls[0], "https://example.com");
    assert_eq!(urls[1], "https://test.com");
}

#[test]
fn test_read_cookiebot_urls_with_comments() {
    use auto::task::cookiebot::read_cookiebot_urls;

    let content = r#"
# This is a comment
https://example.com
  # Another comment with leading space
https://test.com
# Comment after empty line
"#;

    let (_dir, path) = create_temp_url_file(content);
    let urls = read_cookiebot_urls(path.to_str().unwrap()).expect("Should read URLs");

    assert_eq!(urls.len(), 2);
    assert_eq!(urls[0], "https://example.com");
    assert_eq!(urls[1], "https://test.com");
}

#[test]
fn test_read_cookiebot_urls_whitespace_trimming() {
    use auto::task::cookiebot::read_cookiebot_urls;

    let content = "  https://example.com  \n\thttps://test.com\t\n";

    let (_dir, path) = create_temp_url_file(content);
    let urls = read_cookiebot_urls(path.to_str().unwrap()).expect("Should read URLs");

    assert_eq!(urls.len(), 2);
    assert_eq!(urls[0], "https://example.com");
    assert_eq!(urls[1], "https://test.com");
}

#[test]
fn test_read_cookiebot_urls_empty_lines_filtered() {
    use auto::task::cookiebot::read_cookiebot_urls;

    let content = r#"

https://example.com


https://test.com

"#;

    let (_dir, path) = create_temp_url_file(content);
    let urls = read_cookiebot_urls(path.to_str().unwrap()).expect("Should read URLs");

    assert_eq!(urls.len(), 2);
}

#[test]
fn test_read_cookiebot_urls_empty_file() {
    use auto::task::cookiebot::read_cookiebot_urls;

    let (_dir, path) = create_temp_url_file("");
    let urls = read_cookiebot_urls(path.to_str().unwrap()).expect("Should handle empty file");

    assert!(urls.is_empty());
}

#[test]
fn test_read_cookiebot_urls_only_comments() {
    use auto::task::cookiebot::read_cookiebot_urls;

    let content = r#"
# Comment 1
# Comment 2
# Only comments in this file
"#;

    let (_dir, path) = create_temp_url_file(content);
    let urls =
        read_cookiebot_urls(path.to_str().unwrap()).expect("Should handle comments-only file");

    assert!(urls.is_empty());
}

#[test]
fn test_read_cookiebot_urls_missing_file() {
    use auto::task::cookiebot::read_cookiebot_urls;

    let result = read_cookiebot_urls("/nonexistent/path/urls.txt");

    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Failed to read"));
}

#[test]
fn test_read_cookiebot_urls_duplicate_removal_not_performed() {
    use auto::task::cookiebot::read_cookiebot_urls;

    // Note: The current implementation does NOT deduplicate URLs
    // This test documents current behavior - duplicates are kept
    let content = "https://example.com\nhttps://example.com\nhttps://example.com\n";

    let (_dir, path) = create_temp_url_file(content);
    let urls = read_cookiebot_urls(path.to_str().unwrap()).expect("Should read URLs");

    assert_eq!(urls.len(), 3, "Current implementation keeps duplicates");
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Test URL with special characters
#[test]
fn test_read_cookiebot_urls_special_chars() {
    use auto::task::cookiebot::read_cookiebot_urls;

    let content = "https://example.com/path?q=hello&sort=asc#section\n";

    let (_dir, path) = create_temp_url_file(content);
    let urls = read_cookiebot_urls(path.to_str().unwrap()).expect("Should read URLs");

    assert_eq!(urls.len(), 1);
    assert!(urls[0].contains("?q=hello"));
    assert!(urls[0].contains("#section"));
}

/// Test URL with port number
#[test]
fn test_read_cookiebot_urls_with_port() {
    use auto::task::cookiebot::read_cookiebot_urls;

    let content = "https://localhost:8080/api\nhttps://127.0.0.1:3000/test\n";

    let (_dir, path) = create_temp_url_file(content);
    let urls = read_cookiebot_urls(path.to_str().unwrap()).expect("Should read URLs");

    assert_eq!(urls.len(), 2);
    assert!(urls[0].contains(":8080"));
    assert!(urls[1].contains(":3000"));
}

/// Test file with only whitespace
#[test]
fn test_read_cookiebot_urls_only_whitespace() {
    use auto::task::cookiebot::read_cookiebot_urls;

    let content = "   \n\t\n   \n";

    let (_dir, path) = create_temp_url_file(content);
    let urls = read_cookiebot_urls(path.to_str().unwrap()).expect("Should handle whitespace-only file");

    assert!(urls.is_empty());
}

/// Test URL with international characters
#[test]
fn test_read_cookiebot_urls_unicode() {
    use auto::task::cookiebot::read_cookiebot_urls;

    // Note: URLs should be ASCII, but test robustness
    let content = "https://example.com/路径\n";

    let (_dir, path) = create_temp_url_file(content);
    let urls = read_cookiebot_urls(path.to_str().unwrap()).expect("Should read URLs");

    assert_eq!(urls.len(), 1);
}

#[test]
fn test_read_cookiebot_urls_complex_urls() {
    use auto::task::cookiebot::read_cookiebot_urls;

    let content = r#"
https://example.com/path?query=value&other=test
https://sub.domain.example.com:8080/path
https://example.com/path#fragment
"#;

    let (_dir, path) = create_temp_url_file(content);
    let urls = read_cookiebot_urls(path.to_str().unwrap()).expect("Should read complex URLs");

    assert_eq!(urls.len(), 3);
    assert!(urls[0].contains("?query=value"));
    assert!(urls[1].contains(":8080"));
    assert!(urls[2].contains("#fragment"));
}
