//! DSL parser module.
//!
//! Provides functions for parsing and validating DSL task files.
//! - `parse_task_file` - Parse a YAML task file into a `TaskDefinition`
//! - `validate_task_definition` - Validate a `TaskDefinition` for correctness

use crate::task::dsl::{Action, TaskDefinition};
use serde_yml;
use std::path::Path;
use toml;

/// Parse a DSL task file into a `TaskDefinition`.
///
/// # Arguments
/// * `path` - Path to the YAML task file
///
/// # Returns
/// * `Ok(TaskDefinition)` if parsing succeeds
/// * `Err(String)` with error message if parsing fails
pub fn parse_task_file<P: AsRef<Path>>(path: P) -> Result<TaskDefinition, String> {
    let path = path.as_ref();

    let content = std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {e}"))?;

    // Try YAML first
    match serde_yml::from_str::<TaskDefinition>(&content) {
        Ok(task) => Ok(task),
        Err(yaml_err) => {
            // YAML failed, try TOML
            match toml::from_str::<TaskDefinition>(&content) {
                Ok(task) => Ok(task),
                Err(toml_err) => {
                    // Both failed, return YAML error as primary
                    Err(format!(
                        "Failed to parse YAML: {yaml_err}, TOML error: {toml_err}"
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
#[must_use]
pub fn get_task_definition(name: &str) -> Option<TaskDefinition> {
    let registry = crate::task::registry::TaskRegistry::with_built_in_tasks();
    registry.get_task_definition(name).cloned()
}

/// Validate a `TaskDefinition` for correctness.
///
/// Checks:
/// - Task name is not empty
/// - Actions are valid (if any)
///
/// # Arguments
/// * `task_def` - The `TaskDefinition` to validate
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
                errors.push(
                    "'parallel' blocks are not yet implemented — use sequential actions instead"
                        .to_string(),
                );
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

/// Parse a YAML string into a `TaskDefinition`.
///
/// # Arguments
/// * `yaml` - YAML string to parse
///
/// # Returns
/// * `Ok(TaskDefinition)` if parsing succeeds
/// * `Err(String)` with error message if parsing fails
pub fn parse_task_yaml(yaml: &str) -> Result<TaskDefinition, String> {
    serde_yml::from_str(yaml).map_err(|e| format!("Failed to parse YAML: {e}"))
}

/// Parse a TOML string into a `TaskDefinition`.
///
/// # Arguments
/// * `toml_str` - TOML string to parse
///
/// # Returns
/// * `Ok(TaskDefinition)` if parsing succeeds
/// * `Err(String)` with error message if parsing fails
pub fn parse_task_toml(toml_str: &str) -> Result<TaskDefinition, String> {
    toml::from_str(toml_str).map_err(|e| format!("Failed to parse TOML: {e}"))
}

/// Format a `TaskDefinition` into a human-readable string.
///
/// # Arguments
/// * `task_def` - The `TaskDefinition` to format
///
/// # Returns
/// * Formatted string with task details
#[must_use]
pub fn format_task_definition(task_def: &TaskDefinition) -> String {
    let mut output = String::new();
    output.push_str(&format!("Task: {}\n", task_def.name));
    output.push_str(&format!("Description: {}\n", task_def.description));
    output.push_str(&format!("Policy: {}\n", task_def.policy));
    output.push_str(&format!("Parameters: {}\n", task_def.parameters.len()));
    output.push_str(&format!("Actions: {}\n", task_def.actions.len()));

    for (idx, action) in task_def.actions.iter().enumerate() {
        let formatted = format!("{action:?}");
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

    #[test]
    fn test_parse_task_yaml_rejects_invalid_yaml() {
        let result = parse_task_yaml("!:bad");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Failed to parse YAML"));
    }

    #[test]
    fn test_parse_task_toml_rejects_invalid_toml() {
        let result = parse_task_toml("foo = [unterminated");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Failed to parse TOML"));
    }

    #[test]
    fn test_format_task_definition_includes_task_name_and_action_order() {
        let task_def = TaskDefinition {
            name: "ordered".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: std::collections::HashMap::new(),
            include: vec![],
            actions: vec![
                Action::Navigate {
                    url: "https://a".to_string(),
                },
                Action::Click {
                    selector: "#b".to_string(),
                },
            ],
        };

        let formatted = format_task_definition(&task_def);
        let mut action_lines = 0usize;
        for line in formatted.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("1:") {
                action_lines |= 1;
            }
            if trimmed.starts_with("2:") {
                action_lines |= 2;
            }
        }
        assert!(
            formatted.starts_with("Task: ordered\n"),
            "format output should start with task name"
        );
        assert_eq!(
            action_lines, 3,
            "format output should list both actions in order"
        );
    }

    #[test]
    fn test_validate_retry_with_nested_valid_actions() {
        let task_def = TaskDefinition {
            name: "retry_valid".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: std::collections::HashMap::new(),
            include: vec![],
            actions: vec![Action::Retry {
                actions: vec![Action::Click {
                    selector: "#btn".to_string(),
                }],
                max_attempts: Some(3),
                initial_delay_ms: Some(1000),
                max_delay_ms: Some(5000),
                backoff_multiplier: Some(2.0),
                jitter: Some(true),
                retry_on: None,
            }],
        };

        let result = validate_task_definition(&task_def);
        assert!(
            result.is_ok(),
            "Valid Retry block should pass: {:?}",
            result
        );
    }

    #[test]
    fn test_validate_try_with_all_blocks() {
        let task_def = TaskDefinition {
            name: "try_valid".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: std::collections::HashMap::new(),
            include: vec![],
            actions: vec![Action::Try {
                try_actions: vec![Action::Click {
                    selector: "#maybe".to_string(),
                }],
                catch_actions: Some(vec![Action::Log {
                    message: "caught".to_string(),
                    level: None,
                }]),
                error_variable: Some("err".to_string()),
                finally_actions: Some(vec![Action::Log {
                    message: "finally".to_string(),
                    level: None,
                }]),
            }],
        };

        let result = validate_task_definition(&task_def);
        assert!(result.is_ok(), "Valid Try block should pass: {:?}", result);
    }

    #[test]
    fn test_validate_foreach_with_nested_actions() {
        let task_def = TaskDefinition {
            name: "foreach_valid".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: std::collections::HashMap::new(),
            include: vec![],
            actions: vec![Action::Foreach {
                variable: "item".to_string(),
                collection: crate::task::dsl::ForeachCollection::Array {
                    values: vec![serde_yml::Value::String("a".to_string())],
                },
                actions: vec![Action::Log {
                    message: "${item}".to_string(),
                    level: None,
                }],
                max_iterations: Some(50),
            }],
        };

        let result = validate_task_definition(&task_def);
        assert!(result.is_ok(), "Valid Foreach should pass: {:?}", result);
    }

    #[test]
    fn test_validate_parallel_produces_error() {
        let task_def = TaskDefinition {
            name: "parallel_test".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: std::collections::HashMap::new(),
            include: vec![],
            actions: vec![Action::Parallel {
                actions: vec![Action::Wait { duration_ms: 100 }],
                max_concurrency: Some(2),
            }],
        };

        let result = validate_task_definition(&task_def);
        assert!(
            result.is_err(),
            "Parallel blocks should produce a validation error (not yet implemented)"
        );
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.contains("parallel")),
            "Error should mention 'parallel': {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_while_with_valid_actions() {
        let task_def = TaskDefinition {
            name: "while_valid".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: std::collections::HashMap::new(),
            include: vec![],
            actions: vec![Action::While {
                condition: crate::task::dsl::Condition::True,
                actions: vec![Action::Wait { duration_ms: 100 }],
                max_iterations: Some(10),
            }],
        };

        let result = validate_task_definition(&task_def);
        assert!(result.is_ok(), "Valid While should pass: {:?}", result);
    }

    #[test]
    fn test_validate_nested_control_flow_errors() {
        // Retry containing an If with empty then branch
        let task_def = TaskDefinition {
            name: "nested_invalid".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: std::collections::HashMap::new(),
            include: vec![],
            actions: vec![Action::Retry {
                actions: vec![Action::If {
                    condition: crate::task::dsl::Condition::True,
                    then: vec![],
                    r#else: None,
                }],
                max_attempts: Some(2),
                initial_delay_ms: None,
                max_delay_ms: None,
                backoff_multiplier: None,
                jitter: None,
                retry_on: None,
            }],
        };

        let result = validate_task_definition(&task_def);
        assert!(
            result.is_err(),
            "Nested invalid action should produce errors"
        );
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.contains("empty 'then'")),
            "Should detect empty 'then' branch in nested If: {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_loop_without_count_or_condition() {
        let task_def = TaskDefinition {
            name: "loop_invalid".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: std::collections::HashMap::new(),
            include: vec![],
            actions: vec![Action::Loop {
                count: None,
                condition: None,
                actions: vec![Action::Wait { duration_ms: 100 }],
            }],
        };

        let result = validate_task_definition(&task_def);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.contains("count")),
            "Should mention missing count/condition: {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_if_empty_else() {
        let task_def = TaskDefinition {
            name: "if_empty_else".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: std::collections::HashMap::new(),
            include: vec![],
            actions: vec![Action::If {
                condition: crate::task::dsl::Condition::True,
                then: vec![Action::Wait { duration_ms: 10 }],
                r#else: Some(vec![]),
            }],
        };

        let result = validate_task_definition(&task_def);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.contains("empty 'else'")),
            "Should detect empty else branch: {:?}",
            errors
        );
    }

    #[test]
    fn test_get_task_definition_not_found() {
        let result = get_task_definition("definitely-nonexistent-task-name-xyz");
        assert!(result.is_none(), "Should return None for unknown task name");
    }

    #[test]
    fn test_parse_task_yaml_valid() {
        let yaml = r##"
name: yaml_test
description: "From YAML"
policy: default
actions:
  - action: navigate
    url: "https://example.com"
  - action: click
    selector: "#btn"
"##;
        let result = parse_task_yaml(yaml);
        assert!(result.is_ok());
        let task = result.unwrap();
        assert_eq!(task.name, "yaml_test");
        assert_eq!(task.actions.len(), 2);
    }

    #[test]
    fn test_parse_task_toml_valid() {
        let toml_str = r#"
name = "toml_test"
description = "From TOML"
policy = "default"

[[actions]]
action = "wait"
duration_ms = 200
"#;
        let result = parse_task_toml(toml_str);
        assert!(result.is_ok());
        let task = result.unwrap();
        assert_eq!(task.name, "toml_test");
        assert_eq!(task.actions.len(), 1);
    }

    /// Proptest fuzz strategies for DSL parser round-trip.
    ///
    /// Generates random valid TaskDefinitions, serializes to YAML, parses back,
    /// and asserts structural equality. This catches serde tag mismatches,
    /// missing fields, and other serialization/deserialization bugs.
    mod proptests {
        use super::*;
        use crate::task::dsl::{
            Condition, ForeachCollection, IncludeSpec, LogLevel, ParameterDef, ParameterType,
        };
        use proptest::collection::{hash_map, vec};
        use proptest::prelude::*;

        /// Maximum recursion depth for nested action/condition trees.
        const MAX_DEPTH: usize = 3;

        /// Maximum number of collection elements (actions, conditions, values).
        const MAX_SIZE: usize = 3;

        // ── Value strategies ────────────────────────────────────────────────

        fn arb_serde_yml_value() -> impl Strategy<Value = serde_yml::Value> {
            prop_oneof![
                any::<String>().prop_map(serde_yml::Value::String),
                any::<i64>().prop_map(|n| serde_yml::Value::Number(serde_yml::Number::from(n))),
                any::<bool>().prop_map(serde_yml::Value::Bool),
            ]
        }

        fn arb_log_level() -> impl Strategy<Value = LogLevel> {
            prop_oneof![
                Just(LogLevel::Info),
                Just(LogLevel::Debug),
                Just(LogLevel::Warn),
                Just(LogLevel::Error),
            ]
        }

        fn arb_parameter_type() -> impl Strategy<Value = ParameterType> {
            prop_oneof![
                Just(ParameterType::String),
                Just(ParameterType::Integer),
                Just(ParameterType::Boolean),
                Just(ParameterType::Url),
                Just(ParameterType::Selector),
            ]
        }

        // ── Condition strategies (recursive, depth-limited) ─────────────────

        fn arb_condition() -> impl Strategy<Value = Condition> {
            arb_condition_depth(0)
        }

        fn arb_condition_depth(depth: usize) -> impl Strategy<Value = Condition> {
            let leaf = prop_oneof![
                any::<String>().prop_map(|s| Condition::ElementExists { selector: s }),
                any::<String>().prop_map(|s| Condition::ElementVisible { selector: s }),
                (any::<String>(), any::<String>()).prop_map(|(s, v)| Condition::TextEquals {
                    selector: s,
                    value: v
                }),
                (any::<String>(), any::<String>()).prop_map(|(s, p)| Condition::TextMatches {
                    selector: s,
                    pattern: p
                }),
                (any::<String>(), arb_serde_yml_value())
                    .prop_map(|(n, v)| Condition::VariableEquals { name: n, value: v }),
                (any::<String>(), any::<String>()).prop_map(|(n, p)| Condition::VariableMatches {
                    name: n,
                    pattern: p
                }),
                (any::<String>(), any::<f64>())
                    .prop_map(|(n, v)| Condition::NumericGreaterThan { name: n, value: v }),
                (any::<String>(), any::<f64>())
                    .prop_map(|(n, v)| Condition::NumericLessThan { name: n, value: v }),
                (any::<String>(), any::<f64>(), any::<f64>()).prop_map(|(n, mn, mx)| {
                    Condition::NumericRange {
                        name: n,
                        min: mn,
                        max: mx,
                    }
                }),
                (
                    any::<String>(),
                    any::<String>(),
                    proptest::option::of(any::<String>())
                )
                    .prop_map(|(n, d, f)| Condition::DateBefore {
                        name: n,
                        date: d,
                        format: f,
                    }),
                (
                    any::<String>(),
                    any::<String>(),
                    proptest::option::of(any::<String>())
                )
                    .prop_map(|(n, d, f)| Condition::DateAfter {
                        name: n,
                        date: d,
                        format: f,
                    }),
                (any::<String>(), arb_serde_yml_value())
                    .prop_map(|(n, v)| Condition::ArrayContains { name: n, value: v }),
                (
                    any::<String>(),
                    proptest::option::of(1..10usize),
                    proptest::option::of(1..10usize),
                    proptest::option::of(1..10usize),
                )
                    .prop_map(|(n, min, max, exact)| Condition::ArrayLength {
                        name: n,
                        min,
                        max,
                        exact,
                    }),
                Just(Condition::True),
                Just(Condition::False),
                any::<String>().prop_map(|s| Condition::VariableDefined { name: s }),
                any::<String>().prop_map(|s| Condition::VariableNotDefined { name: s }),
            ];

            if depth >= MAX_DEPTH {
                return leaf.boxed();
            }

            let sub = arb_condition_depth(depth + 1).boxed();
            let recursive = prop_oneof![
                vec(sub.clone(), 0..MAX_SIZE)
                    .prop_map(|conds| Condition::And { conditions: conds }),
                vec(sub.clone(), 0..MAX_SIZE).prop_map(|conds| Condition::Or { conditions: conds }),
                sub.clone().prop_map(|cond| Condition::Not {
                    condition: Box::new(cond),
                }),
            ];

            prop_oneof![leaf, recursive].boxed()
        }

        // ── Action strategies (recursive, depth-limited) ────────────────────

        fn arb_foreach_collection() -> impl Strategy<Value = ForeachCollection> {
            prop_oneof![
                vec(arb_serde_yml_value(), 0..MAX_SIZE)
                    .prop_map(|values| ForeachCollection::Array { values }),
                (any::<i64>(), any::<i64>()).prop_map(|(s, e)| {
                    if s < e {
                        ForeachCollection::Range { start: s, end: e }
                    } else {
                        ForeachCollection::Range {
                            start: e,
                            end: s + 1,
                        }
                    }
                }),
                any::<String>().prop_map(|sel| ForeachCollection::Elements { selector: sel }),
                any::<String>().prop_map(|name| ForeachCollection::Variable { name }),
            ]
        }

        fn arb_action() -> impl Strategy<Value = Action> {
            arb_action_depth(0)
        }

        fn arb_action_depth(depth: usize) -> impl Strategy<Value = Action> {
            let leaf = prop_oneof![
                any::<String>().prop_map(|url| Action::Navigate { url }),
                any::<String>().prop_map(|sel| Action::Click { selector: sel }),
                (any::<String>(), any::<String>()).prop_map(|(sel, text)| Action::Type {
                    selector: sel,
                    text
                }),
                (1..60000u64).prop_map(|ms| Action::Wait { duration_ms: ms }),
                (any::<String>(), proptest::option::of(1..30000u64)).prop_map(|(sel, t)| {
                    Action::WaitFor {
                        selector: sel,
                        timeout_ms: t,
                    }
                },),
                any::<String>().prop_map(|sel| Action::ScrollTo { selector: sel }),
                (any::<String>(), proptest::option::of(any::<String>())).prop_map(|(sel, v)| {
                    Action::Extract {
                        selector: sel,
                        variable: v,
                    }
                },),
                any::<String>().prop_map(|script| Action::Execute { script }),
                (any::<String>(), proptest::option::of(arb_log_level())).prop_map(|(msg, lvl)| {
                    Action::Log {
                        message: msg,
                        level: lvl,
                    }
                },),
                (
                    proptest::option::of(any::<String>()),
                    proptest::option::of(any::<String>())
                )
                    .prop_map(|(p, s)| Action::Screenshot {
                        path: p,
                        selector: s,
                    }),
                any::<String>().prop_map(|sel| Action::Clear { selector: sel }),
                any::<String>().prop_map(|sel| Action::Hover { selector: sel }),
                (
                    any::<String>(),
                    any::<String>(),
                    proptest::option::of(any::<bool>())
                )
                    .prop_map(|(sel, val, bv)| Action::Select {
                        selector: sel,
                        value: val,
                        by_value: bv,
                    },),
                any::<String>().prop_map(|sel| Action::RightClick { selector: sel }),
                any::<String>().prop_map(|sel| Action::DoubleClick { selector: sel }),
                (
                    any::<String>(),
                    proptest::option::of(hash_map(
                        any::<String>(),
                        arb_serde_yml_value(),
                        0..MAX_SIZE
                    ))
                )
                    .prop_map(|(task, params)| Action::Call {
                        task,
                        parameters: params,
                    }),
            ];

            if depth >= MAX_DEPTH {
                return leaf.boxed();
            }

            let sub_action = arb_action_depth(depth + 1).boxed();
            let sub_actions = vec(sub_action.clone(), 0..MAX_SIZE);

            let recursive = prop_oneof![
                (
                    arb_condition(),
                    sub_actions.clone(),
                    proptest::option::of(sub_actions.clone())
                )
                    .prop_map(|(cond, then, r#else)| Action::If {
                        condition: cond,
                        then,
                        r#else,
                    }),
                (
                    proptest::option::of(1..10u32),
                    proptest::option::of(arb_condition()),
                    sub_actions.clone(),
                )
                    .prop_map(|(count, cond, actions)| Action::Loop {
                        count,
                        condition: cond,
                        actions,
                    }),
                (
                    any::<String>(),
                    arb_foreach_collection(),
                    sub_actions.clone(),
                    proptest::option::of(1..20u32)
                )
                    .prop_map(|(var, coll, acts, max_iter)| Action::Foreach {
                        variable: var,
                        collection: coll,
                        actions: acts,
                        max_iterations: max_iter,
                    }),
                (
                    arb_condition(),
                    sub_actions.clone(),
                    proptest::option::of(1..20u32)
                )
                    .prop_map(|(cond, acts, max_iter)| Action::While {
                        condition: cond,
                        actions: acts,
                        max_iterations: max_iter,
                    }),
                (
                    sub_actions.clone(),
                    proptest::option::of(1..10u32),
                    proptest::option::of(1..5000u64),
                    proptest::option::of(1..60000u64),
                    proptest::option::of(any::<f64>()),
                    proptest::option::of(any::<bool>()),
                    proptest::option::of(vec(any::<String>(), 0..MAX_SIZE)),
                )
                    .prop_map(
                        |(acts, max_att, init_d, max_d, mult, jit, retry_on)| Action::Retry {
                            actions: acts,
                            max_attempts: max_att,
                            initial_delay_ms: init_d,
                            max_delay_ms: max_d,
                            backoff_multiplier: mult,
                            jitter: jit,
                            retry_on,
                        },
                    ),
                (sub_actions.clone(), proptest::option::of(1..5usize),).prop_map(
                    |(acts, max_conc)| Action::Parallel {
                        actions: acts,
                        max_concurrency: max_conc,
                    }
                ),
                (
                    sub_actions.clone(),
                    proptest::option::of(sub_actions.clone()),
                    proptest::option::of(any::<String>()),
                    proptest::option::of(sub_actions.clone()),
                )
                    .prop_map(|(try_acts, catch_acts, err_var, finally_acts)| {
                        Action::Try {
                            try_actions: try_acts,
                            catch_actions: catch_acts,
                            error_variable: err_var,
                            finally_actions: finally_acts,
                        }
                    },),
            ];

            prop_oneof![leaf, recursive].boxed()
        }

        // ── Task definition strategy ────────────────────────────────────────

        fn arb_parameter_def() -> impl Strategy<Value = ParameterDef> {
            (
                arb_parameter_type(),
                any::<String>(),
                proptest::option::of(arb_serde_yml_value()),
                any::<bool>(),
            )
                .prop_map(|(r#type, description, default, required)| ParameterDef {
                    r#type,
                    description,
                    default,
                    required,
                })
        }

        fn arb_include_spec() -> impl Strategy<Value = IncludeSpec> {
            (any::<String>(), proptest::option::of(any::<String>()))
                .prop_map(|(path, condition)| IncludeSpec { path, condition })
        }

        fn arb_task_definition() -> impl Strategy<Value = TaskDefinition> {
            (
                any::<String>(),
                any::<String>(),
                any::<String>(),
                hash_map(any::<String>(), arb_parameter_def(), 0..MAX_SIZE),
                vec(arb_include_spec(), 0..2),
                vec(arb_action(), 0..MAX_SIZE),
            )
                .prop_map(
                    |(name, description, policy, parameters, include, actions)| TaskDefinition {
                        name,
                        description,
                        policy,
                        parameters,
                        include,
                        actions,
                    },
                )
        }

        // ── Round-trip tests ───────────────────────────────────────────────

        proptest! {
            /// Round-trip a TaskDefinition through YAML serialization and parsing.
            /// Generates random task definitions and verifies structural equality.
            #[test]
            fn test_yaml_round_trip(task_def in arb_task_definition()) {
                let yaml = serde_yml::to_string(&task_def)
                    .expect("YAML serialization should succeed");
                let parsed: TaskDefinition = serde_yml::from_str(&yaml)
                    .expect("YAML deserialization should succeed");
                prop_assert_eq!(task_def, parsed);
            }

            /// Round-trip individual Actions through YAML.
            /// Catches per-action serde tag mismatches.
            #[test]
            fn test_action_yaml_round_trip(action in arb_action()) {
                let yaml = serde_yml::to_string(&action)
                    .expect("Action YAML serialization should succeed");
                let parsed: Action = serde_yml::from_str(&yaml)
                    .expect("Action YAML deserialization should succeed");
                prop_assert_eq!(action, parsed);
            }

            /// Round-trip individual Conditions through YAML.
            /// Catches condition serde tag mismatches.
            #[test]
            fn test_condition_yaml_round_trip(condition in arb_condition()) {
                let yaml = serde_yml::to_string(&condition)
                    .expect("Condition YAML serialization should succeed");
                let parsed: Condition = serde_yml::from_str(&yaml)
                    .expect("Condition YAML deserialization should succeed");
                prop_assert_eq!(condition, parsed);
            }

            /// Round-trip ForeachCollection through YAML.
            #[test]
            fn test_foreach_collection_yaml_round_trip(
                collection in arb_foreach_collection()
            ) {
                let yaml = serde_yml::to_string(&collection)
                    .expect("ForeachCollection YAML serialization should succeed");
                let parsed: ForeachCollection = serde_yml::from_str(&yaml)
                    .expect("ForeachCollection YAML deserialization should succeed");
                prop_assert_eq!(collection, parsed);
            }

            /// Round-trip a ParameterDef through YAML.
            #[test]
            fn test_parameter_def_yaml_round_trip(def in arb_parameter_def()) {
                let yaml = serde_yml::to_string(&def)
                    .expect("ParameterDef YAML serialization should succeed");
                let parsed: ParameterDef = serde_yml::from_str(&yaml)
                    .expect("ParameterDef YAML deserialization should succeed");
                prop_assert_eq!(def, parsed);
            }

            /// Round-trip a TaskDefinition through TOML serialization and parsing.
            /// Note: TOML has limitations with tagged unions, so this covers
            /// simpler task definitions (no deeply nested tagged enums).
            #[test]
            fn test_task_definition_toml_round_trip(task_def in arb_task_definition()) {
                let toml_str = toml::to_string(&task_def)
                    .expect("TOML serialization should succeed");
                let parsed: TaskDefinition = toml::from_str(&toml_str)
                    .expect("TOML deserialization should succeed");
                prop_assert_eq!(task_def, parsed);
            }
        }
    }
}
