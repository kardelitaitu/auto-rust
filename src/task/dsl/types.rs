//! DSL types for task definitions.
//!
//! This module provides the type definitions for DSL tasks:
//! - `TaskDefinition` - main task structure
//! - `Action` - action enum with all supported actions
//! - `Condition` - condition enum for control flow
//! - `ParameterDef`, `ParameterType` - parameter definitions
//! - `DurationMs` - wrapper type for u64 that supports dereferencing

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_definition_serialization() {
        let task = TaskDefinition {
            name: "test-task".to_string(),
            description: "A test task".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![],
            actions: vec![Action::Wait { duration_ms: 1000 }],
        };

        let json = serde_json::to_string(&task).unwrap();
        let deserialized: TaskDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(task, deserialized);
    }

    #[test]
    fn test_task_definition_defaults() {
        let task = TaskDefinition {
            name: "minimal".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![],
            actions: vec![],
        };

        // Test that defaults work
        assert_eq!(task.policy, "default");
        assert!(task.parameters.is_empty());
        assert!(task.include.is_empty());
        assert!(task.actions.is_empty());
    }

    #[test]
    fn test_include_spec_serialization() {
        let include = IncludeSpec {
            path: "other.task".to_string(),
            condition: Some("env == 'test'".to_string()),
        };

        let json = serde_json::to_string(&include).unwrap();
        let deserialized: IncludeSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(include, deserialized);
    }

    #[test]
    fn test_parameter_def_serialization() {
        let param = ParameterDef {
            r#type: ParameterType::String,
            description: "A string parameter".to_string(),
            default: Some(serde_yml::Value::String("default".to_string())),
            required: false,
        };

        let json = serde_json::to_string(&param).unwrap();
        let deserialized: ParameterDef = serde_json::from_str(&json).unwrap();

        assert_eq!(param, deserialized);
    }

    #[test]
    fn test_action_variants() {
        // Test that all action variants can be created and are distinct
        let actions = vec![
            Action::Navigate {
                url: "https://example.com".to_string(),
            },
            Action::Click {
                selector: "#btn".to_string(),
            },
            Action::Type {
                selector: "#input".to_string(),
                text: "hello".to_string(),
            },
            Action::Wait { duration_ms: 1000 },
            Action::WaitFor {
                selector: "#element".to_string(),
                timeout_ms: Some(5000),
            },
            Action::ScrollTo {
                selector: "#target".to_string(),
            },
            Action::Extract {
                selector: "#text".to_string(),
                variable: Some("content".to_string()),
            },
            Action::Execute {
                script: "console.log('test')".to_string(),
            },
            Action::If {
                condition: Condition::ElementVisible {
                    selector: "#btn".to_string(),
                },
                then: vec![],
                r#else: None,
            },
            Action::Loop {
                count: Some(3),
                condition: None,
                actions: vec![],
            },
            Action::Call {
                task: "other-task".to_string(),
                parameters: None,
            },
            Action::Log {
                message: "test".to_string(),
                level: Some(LogLevel::Info),
            },
            Action::Screenshot {
                path: Some("screenshot.png".to_string()),
                selector: None,
            },
        ];

        // Verify we have multiple distinct actions
        assert!(actions.len() > 10);

        // Test serialization of each
        for action in actions {
            let json = serde_json::to_string(&action).unwrap();
            let deserialized: Action = serde_json::from_str(&json).unwrap();
            assert_eq!(action, deserialized);
        }
    }

    #[test]
    fn test_condition_serialization() {
        let conditions = vec![
            Condition::ElementExists {
                selector: "#btn".to_string(),
            },
            Condition::ElementVisible {
                selector: "#visible".to_string(),
            },
            Condition::TextEquals {
                selector: "#status".to_string(),
                value: "ready".to_string(),
            },
            Condition::TextMatches {
                selector: "#msg".to_string(),
                pattern: "success.*".to_string(),
            },
            Condition::VariableEquals {
                name: "status".to_string(),
                value: serde_yml::Value::String("ok".to_string()),
            },
            Condition::NumericGreaterThan {
                name: "count".to_string(),
                value: 5.0,
            },
            Condition::ArrayContains {
                name: "items".to_string(),
                value: serde_yml::Value::String("item1".to_string()),
            },
            Condition::ArrayLength {
                name: "list".to_string(),
                min: Some(1),
                max: Some(10),
                exact: None,
            },
            Condition::And {
                conditions: vec![
                    Condition::ElementVisible {
                        selector: "#a".to_string(),
                    },
                    Condition::ElementVisible {
                        selector: "#b".to_string(),
                    },
                ],
            },
            Condition::Or {
                conditions: vec![
                    Condition::ElementVisible {
                        selector: "#a".to_string(),
                    },
                    Condition::ElementExists {
                        selector: "#b".to_string(),
                    },
                ],
            },
        ];

        for condition in conditions {
            let json = serde_json::to_string(&condition).unwrap();
            let deserialized: Condition = serde_json::from_str(&json).unwrap();
            assert_eq!(condition, deserialized);
        }
    }

    #[test]
    fn test_parameter_types() {
        assert_eq!(ParameterType::String as u8, 0);
        assert_eq!(ParameterType::Integer as u8, 1);
        assert_eq!(ParameterType::Boolean as u8, 2);
        assert_eq!(ParameterType::Url as u8, 3);
        assert_eq!(ParameterType::Selector as u8, 4);
    }

    #[test]
    fn test_log_level_variants() {
        assert_eq!(LogLevel::Info as u8, 0);
        assert_eq!(LogLevel::Debug as u8, 1);
        assert_eq!(LogLevel::Warn as u8, 2);
        assert_eq!(LogLevel::Error as u8, 3);
    }

    #[test]
    fn test_resolve_includes_circular_detection() {
        // A circular include graph: A.task includes B.task, B.task includes A.task.
        // The cycle guard (visited HashSet in resolve_includes_inner) should detect
        // the cycle and skip the duplicate include rather than infinite-looping.
        let dir = tempfile::TempDir::new().unwrap();
        let dir_path = dir.path();

        // Create A.task — references B.task
        let a_yaml = r#"
name: task_a
description: "Task A"
policy: default
actions:
  - action: wait
    duration_ms: 100
include:
  - path: B.task
"#;
        std::fs::write(dir_path.join("A.task"), a_yaml).unwrap();

        // Create B.task — references A.task (circular!)
        let b_yaml = r##"
name: task_b
description: "Task B"
policy: default
actions:
  - action: click
    selector: "#btn"
include:
  - path: A.task
"##;
        std::fs::write(dir_path.join("B.task"), b_yaml).unwrap();

        // Create the initial TaskDefinition for A (with its include spec)
        let task = TaskDefinition {
            name: "task_a".to_string(),
            description: "Task A".to_string(),
            policy: "default".to_string(),
            parameters: std::collections::HashMap::new(),
            include: vec![IncludeSpec {
                path: "B.task".to_string(),
                condition: None,
            }],
            actions: vec![Action::Wait { duration_ms: 100 }],
        };

        // Resolve includes — this should NOT infinite-loop
        let result = task.resolve_includes(Some(dir_path));
        assert!(
            result.is_ok(),
            "resolve_includes should succeed: {:?}",
            result
        );

        let resolved = result.unwrap();

        // Expected actions = 3:
        //   1. A's own Wait(100) — from the initial TaskDefinition
        //   2. B's Click("#btn") — from including B.task
        //   3. A's Wait(100) — B.task includes A.task on disk, so A's actions are
        //      legitimately added via B's include resolution
        //
        // The cycle guard prevents action #4 (re-including B from the on-disk A.task).
        // Without the guard, the chain would be infinite:
        //   A → B → A → B → ...
        // The visited.insert() for "B.task" on the second pass returns false,
        // stopping the recursion. Total = 3, not infinite.
        assert_eq!(
            resolved.actions.len(),
            3,
            "Should have exactly 3 merged actions (A + B + A from B's include), not {}. \
             The cycle guard prevents infinite recursion, but A.task is a distinct file \
             that B legitimately includes, so its actions appear once.",
            resolved.actions.len()
        );

        // Verify both action types are present
        let wait_count = resolved
            .actions
            .iter()
            .filter(|a| matches!(a, Action::Wait { duration_ms: 100 }))
            .count();
        let has_click = resolved
            .actions
            .iter()
            .any(|a| matches!(a, Action::Click { selector } if selector == "#btn"));
        // Wait(100) appears twice: once from initial A, once from on-disk A (included by B)
        assert_eq!(
            wait_count, 2,
            "Wait(100) should appear twice (initial A + on-disk A via B)"
        );
        assert!(has_click, "Click from B.task should be present");
    }

    #[test]
    fn test_resolve_includes_no_cycle_no_duplication() {
        // Sanity check: a non-circular chain should merge all actions exactly once.
        // A.task includes B.task, but B.task does NOT include A.task back.
        let dir = tempfile::TempDir::new().unwrap();
        let dir_path = dir.path();

        let a_yaml = r#"
name: task_a
description: "Task A"
policy: default
actions:
  - action: wait
    duration_ms: 100
include:
  - path: B.task
"#;
        std::fs::write(dir_path.join("A.task"), a_yaml).unwrap();

        let b_yaml = r##"
name: task_b
description: "Task B"
policy: default
actions:
  - action: click
    selector: "#btn2"
include: []
"##;
        std::fs::write(dir_path.join("B.task"), b_yaml).unwrap();

        let task = TaskDefinition {
            name: "task_a".to_string(),
            description: "Task A".to_string(),
            policy: "default".to_string(),
            parameters: std::collections::HashMap::new(),
            include: vec![IncludeSpec {
                path: "B.task".to_string(),
                condition: None,
            }],
            actions: vec![Action::Wait { duration_ms: 100 }],
        };

        let result = task.resolve_includes(Some(dir_path));
        assert!(result.is_ok());

        let resolved = result.unwrap();
        assert_eq!(
            resolved.actions.len(),
            2,
            "Non-circular chain should merge exactly 2 actions"
        );
    }
}
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

