//! Integration tests for CLI parsing functionality.
//! Tests task group parsing, browser filters, and payload handling.

use auto::cli::{parse_browser_filters, parse_task_groups, TaskDefinition};

/// Test browser filter parsing with multiple browsers
#[test]
fn parse_browser_filters_multiple() {
    let input = Some("chrome, firefox, edge");
    let filters = parse_browser_filters(input);

    assert_eq!(filters.len(), 3);
    assert!(filters.contains(&"chrome".to_string()));
    assert!(filters.contains(&"firefox".to_string()));
    assert!(filters.contains(&"edge".to_string()));
}

/// Test browser filter parsing with duplicates (should be deduplicated)
#[test]
fn parse_browser_filters_deduplicates() {
    let input = Some("chrome, chrome, firefox");
    let filters = parse_browser_filters(input);

    assert_eq!(filters.len(), 2);
    assert!(filters.contains(&"chrome".to_string()));
    assert!(filters.contains(&"firefox".to_string()));
}

/// Test browser filter parsing with mixed case (should normalize to lowercase)
#[test]
fn parse_browser_filters_normalizes_case() {
    let input = Some("Chrome, FIREFOX, Edge");
    let filters = parse_browser_filters(input);

    assert!(filters.contains(&"chrome".to_string()));
    assert!(filters.contains(&"firefox".to_string()));
    assert!(filters.contains(&"edge".to_string()));
}

/// Test browser filter parsing with empty input
#[test]
fn parse_browser_filters_empty() {
    let filters = parse_browser_filters(None);
    assert!(filters.is_empty());

    let filters = parse_browser_filters(Some(""));
    assert!(filters.is_empty());
}

/// Test browser filter parsing with whitespace trimming
#[test]
fn parse_browser_filters_trims_whitespace() {
    let input = Some("  chrome  ,  firefox  ");
    let filters = parse_browser_filters(input);

    assert_eq!(filters.len(), 2);
    assert!(filters.contains(&"chrome".to_string()));
    assert!(filters.contains(&"firefox".to_string()));
}

/// Test task group parsing with single task
#[test]
fn parse_task_groups_single_task() {
    let args = vec!["cookiebot".to_string()];
    let groups = parse_task_groups(&args);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].len(), 1);
    assert_eq!(groups[0][0].name, "cookiebot");
}

/// Test task group parsing with multiple tasks in same group
#[test]
fn parse_task_groups_multiple_tasks() {
    let args = vec!["cookiebot".to_string(), "pageview".to_string()];
    let groups = parse_task_groups(&args);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].len(), 2);
    assert_eq!(groups[0][0].name, "cookiebot");
    assert_eq!(groups[0][1].name, "pageview");
}

/// Test task group parsing with "then" separator
#[test]
fn parse_task_groups_with_then_separator() {
    let args = vec![
        "cookiebot".to_string(),
        "then".to_string(),
        "pageview".to_string(),
    ];
    let groups = parse_task_groups(&args);

    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].len(), 1);
    assert_eq!(groups[0][0].name, "cookiebot");
    assert_eq!(groups[1].len(), 1);
    assert_eq!(groups[1][0].name, "pageview");
}

/// Test task group parsing with multiple "then" separators
#[test]
fn parse_task_groups_multiple_then_separators() {
    let args = vec![
        "task1".to_string(),
        "then".to_string(),
        "task2".to_string(),
        "then".to_string(),
        "task3".to_string(),
    ];
    let groups = parse_task_groups(&args);

    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0][0].name, "task1");
    assert_eq!(groups[1][0].name, "task2");
    assert_eq!(groups[2][0].name, "task3");
}

/// Test task group parsing with empty args
#[test]
fn parse_task_groups_empty_args() {
    let args: Vec<String> = vec![];
    let groups = parse_task_groups(&args);

    assert!(groups.is_empty());
}

