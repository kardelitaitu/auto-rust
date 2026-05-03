//! Task registry validation module.
//!
//! Provides validation for task names using the unified task registry.
//! - Validates task names against the registry
//! - Provides detailed error messages for invalid task names
//! - Uses registry as single source of truth
//!
//! This module handles task name validation, while `task.rs`
//! handles task payload validation.

use crate::cli::TaskDefinition;
use crate::error::{ConfigError, Result};
use crate::task::registry::{RegistryError, TaskRegistry};
use log::warn;
use std::collections::HashSet;

/// Result of task validation
#[derive(Debug, Clone)]
pub struct TaskValidationResult {
    pub task_name: String,
    pub is_known: bool,
    pub source: String,
    pub policy_name: String,
    pub warnings: Vec<String>,
}

/// Check if a task name is known (exists in registry)
pub fn is_known_task(task_name: &str) -> bool {
    let registry = TaskRegistry::with_built_in_tasks();
    registry.is_known(task_name)
}

/// Get task descriptor from registry
pub fn get_task_descriptor(
    task_name: &str,
) -> std::result::Result<crate::task::registry::TaskDescriptor, RegistryError> {
    let registry = TaskRegistry::with_built_in_tasks();
    registry.lookup(task_name)
}

/// Validate a task name and return validation result
pub fn validate_task(task_name: &str) -> TaskValidationResult {
    let registry = TaskRegistry::with_built_in_tasks();
    validate_task_with_registry(task_name, &registry)
}

fn validate_task_with_registry(task_name: &str, registry: &TaskRegistry) -> TaskValidationResult {
    let clean_name = crate::task::normalize_task_name(task_name);
    let mut warnings = Vec::new();

    match registry.lookup(clean_name) {
        Ok(descriptor) => TaskValidationResult {
            task_name: clean_name.to_string(),
            is_known: true,
            source: format!("{:?}", descriptor.source),
            policy_name: descriptor.policy_name.to_string(),
            warnings,
        },
        Err(RegistryError::UnknownTask { .. }) => {
            let known_tasks = registry.task_names().join(", ");
            warnings.push(format!(
                "Unknown task name '{}'. Known tasks: {}",
                clean_name, known_tasks
            ));

            TaskValidationResult {
                task_name: clean_name.to_string(),
                is_known: false,
                source: "Unknown".to_string(),
                policy_name: "default".to_string(),
                warnings,
            }
        }
        Err(RegistryError::Conflict { name, sources }) => {
            warnings.push(format_conflict_warning(&name, &sources));

            TaskValidationResult {
                task_name: clean_name.to_string(),
                is_known: false,
                source: "Conflict".to_string(),
                policy_name: "default".to_string(),
                warnings,
            }
        }
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

fn format_conflict_warning(name: &str, sources: &[crate::task::registry::TaskSource]) -> String {
    format!("Task '{}' exists in multiple sources: {:?}", name, sources)
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
        assert!(result.source.contains("BuiltInRust"));
        assert_eq!(result.policy_name, "cookiebot");
    }

    #[test]
    fn test_validate_task_unknown_task() {
        let result = validate_task("unknown_task");
        assert!(!result.is_known);
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].contains("Unknown task name"));
        assert!(result.source.contains("Unknown"));
    }

    #[test]
    fn test_validate_task_js_suffix_known_task() {
        let result = validate_task("cookiebot.js");
        assert!(result.is_known);
        assert!(result.warnings.is_empty());
        assert_eq!(result.task_name, "cookiebot");
        assert!(result.source.contains("BuiltInRust"));
        assert_eq!(result.policy_name, "cookiebot");
    }

    #[test]
    fn test_validate_task_js_suffix_unknown_task() {
        let result = validate_task("unknown_task.js");
        assert!(!result.is_known);
        assert_eq!(result.task_name, "unknown_task");
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].contains("Unknown task name 'unknown_task'"));
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
        assert!(cookiebot_result.source.contains("BuiltInRust"));
        assert_eq!(cookiebot_result.policy_name, "cookiebot");

        // unknown_task should have warnings
        let unknown_result = results
            .iter()
            .find(|r| r.task_name == "unknown_task")
            .unwrap();
        assert!(!unknown_result.is_known);
        assert!(!unknown_result.warnings.is_empty());
        assert!(unknown_result.source.contains("Unknown"));
    }

    #[test]
    fn test_get_task_descriptor_known() {
        let result = get_task_descriptor("cookiebot");
        assert!(result.is_ok());
        let descriptor = result.unwrap();
        assert_eq!(descriptor.name, "cookiebot");
        assert!(descriptor.source.is_built_in());
    }

    #[test]
    fn test_get_task_descriptor_unknown() {
        let result = get_task_descriptor("nonexistent_task");
        assert!(result.is_err());
        assert!(
            matches!(result, Err(RegistryError::UnknownTask { name }) if name == "nonexistent_task")
        );
    }

    #[test]
    fn test_format_conflict_warning() {
        let warning = format_conflict_warning(
            "cookiebot",
            &[
                crate::task::registry::TaskSource::BuiltInRust,
                crate::task::registry::TaskSource::ConfiguredPath(std::path::PathBuf::from(
                    r"C:\external\cookiebot.task",
                )),
            ],
        );

        assert!(warning.contains("Task 'cookiebot' exists in multiple sources"));
        assert!(warning.contains("BuiltInRust"));
        assert!(warning.contains("ConfiguredPath"));
    }
}