impl TaskDefinition {
    /// Resolve includes by loading and merging included task files into actions.
    ///
    /// Recursively resolves nested includes. Unconditional includes (condition is None)
    /// are resolved eagerly; conditional includes are skipped with a warning.
    ///
    /// # Arguments
    /// * `base_path` - Optional base directory for resolving relative include paths.
    ///   If None, relative paths are resolved against the current working directory.
    ///
    /// # Returns
    /// `Ok(self)` with all included actions merged, or `Err` with error message.
    pub fn resolve_includes(self, base_path: Option<&std::path::Path>) -> Result<Self, String> {
        let mut visited = HashSet::new();
        self.resolve_includes_inner(base_path, &mut visited)
    }

    /// Internal recursive resolve with cycle detection.
    fn resolve_includes_inner(
        mut self,
        base_path: Option<&std::path::Path>,
        visited: &mut HashSet<std::path::PathBuf>,
    ) -> Result<Self, String> {
        let includes = std::mem::take(&mut self.include);

        if includes.is_empty() {
            return Ok(self);
        }

        for include in includes {
            // Skip conditional includes for now
            if include.condition.is_some() {
                log::warn!(
                    "Conditional includes not yet supported: skipping '{}'",
                    include.path
                );
                continue;
            }

            // Resolve the path
            let path = std::path::Path::new(&include.path);
            let resolved_path = if path.is_relative() {
                if let Some(base) = base_path {
                    base.join(path)
                } else {
                    path.to_path_buf()
                }
            } else {
                path.to_path_buf()
            };

            log::debug!(
                "Resolving include '{}' from path '{:?}'",
                include.path,
                resolved_path
            );

            // Check for circular includes
            if !visited.insert(resolved_path.clone()) {
                log::warn!(
                    "Circular include detected: '{}' already processed, skipping",
                    include.path
                );
                continue;
            }

            // Load the included task file
            let included = crate::task::dsl::parser::parse_task_file(&resolved_path)?;

            // Recursively resolve its includes (with the same base dir)
            let base_for_child = resolved_path.parent();
            let resolved = included.resolve_includes_inner(base_for_child, visited)?;

            // Merge actions from the included task
            let count = resolved.actions.len();
            self.actions.extend(resolved.actions);
            log::info!(
                "Merged {} actions from included task '{}'",
                count,
                include.path
            );

            // Merge parameters (don't overwrite existing)
            for (key, value) in resolved.parameters {
                self.parameters.entry(key).or_insert(value);
            }
        }

        Ok(self)
    }
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
    pub default: Option<serde_yml::Value>,
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
        parameters: Option<HashMap<String, serde_yml::Value>>,
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
    /// Iterate over a collection with a loop variable
    Foreach {
        /// Variable name to bind each iteration value (e.g., "item", "index")
        variable: String,
        /// Collection to iterate over: array, range, or selector for DOM elements
        collection: ForeachCollection,
        /// Actions to execute for each iteration
        actions: Vec<Action>,
        /// Maximum number of iterations (default: 100, safety limit)
        max_iterations: Option<u32>,
    },
    /// While loop with condition-based execution
    While {
        /// Condition to evaluate before each iteration
        condition: Condition,
        /// Actions to execute while condition is true
        actions: Vec<Action>,
        /// Maximum number of iterations (default: 1000, safety limit)
        max_iterations: Option<u32>,
    },
    /// Try-catch-finally for error handling
    Try {
        /// Actions to attempt (try block)
        try_actions: Vec<Action>,
        /// Actions to execute on error (catch block)
        catch_actions: Option<Vec<Action>>,
        /// Variable name to store error message
        error_variable: Option<String>,
        /// Actions to always execute (finally block)
        finally_actions: Option<Vec<Action>>,
    },
}

