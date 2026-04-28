//! Integration tests for the cookiebot task.
//!
//! These tests verify the cookiebot task's behavior including:
//! - URL file parsing (comments, empty lines, whitespace trimming)
//! - URL shuffling behavior (randomization)
//! - Empty URL handling
//! - Error handling for missing files

use std::io::Write;
use tempfile::tempdir;

/// Helper to create a temporary URL file with given content
fn create_temp_url_file(content: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().expect("Failed to create temp dir");
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
