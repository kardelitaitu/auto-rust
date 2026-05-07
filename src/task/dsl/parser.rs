//! DSL parser module.
//!
//! Provides functions for parsing and validating DSL task files.
//! - `parse_task_file` - Parse a YAML task file into a TaskDefinition
//! - `validate_task_definition` - Validate a TaskDefinition for correctness

use crate::task::dsl::{Action, TaskDefinition};
use serde_yaml;
use std::path::Path;
use toml;

/// Parse a DSL task file into a TaskDefinition.
///
/// # Arguments
/// * `path` - Path to the YAML task file
///
/// # Returns
/// * `Ok(TaskDefinition)` if parsing succeeds
/// * `Err(String)` with error message if parsing fails
pub fn parse_task_file<P: AsRef<Path>>(path: P) -> Result<TaskDefinition, String> {
    let path = path.as_ref();

    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Try YAML first
    match serde_yaml::from_str::<TaskDefinition>(&content) {
        Ok(task) => Ok(task),
        Err(yaml_err) => {
            // YAML failed, try TOML
            match toml::from_str::<TaskDefinition>(&content) {
                Ok(task) => Ok(task),
                Err(toml_err) => {
                    // Both failed, return YAML error as primary
                    Err(format!(
                        "Failed to parse YAML: {}, TOML error: {}",
                        yaml_err, toml_err
                    ))
                }
            }
        }
    }
}

/// Get a task definition by name.
///
/// Looks up a task in the registry and returns its definition if found.
/// This is a convenience wrapper around the registry.
///
/// # Arguments
/// * `name` - Task name to look up
///
/// # Returns
/// * `Some(TaskDefinition)` if found (cloned)
/// * `None` if not found
pub fn get_task_definition(name: &str) -> Option<TaskDefinition> {
    let registry = crate::task::registry::TaskRegistry::with_built_in_tasks();
    registry.get_task_definition(name).cloned()
}