/// Collection types for the Foreach action.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ForeachCollection {
    /// Array of values to iterate over
    Array { values: Vec<serde_yml::Value> },
    /// Range of integers (inclusive start, exclusive end)
    Range { start: i64, end: i64 },
    /// DOM elements matching a selector
    Elements { selector: String },
    /// Use a variable containing an array
    Variable { name: String },
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
    /// Check text matches regex pattern
    TextMatches { selector: String, pattern: String },
    /// Check if variable equals value
    VariableEquals {
        name: String,
        value: serde_yml::Value,
    },
    /// Check if variable matches regex pattern
    VariableMatches { name: String, pattern: String },
    /// Check if numeric variable is greater than value
    NumericGreaterThan { name: String, value: f64 },
    /// Check if numeric variable is less than value
    NumericLessThan { name: String, value: f64 },
    /// Check numeric variable is within range (inclusive)
    NumericRange { name: String, min: f64, max: f64 },
    /// Check if date string is before reference date
    DateBefore {
        name: String,
        date: String,
        format: Option<String>,
    },
    /// Check if date string is after reference date
    DateAfter {
        name: String,
        date: String,
        format: Option<String>,
    },
    /// Check if array variable contains a value
    ArrayContains {
        name: String,
        value: serde_yml::Value,
    },
    /// Check if array variable length matches
    ArrayLength {
        name: String,
        min: Option<usize>,
        max: Option<usize>,
        exact: Option<usize>,
    },
    /// Logical AND of multiple conditions
    And { conditions: Vec<Condition> },
    /// Logical OR of multiple conditions
    Or { conditions: Vec<Condition> },
    /// Negate a condition
    Not { condition: Box<Condition> },
    /// Always true
    True,
    /// Always false
    False,
    /// Check if variable is defined
    VariableDefined { name: String },
    /// Check if variable is not defined
    VariableNotDefined { name: String },
}
