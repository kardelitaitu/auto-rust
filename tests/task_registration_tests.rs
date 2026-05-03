//! Integration tests for task registration and validation.
//! Tests task discovery, validation, and registry functionality.

use auto::task::{is_known_task, known_task_names, normalize_task_name, TASK_NAMES};
use auto::validation::{
    get_task_descriptor, is_known_task as validate_is_known_task, validate_task_groups,
    validate_task_groups_strict,
};

/// Test that task names list is not empty
#[test]
fn task_names_list_not_empty() {
    assert!(!TASK_NAMES.is_empty());
}

/// Test that all task names are lowercase and valid
#[test]
fn task_names_are_valid() {
    for name in TASK_NAMES.iter() {
        // Should not be empty
        assert!(!name.is_empty(), "Task name cannot be empty");

        // Should not contain spaces
        assert!(!name.contains(' '), "Task name '{}' contains spaces", name);

        // Should be lowercase
        assert_eq!(
            *name,
            name.to_lowercase(),
            "Task name '{}' is not lowercase",
            name
        );
    }
}

/// Test normalize_task_name with .js suffix
#[test]
fn normalize_task_name_strips_js_suffix() {
    assert_eq!(normalize_task_name("cookiebot.js"), "cookiebot");
    assert_eq!(normalize_task_name("pageview.js"), "pageview");
}

/// Test normalize_task_name without suffix
#[test]
fn normalize_task_name_no_suffix() {
    assert_eq!(normalize_task_name("cookiebot"), "cookiebot");
    assert_eq!(normalize_task_name("pageview"), "pageview");
}

/// Test normalize_task_name with empty string
#[test]
fn normalize_task_name_empty() {
    assert_eq!(normalize_task_name(""), "");
}

/// Test normalize_task_name with multiple .js suffixes
#[test]
fn normalize_task_name_multiple_suffixes() {
    assert_eq!(normalize_task_name("task.js.js"), "task.js");
}

/// Test is_known_task with valid task names
#[test]
fn is_known_task_valid_tasks() {
    assert!(is_known_task("cookiebot"));
    assert!(is_known_task("pageview"));
    assert!(is_known_task("twitteractivity"));
}

/// Test is_known_task with invalid task names
#[test]
fn is_known_task_invalid_tasks() {
    assert!(!is_known_task("unknown_task"));
    assert!(!is_known_task("fake"));
    assert!(!is_known_task(""));
}

/// Test is_known_task with .js suffix (should normalize)
#[test]
fn is_known_task_with_js_suffix() {
    assert!(is_known_task("cookiebot.js"));
    assert!(is_known_task("pageview.js"));
}

/// Test known_task_names returns expected tasks
#[test]
fn known_task_names_contains_expected() {
    let names = known_task_names();

    assert!(names.contains(&"cookiebot"));
    assert!(names.contains(&"pageview"));
    assert!(names.contains(&"twitteractivity"));
}

/// Test validation layer is_known_task matches task layer
#[test]
fn validation_is_known_task_matches_task_layer() {
    for name in ["cookiebot", "pageview", "twitteractivity", "unknown"] {
        assert_eq!(
            validate_is_known_task(name),
            is_known_task(name),
            "Mismatch for task '{}' between validation and task layers",
            name
        );
    }
}

/// Test task descriptor lookup for known tasks
#[test]
fn task_descriptor_lookup_known_tasks() {
    let cookiebot = get_task_descriptor("cookiebot").unwrap();
    assert_eq!(cookiebot.name, "cookiebot");
    assert!(cookiebot.source.is_built_in());

    let pageview = get_task_descriptor("pageview").unwrap();
    assert_eq!(pageview.name, "pageview");
    assert!(pageview.source.is_built_in());
}

/// Test task descriptor lookup for unknown tasks
#[test]
fn task_descriptor_lookup_unknown_tasks() {
    assert!(get_task_descriptor("unknown_task").is_err());
    assert!(get_task_descriptor("fake").is_err());
}