/// Validate a TaskDefinition for correctness.
///
/// Checks:
/// - Task name is not empty
/// - Actions are valid (if any)
///
/// # Arguments
/// * `task_def` - The TaskDefinition to validate
///
/// # Returns
/// * `Ok(())` if validation passes
/// * `Err(Vec<String>)` with error messages if validation fails
pub fn validate_task_definition(task_def: &TaskDefinition) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    // Validate task name
    if task_def.name.is_empty() {
        errors.push("Task name cannot be empty".to_string());
    }

    // Validate actions
    if task_def.actions.is_empty() {
        errors.push("Task must have at least one action".to_string());
    }

    // Validate actions (recursively check nested actions)
    fn validate_action(action: &Action, errors: &mut Vec<String>) {
        match action {
            Action::If {
                condition: _,
                then,
                r#else,
            } => {
                if then.is_empty() {
                    errors.push("'if' block has empty 'then' branch".to_string());
                }
                if let Some(else_actions) = r#else {
                    if else_actions.is_empty() {
                        errors.push("'if' block has empty 'else' branch".to_string());
                    }
                    // Recursively validate nested actions
                    for sub_action in else_actions {
                        validate_action(sub_action, errors);
                    }
                }
                // Recursively validate nested actions
                for sub_action in then {
                    validate_action(sub_action, errors);
                }
            }
            Action::Loop {
                count,
                condition,
                actions,
            } => {
                if count.is_none() && condition.is_none() {
                    errors.push("'loop' must have 'count' or 'condition'".to_string());
                }
                if actions.is_empty() {
                    errors.push("'loop' block has no actions".to_string());
                }
                for sub_action in actions {
                    validate_action(sub_action, errors);
                }
            }
            Action::While {
                condition: _,
                actions,
                max_iterations: _,
            } => {
                for sub_action in actions {
                    validate_action(sub_action, errors);
                }
            }
            Action::Foreach {
                variable: _,
                collection: _,
                actions,
                ..
            } => {
                for sub_action in actions {
                    validate_action(sub_action, errors);
                }
            }
            Action::Retry { actions, .. } => {
                for sub_action in actions {
                    validate_action(sub_action, errors);
                }
            }
            Action::Parallel { actions, .. } => {
                for sub_action in actions {
                    validate_action(sub_action, errors);
                }
            }
            Action::Try {
                try_actions,
                catch_actions,
                finally_actions,
                ..
            } => {
                for sub_action in try_actions {
                    validate_action(sub_action, errors);
                }
                if let Some(catch) = catch_actions {
                    for sub_action in catch {
                        validate_action(sub_action, errors);
                    }
                }
                if let Some(finally) = finally_actions {
                    for sub_action in finally {
                        validate_action(sub_action, errors);
                    }
                }
            }
            _ => {
                // Other actions don't need special validation
            }
        }
    }

    for action in &task_def.actions {
        validate_action(action, &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Parse a YAML string into a TaskDefinition.
///
/// # Arguments
/// * `yaml` - YAML string to parse
///
/// # Returns
/// * `Ok(TaskDefinition)` if parsing succeeds
/// * `Err(String)` with error message if parsing fails
pub fn parse_task_yaml(yaml: &str) -> Result<TaskDefinition, String> {
    serde_yaml::from_str(yaml).map_err(|e| format!("Failed to parse YAML: {}", e))
}

/// Parse a TOML string into a TaskDefinition.
///
/// # Arguments
/// * `toml_str` - TOML string to parse
///
/// # Returns
/// * `Ok(TaskDefinition)` if parsing succeeds
/// * `Err(String)` with error message if parsing fails
pub fn parse_task_toml(toml_str: &str) -> Result<TaskDefinition, String> {
    toml::from_str(toml_str).map_err(|e| format!("Failed to parse TOML: {}", e))
}

/// Format a TaskDefinition into a human-readable string.
///
/// # Arguments
/// * `task_def` - The TaskDefinition to format
///
/// # Returns
/// * Formatted string with task details
pub fn format_task_definition(task_def: &TaskDefinition) -> String {
    let mut output = String::new();
    output.push_str(&format!("Task: {}\n", task_def.name));
    output.push_str(&format!("Description: {}\n", task_def.description));
    output.push_str(&format!("Policy: {}\n", task_def.policy));
    output.push_str(&format!("Parameters: {}\n", task_def.parameters.len()));
    output.push_str(&format!("Actions: {}\n", task_def.actions.len()));

    for (idx, action) in task_def.actions.iter().enumerate() {
        let formatted = format!("{:?}", action);
        let action_type = formatted.split('(').next().unwrap_or("Unknown");
        output.push_str(&format!("  {}: {}\n", idx + 1, action_type));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::dsl::Action;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_valid_task_file() {
        let dir = TempDir::new().unwrap();
        let task_file = dir.path().join("test.task");

        let content = r#"
name: test_task
description: "A test task"
policy: default
actions:
  - action: wait
    duration_ms: 100
"#;

        fs::write(&task_file, content).unwrap();

        let result = parse_task_file(&task_file);
        assert!(result.is_ok());

        let task_def = result.unwrap();
        assert_eq!(task_def.name, "test_task");
        assert_eq!(task_def.actions.len(), 1);
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let dir = TempDir::new().unwrap();
        let task_file = dir.path().join("invalid.task");

        fs::write(&task_file, "invalid: yaml: content: [").unwrap();

        let result = parse_task_file(&task_file);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Failed to parse YAML"));
    }

    #[test]
    fn test_parse_nonexistent_file() {
        let result = parse_task_file("/nonexistent/path.task");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Failed to read file"));
    }

    #[test]
    fn test_validate_valid_task() {
        let task_def = TaskDefinition {
            name: "valid_task".to_string(),
            description: "Valid".to_string(),
            policy: "default".to_string(),
            parameters: std::collections::HashMap::new(),
            include: vec![],
            actions: vec![Action::Wait { duration_ms: 100 }],
        };

        let result = validate_task_definition(&task_def);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_empty_name() {
        let task_def = TaskDefinition {
            name: "".to_string(),
            description: "Invalid".to_string(),
            policy: "default".to_string(),
            parameters: std::collections::HashMap::new(),
            include: vec![],
            actions: vec![],
        };

        let result = validate_task_definition(&task_def);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        assert!(errors[0].contains("cannot be empty"));
    }
}
