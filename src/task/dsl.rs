//! Task DSL (Domain Specific Language) parser for external task definitions.
//!
//! This module provides parsing for `.task` files that define tasks
//! declaratively rather than as Rust code. Supports YAML and TOML formats.
//!
//! # Example task file (YAML):
//! ```yaml
//! name: custom_greeting
//! description: "A simple greeting task"
//! policy: default
//!
//! actions:
//!   - navigate:
//!       url: "https://example.com"
//!   - wait:
//!       duration_ms: 1000
//!   - click:
//!       selector: "#greeting-button"
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A task definition loaded from a DSL file.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TaskDefinition {
    /// Task name (must be unique)
    pub name: String,
    /// Human-readable description
    #[serde(default)]
    pub description: String,
    /// Policy name for timeout/permission configuration
    #[serde(default = "default_policy")]
    pub policy: String,
    /// Task parameters/inputs
    #[serde(default)]
    pub parameters: HashMap<String, ParameterDef>,
    /// Sequence of actions to execute
    #[serde(default)]
    pub actions: Vec<Action>,
}

fn default_policy() -> String {
    "default".to_string()
}

/// Parameter definition for task inputs.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ParameterDef {
    /// Parameter type
    #[serde(default)]
    pub r#type: ParameterType,
    /// Human-readable description
    #[serde(default)]
    pub description: String,
    /// Default value (optional)
    pub default: Option<serde_yaml::Value>,
    /// Whether parameter is required
    #[serde(default)]
    pub required: bool,
}

/// Parameter types supported by the DSL.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ParameterType {
    #[default]
    String,
    Integer,
    Boolean,
    Url,
    Selector,
}

/// A single action/step in a task.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    /// Navigate to a URL
    Navigate { url: String },
    /// Click an element
    Click { selector: String },
    /// Type text into an element
    Type { selector: String, text: String },
    /// Wait for a duration
    Wait { duration_ms: u64 },
    /// Wait for an element to be visible
    WaitFor {
        selector: String,
        timeout_ms: Option<u64>,
    },
    /// Scroll to an element
    ScrollTo { selector: String },
    /// Extract text from an element
    Extract {
        selector: String,
        variable: Option<String>,
    },
    /// Execute JavaScript
    Execute { script: String },
    /// Conditional action
    If {
        condition: Condition,
        then: Vec<Action>,
        r#else: Option<Vec<Action>>,
    },
    /// Loop over actions
    Loop {
        count: Option<u32>,
        condition: Option<Condition>,
        actions: Vec<Action>,
    },
    /// Call another task
    Call {
        task: String,
        parameters: Option<HashMap<String, serde_yaml::Value>>,
    },
    /// Log a message
    Log {
        message: String,
        level: Option<LogLevel>,
    },
}

/// Log levels for the Log action.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    #[default]
    Info,
    Debug,
    Warn,
    Error,
}

/// Condition for conditional/loop actions.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    /// Check if element exists
    ElementExists { selector: String },
    /// Check if element is visible
    ElementVisible { selector: String },
    /// Check text content equals value
    TextEquals { selector: String, value: String },
    /// Check if variable equals value
    VariableEquals {
        name: String,
        value: serde_yaml::Value,
    },
    /// Logical AND of multiple conditions
    And { conditions: Vec<Condition> },
    /// Logical OR of multiple conditions
    Or { conditions: Vec<Condition> },
    /// Negate a condition
    Not { condition: Box<Condition> },
}

/// Parse a task definition from a file path.
///
/// Automatically detects format based on file extension:
/// - `.yaml` or `.yml` → YAML
/// - `.toml` → TOML
///
/// # Arguments
/// * `path` - Path to the task definition file
///
/// # Returns
/// Parsed TaskDefinition on success
///
/// # Errors
/// Returns error if file not found, unreadable, or invalid format
pub fn parse_task_file(path: &Path) -> Result<TaskDefinition> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read task file: {}", path.display()))?;

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "yaml" | "yml" => parse_task_yaml(&content),
        "toml" => parse_task_toml(&content),
        _ => {
            // Try YAML first, then TOML
            parse_task_yaml(&content)
                .or_else(|_| parse_task_toml(&content))
                .with_context(|| {
                    format!(
                        "Failed to parse task file as YAML or TOML: {}",
                        path.display()
                    )
                })
        }
    }
}

