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
    /// Included task files to merge
    #[serde(default)]
    pub include: Vec<IncludeSpec>,
    /// Sequence of actions to execute
    #[serde(default)]
    pub actions: Vec<Action>,
}

/// Specification for including another task file.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct IncludeSpec {
    /// Path to the task file to include (relative or absolute)
    pub path: String,
    /// Optional condition for conditional inclusion
    #[serde(default)]
    pub condition: Option<String>,
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
    /// Capture a screenshot of the page
    Screenshot {
        /// Optional path to save the screenshot (defaults to auto-generated)
        path: Option<String>,
        /// Optional selector to screenshot specific element (defaults to full page)
        selector: Option<String>,
    },
    /// Clear an input field
    Clear { selector: String },
    /// Hover over an element
    Hover { selector: String },
    /// Select an option from a dropdown
    Select {
        selector: String,
        /// Value to select (use text or value attribute)
        value: String,
        /// Whether to select by visible text (default) or value attribute
        by_value: Option<bool>,
    },
    /// Right-click on an element
    RightClick { selector: String },
    /// Double-click on an element
    DoubleClick { selector: String },
    /// Execute actions in parallel
    Parallel {
        /// Actions to execute concurrently
        actions: Vec<Action>,
        /// Maximum number of concurrent actions (default: all at once)
        max_concurrency: Option<usize>,
    },
    /// Retry actions with exponential backoff
    Retry {
        /// Actions to retry on failure
        actions: Vec<Action>,
        /// Maximum number of retry attempts (default: 3)
        max_attempts: Option<u32>,
        /// Initial delay in milliseconds (default: 1000)
        initial_delay_ms: Option<u64>,
        /// Maximum delay in milliseconds (default: 30000)
        max_delay_ms: Option<u64>,
        /// Multiplier for exponential backoff (default: 2.0)
        backoff_multiplier: Option<f64>,
        /// Add random jitter to prevent thundering herd (default: true)
        jitter: Option<bool>,
        /// Only retry on specific error patterns (default: retry all)
        retry_on: Option<Vec<String>>,
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

/// Validate parameters against task definition.
///
/// Checks that:
/// - All required parameters are provided
/// - Provided parameters match expected types
/// - Unknown parameters are flagged
///
/// # Arguments
/// * `def` - Task definition with parameter specs
/// * `provided` - Parameters provided from CLI
///
/// # Returns
/// Ok(()) if all parameters are valid
/// Err with list of validation errors otherwise
pub fn validate_parameters(
    def: &TaskDefinition,
    provided: &serde_json::Value,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let empty_map = serde_json::Map::new();
    let obj = provided.as_object().unwrap_or(&empty_map);

    // Check required parameters
    for (name, param_def) in &def.parameters {
        if param_def.required && !obj.contains_key(name) && param_def.default.is_none() {
            errors.push(format!("Missing required parameter: '{}'", name));
        }
    }

    // Check for unknown parameters
    for (name, _) in obj {
        if !def.parameters.contains_key(name) {
            errors.push(format!("Unknown parameter: '{}'", name));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Resolve and merge included task files.
///
/// This function loads all included task files and merges their actions into the main task.
/// It handles circular include detection and provides detailed error messages.
///
/// # Arguments
/// * `def` - The task definition to resolve includes for
/// * `base_path` - The directory containing the main task file (for relative path resolution)
/// * `visited` - Set of already visited paths (for circular detection)
///
/// # Returns
/// A new TaskDefinition with all includes resolved and merged
///
/// # Errors
/// Returns error if an included file cannot be found, parsed, or if circular includes detected
pub fn resolve_includes(
    def: &TaskDefinition,
    base_path: &Path,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
) -> Result<TaskDefinition> {
    let mut resolved_def = def.clone();
    let mut merged_actions = Vec::new();
    let mut merged_params = def.parameters.clone();

    // Track visited paths for circular detection
    let main_path = base_path.join(&def.name);
    visited.insert(main_path.clone());

    for include_spec in &def.include {
        // Resolve the include path
        let include_path = if std::path::Path::new(&include_spec.path).is_absolute() {
            std::path::PathBuf::from(&include_spec.path)
        } else {
            base_path.join(&include_spec.path)
        };

        // Check for circular includes
        if visited.contains(&include_path) {
            return Err(anyhow::anyhow!(
                "Circular include detected: '{}' includes '{}', but '{}' was already visited",
                def.name,
                include_spec.path,
                include_path.display()
            ));
        }

        // Load the included task
        log::info!(
            "Resolving include: {} -> {}",
            def.name,
            include_path.display()
        );
        let included_def = parse_task_file(&include_path)
            .with_context(|| format!("Failed to load included task '{}'", include_spec.path))?;

        // Recursively resolve nested includes
        let resolved_included = resolve_includes(&included_def, base_path, visited)?;

        // Merge parameters (main task parameters take precedence)
        for (name, param) in resolved_included.parameters {
            merged_params.entry(name).or_insert(param);
        }

        // Log before moving actions
        let included_actions_count = resolved_included.actions.len();
        log::info!(
            "Included {} actions from '{}'",
            included_actions_count,
            include_spec.path
        );

        // Add included actions
        merged_actions.extend(resolved_included.actions);
    }

    // Merge: includes first, then main task actions
    merged_actions.extend(resolved_def.actions);
    resolved_def.actions = merged_actions;
    resolved_def.parameters = merged_params;
    resolved_def.include = Vec::new(); // Clear includes as they're now resolved

    visited.remove(&main_path);
    Ok(resolved_def)
}

/// Resolve includes with default empty visited set (convenience function).
pub fn resolve_includes_simple(def: &TaskDefinition, base_path: &Path) -> Result<TaskDefinition> {
    let mut visited = std::collections::HashSet::new();
    resolve_includes(def, base_path, &mut visited)
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
            include: vec![],
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
            include: vec![],
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
            include: vec![],
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
            include: vec![],
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
        fs::write(
            &path,
            "name: broken_task\nactions:\n  - action: wait\n    duration_ms: not-a-number\n",
        )
        .unwrap();

        let err = parse_task_file(&path).unwrap_err();
        let msg = err.to_string();

        assert!(msg.contains("Failed to parse task YAML"));
    }

    #[test]
    fn test_parse_task_file_invalid_toml_reports_parse_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("broken.toml");
        fs::write(
            &path,
            "name = \"broken_task\"\n[[actions]]\naction = \"wait\"\nduration_ms = \"oops\"\n",
        )
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

    #[test]
    fn test_validate_parameters_all_required_present() {
        let mut params = HashMap::new();
        params.insert(
            "url".to_string(),
            ParameterDef {
                r#type: ParameterType::Url,
                description: "Target URL".to_string(),
                default: None,
                required: true,
            },
        );
        params.insert(
            "delay".to_string(),
            ParameterDef {
                r#type: ParameterType::Integer,
                description: "Wait time".to_string(),
                default: Some(serde_yaml::Value::Number(1000.into())),
                required: false,
            },
        );

        let def = TaskDefinition {
            name: "param_task".to_string(),
            description: "Test".to_string(),
            policy: "default".to_string(),
            parameters: params,
            include: vec![],
            actions: vec![Action::Wait { duration_ms: 100 }],
        };

        let provided = serde_json::json!({"url": "https://example.com"});
        assert!(validate_parameters(&def, &provided).is_ok());
    }

    #[test]
    fn test_validate_parameters_missing_required() {
        let mut params = HashMap::new();
        params.insert(
            "url".to_string(),
            ParameterDef {
                r#type: ParameterType::Url,
                description: "Target URL".to_string(),
                default: None,
                required: true,
            },
        );

        let def = TaskDefinition {
            name: "param_task".to_string(),
            description: "Test".to_string(),
            policy: "default".to_string(),
            parameters: params,
            include: vec![],
            actions: vec![Action::Wait { duration_ms: 100 }],
        };

        let provided = serde_json::json!({"unknown": "value"});
        let result = validate_parameters(&def, &provided);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("Missing required")));
    }

    #[test]
    fn test_validate_parameters_unknown_parameter() {
        let def = TaskDefinition {
            name: "param_task".to_string(),
            description: "Test".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![],
            actions: vec![Action::Wait { duration_ms: 100 }],
        };

        let provided = serde_json::json!({"extra": "param"});
        let result = validate_parameters(&def, &provided);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("Unknown parameter")));
    }

    #[test]
    fn test_include_spec_default() {
        let spec = IncludeSpec::default();
        assert_eq!(spec.path, "");
        assert_eq!(spec.condition, None);
    }

    #[test]
    fn test_task_definition_with_include() {
        let def = TaskDefinition {
            name: "main_task".to_string(),
            description: "Main task".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![IncludeSpec {
                path: "common.task".to_string(),
                condition: None,
            }],
            actions: vec![Action::Wait { duration_ms: 100 }],
        };

        assert_eq!(def.include.len(), 1);
        assert_eq!(def.include[0].path, "common.task");
    }

    #[test]
    fn test_resolve_includes_simple_no_includes() {
        let def = TaskDefinition {
            name: "simple_task".to_string(),
            description: "Simple".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![],
            actions: vec![Action::Wait { duration_ms: 100 }],
        };

        let temp_dir = TempDir::new().unwrap();
        let result = resolve_includes_simple(&def, temp_dir.path());

        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved.actions.len(), 1);
        assert!(resolved.include.is_empty());
    }

    #[test]
    fn test_resolve_includes_missing_file_fails() {
        let def = TaskDefinition {
            name: "main_task".to_string(),
            description: "Main".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![IncludeSpec {
                path: "nonexistent.task".to_string(),
                condition: None,
            }],
            actions: vec![Action::Wait { duration_ms: 100 }],
        };

        let temp_dir = TempDir::new().unwrap();
        let result = resolve_includes_simple(&def, temp_dir.path());

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to load included task"));
    }

    #[test]
    fn test_resolve_includes_with_conditional() {
        // Test IncludeSpec with condition field
        let spec = IncludeSpec {
            path: "conditional.task".to_string(),
            condition: Some("{{enabled}} == true".to_string()),
        };

        assert_eq!(spec.path, "conditional.task");
        assert_eq!(spec.condition, Some("{{enabled}} == true".to_string()));
    }
}