/// Test task group parsing with empty strings (should be skipped)
#[test]
fn parse_task_groups_skips_empty_strings() {
    let args = vec![
        "cookiebot".to_string(),
        "".to_string(),
        "pageview".to_string(),
    ];
    let groups = parse_task_groups(&args);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].len(), 2);
}

/// Test task group parsing with task=value shorthand
#[test]
fn parse_task_groups_task_value_shorthand() {
    let args = vec!["pageview=https://example.com".to_string()];
    let groups = parse_task_groups(&args);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0][0].name, "pageview");
    assert_eq!(
        groups[0][0].payload.get("url"),
        Some(&serde_json::Value::String(
            "https://example.com".to_string()
        ))
    );
}

/// Test task group parsing with .js suffix stripping
#[test]
fn parse_task_groups_strips_js_suffix() {
    let args = vec!["cookiebot.js".to_string()];
    let groups = parse_task_groups(&args);

    assert_eq!(groups[0][0].name, "cookiebot");
}

/// Test task group parsing with numeric value
#[test]
fn parse_task_groups_numeric_value() {
    let args = vec!["delay=5000".to_string()];
    let groups = parse_task_groups(&args);

    assert_eq!(groups[0][0].name, "delay");
    // Numeric values should be parsed as numbers
    assert!(groups[0][0].payload.contains_key("value"));
}

/// Test task definition structure
#[test]
fn task_definition_structure() {
    let task = TaskDefinition {
        name: "test".to_string(),
        payload: std::collections::HashMap::new(),
    };

    assert_eq!(task.name, "test");
    assert!(task.payload.is_empty());
}

/// Test task group parsing preserves task name case (only strips .js)
#[test]
fn parse_task_groups_preserves_task_name_case() {
    let args = vec![
        "CookieBot".to_string(),
        "then".to_string(),
        "PageView".to_string(),
    ];
    let groups = parse_task_groups(&args);

    // Task names preserve case but "then" separator is case-insensitive
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0][0].name, "CookieBot"); // preserves case
    assert_eq!(groups[1][0].name, "PageView"); // preserves case
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Test task group parsing with "then" at start (should handle gracefully)
#[test]
fn parse_task_groups_then_at_start() {
    let args = vec!["then".to_string(), "task1".to_string()];
    let groups = parse_task_groups(&args);

    // Should create empty first group + second group with task
    assert!(!groups.is_empty());
}

/// Test task group parsing with payload value containing equals
#[test]
fn parse_task_groups_payload_with_equals() {
    // The format task=value=rest treats "value=rest" as the value
    let args = vec!["task=key=value1".to_string()];
    let groups = parse_task_groups(&args);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0][0].name, "task");
    // The value "key=value1" is stored as "url"
    assert!(groups[0][0].payload.contains_key("url"));
}

/// Test task group parsing with task=url= format
#[test]
fn parse_task_groups_task_url_format() {
    let args = vec!["pageview=url=https://example.com".to_string()];
    let groups = parse_task_groups(&args);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0][0].name, "pageview");
    assert_eq!(
        groups[0][0].payload.get("url"),
        Some(&serde_json::Value::String(
            "https://example.com".to_string()
        ))
    );
}

/// Test browser filter with special characters
#[test]
fn parse_browser_filters_special_chars() {
    let input = Some("chrome-beta, firefox-esr");
    let filters = parse_browser_filters(input);

    assert_eq!(filters.len(), 2);
    assert!(filters.contains(&"chrome-beta".to_string()));
    assert!(filters.contains(&"firefox-esr".to_string()));
}

/// Test task group with numeric task name
#[test]
fn parse_task_groups_numeric_task_name() {
    let args = vec!["123".to_string()];
    let groups = parse_task_groups(&args);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0][0].name, "123");
}

/// Test that "then" is case-insensitive
#[test]
fn parse_task_groups_then_case_insensitive() {
    let args = vec!["task1".to_string(), "THEN".to_string(), "task2".to_string()];
    let groups = parse_task_groups(&args);

    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0][0].name, "task1");
    assert_eq!(groups[1][0].name, "task2");
}
