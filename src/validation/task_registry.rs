//! Task registry validation module.
//!
//! Provides validation for task names and file existence:
//! - Validates task names against known task types
//! - Checks if task files exist in the task directory
//! - Provides detailed error messages for invalid task names
//!
//! This module handles task name/presence validation, while `task.rs`
//! handles task payload validation.

use crate::error::{ConfigError, Result};
use crate::cli::TaskDefinition;
use log::warn;
use std::collections::HashSet;
use std::path::Path;

/// Result of task validation
#[derive(Debug, Clone)]
pub struct TaskValidationResult {
    pub task_name: String,
    pub is_known: bool,
    pub file_exists: bool,
    pub warnings: Vec<String>,
}

/// Check if a task name is known (registered in the orchestrator)
pub fn is_known_task(task_name: &str) -> bool {
    crate::task::is_known_task(task_name)
}

/// Check if a task file exists in the task directory
pub fn task_file_exists(task_name: &str) -> bool {
    let clean_name = crate::task::normalize_task_name(task_name);

    // Check for .rs file in src/task/ directory
    let task_path = Path::new("src/task").join(format!("{}.rs", clean_name));
    if task_path.exists() {
        return true;
    }

    // Also check for .js fallback (for compatibility)
    let js_path = Path::new("src/task").join(format!("{}.js", clean_name));
    js_path.exists()
}

/// Validate a task name and return validation result
pub fn validate_task(task_name: &str) -> TaskValidationResult {
    let clean_name = crate::task::normalize_task_name(task_name);
    let mut warnings = Vec::new();

    let is_known = is_known_task(clean_name);
    let file_exists = task_file_exists(clean_name);

    if !is_known {
        let known_tasks = crate::task::known_task_names().join(", ");
        warnings.push(format!(
            "Unknown task name '{}'. Known tasks: {}",
            clean_name, known_tasks
        ));
    }

    if !file_exists {
        warnings.push(format!(
            "Task file for '{}' not found in task/ directory",
            clean_name
        ));
    }

    TaskValidationResult {
        task_name: clean_name.to_string(),
        is_known,
        file_exists,
        warnings,
    }
}

/// Validate all tasks in a group and log warnings for unknown tasks
pub fn validate_task_groups(groups: &[Vec<TaskDefinition>]) -> Vec<TaskValidationResult> {
    let mut results = Vec::new();
    let mut seen_tasks: HashSet<String> = HashSet::new();

    for group in groups {
        for task in group {
            let normalized = crate::task::normalize_task_name(&task.name).to_string();
            if !seen_tasks.contains(&normalized) {
                let result = validate_task(&task.name);

                // Log warnings for unknown tasks
                for warning in &result.warnings {
                    warn!("{}", warning);
                }

                results.push(result);
                seen_tasks.insert(normalized);
            }
        }
    }

    results
}

/// Validate task groups and fail fast on any warning.
pub fn validate_task_groups_strict(groups: &[Vec<TaskDefinition>]) -> Result<()> {
    let results = validate_task_groups(groups);
    let mut errors = Vec::new();

    for result in results {
        if !result.warnings.is_empty() {
            errors.extend(result.warnings);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ConfigError::ValidationFailed(errors.join(" | ")).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_is_known_task_valid() {
        assert!(is_known_task("cookiebot"));
        assert!(is_known_task("pageview"));
        assert!(is_known_task("twitteractivity"));
        assert!(is_known_task("cookiebot.js"));
    }

    #[test]
    fn test_is_known_task_invalid() {
        assert!(!is_known_task("unknown_task"));
        assert!(!is_known_task("nonexistent"));
    }

    #[test]
    fn test_is_known_task_normalizes_js_suffix() {
        assert!(is_known_task("cookiebot.js"));
        assert!(is_known_task(crate::task::normalize_task_name(
            "pageview.js"
        )));
    }

    #[test]
    fn test_validate_task_known_task() {
        let result = validate_task("cookiebot");
        assert!(result.is_known);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_validate_task_unknown_task() {
        let result = validate_task("unknown_task");
        assert!(!result.is_known);
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].contains("Unknown task name"));
    }

    #[test]
    fn test_validate_task_groups_strict_known_tasks() {
        let groups = vec![vec![TaskDefinition {
            name: "cookiebot".to_string(),
            payload: HashMap::new(),
        }]];

        assert!(validate_task_groups_strict(&groups).is_ok());
    }

    #[test]
    fn test_validate_task_groups_strict_unknown_task() {
        let groups = vec![vec![TaskDefinition {
            name: "unknown_task".to_string(),
            payload: HashMap::new(),
        }]];

        assert!(validate_task_groups_strict(&groups).is_err());
    }

    #[test]
    fn test_validate_task_groups_logs_warnings() {
        let groups = vec![vec![
            TaskDefinition {
                name: "cookiebot".to_string(),
                payload: HashMap::new(),
            },
            TaskDefinition {
                name: "unknown_task".to_string(),
                payload: HashMap::new(),
            },
        ]];

        let results = validate_task_groups(&groups);

        // Should have 2 results (cookiebot and unknown_task)
        assert_eq!(results.len(), 2);

        // cookiebot should have no warnings
        let cookiebot_result = results.iter().find(|r| r.task_name == "cookiebot").unwrap();
        assert!(cookiebot_result.is_known);
        assert!(cookiebot_result.warnings.is_empty());

        // unknown_task should have warnings
        let unknown_result = results
            .iter()
            .find(|r| r.task_name == "unknown_task")
            .unwrap();
        assert!(!unknown_result.is_known);
        assert!(!unknown_result.warnings.is_empty());
    }
}