/// Test validate_task_groups with valid groups
#[test]
fn validate_task_groups_valid() {
    use auto::cli::TaskDefinition;

    let groups = vec![vec![TaskDefinition {
        name: "cookiebot".to_string(),
        payload: std::collections::HashMap::new(),
    }]];

    let results = validate_task_groups(&groups);
    assert_eq!(results.len(), 1);
    assert!(results[0].is_known);
    assert!(results[0].source.contains("BuiltInRust"));
    assert_eq!(results[0].policy_name, "cookiebot");
}

/// Test validate_task_groups with invalid task
#[test]
fn validate_task_groups_invalid_task() {
    use auto::cli::TaskDefinition;

    let groups = vec![vec![TaskDefinition {
        name: "unknown_task".to_string(),
        payload: std::collections::HashMap::new(),
    }]];

    let results = validate_task_groups(&groups);
    assert_eq!(results.len(), 1);
    // Unknown task should not be known
    assert!(!results[0].is_known);
    assert!(results[0].source.contains("Unknown"));
}

/// Test validate_task_groups_strict with valid groups
#[test]
fn validate_task_groups_strict_valid() {
    use auto::cli::TaskDefinition;

    let groups = vec![vec![
        TaskDefinition {
            name: "cookiebot".to_string(),
            payload: std::collections::HashMap::new(),
        },
        TaskDefinition {
            name: "pageview".to_string(),
            payload: std::collections::HashMap::new(),
        },
    ]];

    let result = validate_task_groups_strict(&groups);
    assert!(result.is_ok());
}

/// Test validate_task_groups_strict with invalid task
#[test]
fn validate_task_groups_strict_invalid() {
    use auto::cli::TaskDefinition;

    let groups = vec![vec![TaskDefinition {
        name: "unknown_task".to_string(),
        payload: std::collections::HashMap::new(),
    }]];

    let result = validate_task_groups_strict(&groups);
    assert!(result.is_err());
}

/// Test validate_task_groups_strict with empty groups
#[test]
fn validate_task_groups_strict_empty_groups() {
    let groups: Vec<Vec<auto::cli::TaskDefinition>> = vec![];

    let result = validate_task_groups_strict(&groups);
    assert!(result.is_ok());
}

/// Test that task count is consistent
#[test]
fn task_names_count_consistent() {
    assert_eq!(TASK_NAMES.len(), known_task_names().len());
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Test normalize_task_name with special characters
#[test]
fn normalize_task_name_special_chars() {
    // Special characters should be preserved (not stripped)
    assert_eq!(normalize_task_name("task-name"), "task-name");
    assert_eq!(normalize_task_name("task_name"), "task_name");
}

/// Test normalize_task_name with multiple dots
#[test]
fn normalize_task_name_multiple_dots() {
    assert_eq!(normalize_task_name("task..js"), "task.");
    assert_eq!(normalize_task_name(".js"), "");
}

/// Test is_known_task with whitespace
#[test]
fn is_known_task_with_whitespace() {
    // Task names with whitespace should not be known
    assert!(!is_known_task("cookiebot "));
    assert!(!is_known_task(" cookiebot"));
    assert!(!is_known_task("cookie bot"));
}

/// Test validate_task_groups with empty task definition
#[test]
fn validate_task_groups_empty_task() {
    let groups = vec![vec![auto::cli::TaskDefinition {
        name: "".to_string(),
        payload: std::collections::HashMap::new(),
    }]];

    let results = validate_task_groups(&groups);
    assert!(!results.is_empty());
    assert!(!results[0].is_known);
}

/// Test registry-based task validation
#[test]
fn registry_based_task_validation() {
    use auto::task::registry::TaskRegistry;

    let registry = TaskRegistry::with_built_in_tasks();

    // Known tasks should be in registry
    assert!(registry.is_known("cookiebot"));
    assert!(registry.is_known("pageview"));
    assert_eq!(registry.task_count(), 15);

    // Unknown tasks should not be in registry
    assert!(!registry.is_known("unknown_task"));
}