/// Parse a task definition from YAML content.
pub fn parse_task_yaml(content: &str) -> Result<TaskDefinition> {
    serde_yaml::from_str(content).context("Failed to parse task YAML")
}

/// Parse a task definition from TOML content.
pub fn parse_task_toml(content: &str) -> Result<TaskDefinition> {
    toml::from_str(content).context("Failed to parse task TOML")
}

/// Validate a task definition.
///
/// Checks:
/// - Name is not empty
/// - At least one action is defined
/// - All action references are valid (basic checks)
pub fn validate_task_definition(def: &TaskDefinition) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if def.name.is_empty() {
        errors.push("Task name cannot be empty".to_string());
    }

    if def.name.contains(' ') {
        errors.push("Task name cannot contain spaces".to_string());
    }

    if def.actions.is_empty() {
        errors.push("Task must have at least one action".to_string());
    }

    // Validate action nesting (check for empty blocks)
    for (idx, action) in def.actions.iter().enumerate() {
        validate_action(action, idx, &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_action(action: &Action, index: usize, errors: &mut Vec<String>) {
    match action {
        Action::If { then, r#else, .. } => {
            if then.is_empty() {
                errors.push(format!(
                    "Action {}: 'if' block has empty 'then' branch",
                    index
                ));
            }
            if let Some(else_branch) = r#else {
                if else_branch.is_empty() {
                    errors.push(format!(
                        "Action {}: 'if' block has empty 'else' branch",
                        index
                    ));
                }
            }
            for (i, a) in then.iter().enumerate() {
                validate_action(a, index * 100 + i, errors);
            }
        }
        Action::Loop {
            count,
            condition,
            actions,
            ..
        } => {
            if count.is_none() && condition.is_none() {
                errors.push(format!(
                    "Action {}: 'loop' must have 'count' or 'condition'",
                    index
                ));
            }
            if actions.is_empty() {
                errors.push(format!("Action {}: 'loop' block has no actions", index));
            }
        }
        _ => {}
    }
}

/// Format a task definition for display.
pub fn format_task_definition(def: &TaskDefinition) -> String {
    let mut output = String::new();
    output.push_str(&format!("Task: {}\n", def.name));
    output.push_str(&format!("Policy: {}\n", def.policy));
    if !def.description.is_empty() {
        output.push_str(&format!("Description: {}\n", def.description));
    }
    if !def.parameters.is_empty() {
        output.push_str(&format!("Parameters: {}\n", def.parameters.len()));
    }
    output.push_str(&format!("Actions: {}\n", def.actions.len()));
    for (idx, action) in def.actions.iter().enumerate() {
        output.push_str(&format!("  {}. {:?}\n", idx + 1, action));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_simple_yaml_task() {
        let yaml = r#"
name: simple_task
description: "A simple test task"
policy: default
actions:
  - action: navigate
    url: "https://example.com"
  - action: wait
    duration_ms: 1000
"#;

        let def = parse_task_yaml(yaml).unwrap();
        assert_eq!(def.name, "simple_task");
        assert_eq!(def.description, "A simple test task");
        assert_eq!(def.policy, "default");
        assert_eq!(def.actions.len(), 2);
    }

    #[test]
    fn test_parse_task_with_parameters() {
        let yaml = r#"
name: parameterized_task
description: "Task with parameters"
parameters:
  target_url:
    type: url
    description: "URL to navigate to"
    required: true
  wait_time:
    type: integer
    description: "Wait duration in ms"
    default: 1000
actions:
  - action: navigate
    url: "{{target_url}}"
"#;

        let def = parse_task_yaml(yaml).unwrap();
        assert_eq!(def.parameters.len(), 2);
        assert!(def.parameters.contains_key("target_url"));
        assert!(def.parameters.contains_key("wait_time"));
    }

    #[test]
    fn test_parse_toml_task() {
        let toml = r##"
name = "toml_task"
description = "A TOML task"
policy = "default"

[[actions]]
action = "navigate"
url = "https://example.com"

[[actions]]
action = "click"
selector = "#button"
"##;

        let def = parse_task_toml(toml).unwrap();
        assert_eq!(def.name, "toml_task");
        assert_eq!(def.actions.len(), 2);
    }

    #[test]
    fn test_validate_valid_task() {
        let def = TaskDefinition {
            name: "valid_task".to_string(),
            description: "A valid task".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            actions: vec![Action::Wait { duration_ms: 1000 }],
        };

        assert!(validate_task_definition(&def).is_ok());
    }

    #[test]
    fn test_validate_empty_name() {
        let def = TaskDefinition {
            name: "".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            actions: vec![Action::Wait { duration_ms: 1000 }],
        };

        let result = validate_task_definition(&def);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("name cannot be empty")));
    }

    #[test]
    fn test_validate_no_actions() {
        let def = TaskDefinition {
            name: "no_actions".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            actions: vec![],
        };

        let result = validate_task_definition(&def);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("at least one action")));
    }

    #[test]
    fn test_validate_name_with_spaces() {
        let def = TaskDefinition {
            name: "invalid name".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            actions: vec![Action::Wait { duration_ms: 1000 }],
        };

        let result = validate_task_definition(&def);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("cannot contain spaces")));
    }

    #[test]
    fn test_parse_click_action() {
        let yaml = r##"
name: click_task
actions:
  - action: click
    selector: "#submit-button"
"##;

        let def = parse_task_yaml(yaml).unwrap();
        match &def.actions[0] {
            Action::Click { selector } => {
                assert_eq!(selector, "#submit-button");
            }
            _ => panic!("Expected Click action"),
        }
    }

    #[test]
    fn test_parse_type_action() {
        let yaml = r##"
name: type_task
actions:
  - action: type
    selector: "#username"
    text: "john_doe"
"##;

        let def = parse_task_yaml(yaml).unwrap();
        match &def.actions[0] {
            Action::Type { selector, text } => {
                assert_eq!(selector, "#username");
                assert_eq!(text, "john_doe");
            }
            _ => panic!("Expected Type action"),
        }
    }

    #[test]
    fn test_default_policy() {
        let yaml = r#"
name: no_policy_task
actions:
  - action: wait
    duration_ms: 100
"#;

        let def = parse_task_yaml(yaml).unwrap();
        assert_eq!(def.policy, "default");
    }

    #[test]
    fn test_parse_task_file_missing_file_reports_read_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("missing.task");

        let err = parse_task_file(&path).unwrap_err();
        let msg = err.to_string();

        assert!(msg.contains("Failed to read task file"));
        assert!(msg.contains("missing.task"));
    }

    #[test]
    fn test_parse_task_file_invalid_yaml_reports_parse_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("broken.yaml");
        fs::write(&path, "name: broken_task\nactions:\n  - action: wait\n    duration_ms: not-a-number\n")
            .unwrap();

        let err = parse_task_file(&path).unwrap_err();
        let msg = err.to_string();

        assert!(msg.contains("Failed to parse task YAML"));
    }

    #[test]
    fn test_parse_task_file_invalid_toml_reports_parse_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("broken.toml");
        fs::write(&path, "name = \"broken_task\"\n[[actions]]\naction = \"wait\"\nduration_ms = \"oops\"\n")
            .unwrap();

        let err = parse_task_file(&path).unwrap_err();
        let msg = err.to_string();

        assert!(msg.contains("Failed to parse task TOML"));
    }

    #[test]
    fn test_parse_task_file_invalid_unknown_extension_reports_fallback_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("broken.task");
        fs::write(&path, "not valid yaml or toml").unwrap();

        let err = parse_task_file(&path).unwrap_err();
        let msg = err.to_string();

        assert!(msg.contains("Failed to parse task file as YAML or TOML"));
        assert!(msg.contains("broken.task"));
    }
}
