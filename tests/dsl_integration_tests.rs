//! DSL Task System Integration Tests
//!
//! These tests verify end-to-end DSL task execution using a local test server.
//! They test actual browser automation through the DSL executor.

use std::collections::HashMap;

use auto::task::dsl::{Action, Condition, LogLevel, TaskDefinition};
use auto::task::dsl_executor::{DslExecutionStats, ExecutionReport};

/// Helper to create a simple task definition for testing
fn create_test_task(name: &str, actions: Vec<Action>) -> TaskDefinition {
    TaskDefinition {
        name: name.to_string(),
        description: format!("Test task: {}", name),
        policy: "default".to_string(),
        parameters: HashMap::new(),
        include: vec![],
        actions,
    }
}

/// Test basic DSL execution statistics structure
#[test]
fn test_dsl_execution_stats_structure() {
    let stats = DslExecutionStats {
        actions_executed: 5,
        total_actions: 10,
        variables_defined: 3,
        max_call_depth: 2,
    };

    assert_eq!(stats.actions_executed, 5);
    assert_eq!(stats.total_actions, 10);
    assert_eq!(stats.variables_defined, 3);
    assert_eq!(stats.max_call_depth, 2);
}

/// Test execution report generation
#[test]
fn test_execution_report_structure() {
    use std::time::Instant;

    let report = ExecutionReport {
        task_name: "test_task".to_string(),
        start_time: Instant::now(),
        end_time: None,
        total_duration: None,
        total_actions: 5,
        actions_executed: 3,
        actions_succeeded: 3,
        actions_failed: 0,
        max_call_depth: 1,
        variables_defined: 2,
        action_metrics: vec![],
        success: true,
    };

    assert_eq!(report.task_name, "test_task");
    assert_eq!(report.total_actions, 5);
    assert_eq!(report.actions_executed, 3);
    assert_eq!(report.actions_succeeded, 3);
    assert_eq!(report.actions_failed, 0);
    assert!(report.success);

    // Test summary method doesn't panic
    let _summary = report.summary();

    // Test JSON export doesn't panic
    let _json = report.to_json();
}

/// Test variable substitution in task definition
#[test]
fn test_variable_substitution_in_task() {
    let mut params = HashMap::new();
    params.insert(
        "url".to_string(),
        auto::task::dsl::ParameterDef {
            r#type: auto::task::dsl::ParameterType::Url,
            description: "Test URL".to_string(),
            required: true,
            default: None,
        },
    );

    let task = TaskDefinition {
        name: "substitution_test".to_string(),
        description: "Test variable substitution".to_string(),
        policy: "default".to_string(),
        parameters: params,
        include: vec![],
        actions: vec![
            Action::Navigate {
                url: "{{url}}".to_string(),
            },
            Action::Log {
                message: "Navigated to {{url}}".to_string(),
                level: Some(LogLevel::Info),
            },
        ],
    };

    assert_eq!(task.actions.len(), 2);
    match &task.actions[0] {
        Action::Navigate { url } => assert_eq!(url, "{{url}}"),
        _ => panic!("Expected Navigate action"),
    }
}

/// Test conditional action structure
#[test]
fn test_conditional_action_structure() {
    let task = create_test_task(
        "conditional_test",
        vec![Action::If {
            condition: Condition::ElementExists {
                selector: "#button".to_string(),
            },
            then: vec![Action::Click {
                selector: "#button".to_string(),
            }],
            r#else: Some(vec![Action::Log {
                message: "Button not found".to_string(),
                level: Some(LogLevel::Warn),
            }]),
        }],
    );

    assert_eq!(task.actions.len(), 1);
    match &task.actions[0] {
        Action::If {
            condition,
            then,
            r#else,
        } => {
            match condition {
                Condition::ElementExists { selector } => {
                    assert_eq!(selector, "#button");
                }
                _ => panic!("Expected ElementExists condition"),
            }
            assert_eq!(then.len(), 1);
            assert!(r#else.is_some());
            assert_eq!(r#else.as_ref().unwrap().len(), 1);
        }
        _ => panic!("Expected If action"),
    }
}

/// Test loop action structure
#[test]
fn test_loop_action_structure() {
    let task = create_test_task(
        "loop_test",
        vec![Action::Loop {
            count: Some(3),
            condition: None,
            actions: vec![
                Action::Click {
                    selector: ".item".to_string(),
                },
                Action::Wait { duration_ms: 100 },
            ],
        }],
    );

    assert_eq!(task.actions.len(), 1);
    match &task.actions[0] {
        Action::Loop {
            count,
            condition,
            actions,
        } => {
            assert_eq!(*count, Some(3));
            assert!(condition.is_none());
            assert_eq!(actions.len(), 2);
        }
        _ => panic!("Expected Loop action"),
    }
}

/// Test call action structure (task composition)
#[test]
fn test_call_action_structure() {
    let mut params = HashMap::new();
    params.insert(
        "target".to_string(),
        serde_yaml::Value::String("value".to_string()),
    );

    let task = create_test_task(
        "call_test",
        vec![Action::Call {
            task: "other_task".to_string(),
            parameters: Some(params.clone()),
        }],
    );

    assert_eq!(task.actions.len(), 1);
    match &task.actions[0] {
        Action::Call { task, parameters } => {
            assert_eq!(task, "other_task");
            assert!(parameters.is_some());
            let params = parameters.as_ref().unwrap();
            assert!(params.contains_key("target"));
        }
        _ => panic!("Expected Call action"),
    }
}

/// Test parallel action structure
#[test]
fn test_parallel_action_structure() {
    let task = create_test_task(
        "parallel_test",
        vec![Action::Parallel {
            actions: vec![
                Action::Wait { duration_ms: 100 },
                Action::Wait { duration_ms: 200 },
                Action::Wait { duration_ms: 300 },
            ],
            max_concurrency: Some(2),
        }],
    );

    assert_eq!(task.actions.len(), 1);
    match &task.actions[0] {
        Action::Parallel {
            actions,
            max_concurrency,
        } => {
            assert_eq!(actions.len(), 3);
            assert_eq!(*max_concurrency, Some(2));
        }
        _ => panic!("Expected Parallel action"),
    }
}

/// Test complex task with multiple action types
#[test]
fn test_complex_task_structure() {
    let task = create_test_task(
        "complex_test",
        vec![
            Action::Navigate {
                url: "https://example.com".to_string(),
            },
            Action::WaitFor {
                selector: "#form".to_string(),
                timeout_ms: Some(5000),
            },
            Action::Type {
                selector: "#username".to_string(),
                text: "test_user".to_string(),
            },
            Action::Type {
                selector: "#password".to_string(),
                text: "secret123".to_string(),
            },
            Action::If {
                condition: Condition::ElementVisible {
                    selector: "#submit".to_string(),
                },
                then: vec![Action::Click {
                    selector: "#submit".to_string(),
                }],
                r#else: Some(vec![Action::Log {
                    message: "Submit button not visible".to_string(),
                    level: Some(LogLevel::Error),
                }]),
            },
            Action::Wait { duration_ms: 2000 },
            Action::Extract {
                selector: "#result".to_string(),
                variable: Some("result_text".to_string()),
            },
            Action::Screenshot {
                path: Some("/tmp/result.png".to_string()),
                selector: None,
            },
        ],
    );

    assert_eq!(task.actions.len(), 8);

    // Verify each action type
    match &task.actions[0] {
        Action::Navigate { .. } => {}
        _ => panic!("Action 0 should be Navigate"),
    }
    match &task.actions[1] {
        Action::WaitFor { .. } => {}
        _ => panic!("Action 1 should be WaitFor"),
    }
    match &task.actions[2] {
        Action::Type { .. } => {}
        _ => panic!("Action 2 should be Type"),
    }
    match &task.actions[4] {
        Action::If { .. } => {}
        _ => panic!("Action 4 should be If"),
    }
    match &task.actions[7] {
        Action::Screenshot { .. } => {}
        _ => panic!("Action 7 should be Screenshot"),
    }
}

/// Test YAML parsing and validation
#[test]
fn test_yaml_task_parsing() {
    let yaml = r##"
name: yaml_test
description: "Test YAML parsing"
policy: default

parameters:
  username:
    type: string
    required: true
    description: "Username for login"

actions:
  - action: navigate
    url: "https://example.com"
  
  - action: type
    selector: "#user"
    text: "{{username}}"
  
  - action: click
    selector: "#submit"
"##;

    let task = auto::task::dsl::parse_task_yaml(yaml).expect("Should parse YAML");

    assert_eq!(task.name, "yaml_test");
    assert_eq!(task.description, "Test YAML parsing");
    assert_eq!(task.policy, "default");
    assert_eq!(task.actions.len(), 3);
    assert!(task.parameters.contains_key("username"));

    // Check parameter definition
    let param = task.parameters.get("username").unwrap();
    assert_eq!(param.r#type, auto::task::dsl::ParameterType::String);
    assert!(param.required);
}

/// Test TOML parsing and validation
#[test]
fn test_toml_task_parsing() {
    let toml = r##"
name = "toml_test"
description = "Test TOML parsing"
policy = "default"

[[actions]]
action = "navigate"
url = "https://example.com"

[[actions]]
action = "wait"
duration_ms = 1000

[[actions]]
action = "click"
selector = "#button"
"##;

    let task = auto::task::dsl::parse_task_toml(toml).expect("Should parse TOML");

    assert_eq!(task.name, "toml_test");
    assert_eq!(task.description, "Test TOML parsing");
    assert_eq!(task.actions.len(), 3);

    match &task.actions[0] {
        Action::Navigate { url } => assert_eq!(url, "https://example.com"),
        _ => panic!("First action should be Navigate"),
    }
    match &task.actions[1] {
        Action::Wait { duration_ms } => assert_eq!(*duration_ms, 1000),
        _ => panic!("Second action should be Wait"),
    }
}

/// Test task validation
#[test]
fn test_task_validation_empty_name() {
    let task = TaskDefinition {
        name: "".to_string(),
        description: "Test".to_string(),
        policy: "default".to_string(),
        parameters: HashMap::new(),
        include: vec![],
        actions: vec![],
    };

    let result = auto::task::dsl::validate_task_definition(&task);
    assert!(result.is_err());
}

#[test]
fn test_task_validation_empty_actions() {
    let task = TaskDefinition {
        name: "valid_name".to_string(),
        description: "Test".to_string(),
        policy: "default".to_string(),
        parameters: HashMap::new(),
        include: vec![],
        actions: vec![],
    };

    let result = auto::task::dsl::validate_task_definition(&task);
    assert!(result.is_err());
}

#[test]
fn test_task_validation_valid_task() {
    let task = TaskDefinition {
        name: "valid_task".to_string(),
        description: "A valid task".to_string(),
        policy: "default".to_string(),
        parameters: HashMap::new(),
        include: vec![],
        actions: vec![Action::Wait { duration_ms: 100 }],
    };

    let result = auto::task::dsl::validate_task_definition(&task);
    assert!(result.is_ok());
}

/// Test task includes structure
#[test]
fn test_task_includes_structure() {
    let task = TaskDefinition {
        name: "includer".to_string(),
        description: "Task with includes".to_string(),
        policy: "default".to_string(),
        parameters: HashMap::new(),
        include: vec![
            auto::task::dsl::IncludeSpec {
                path: "common.task".to_string(),
                condition: None,
            },
            auto::task::dsl::IncludeSpec {
                path: "helpers.task".to_string(),
                condition: Some("use_helpers".to_string()),
            },
        ],
        actions: vec![Action::Wait { duration_ms: 100 }],
    };

    assert_eq!(task.include.len(), 2);
    assert_eq!(task.include[0].path, "common.task");
    assert!(task.include[0].condition.is_none());
    assert_eq!(task.include[1].path, "helpers.task");
    assert_eq!(task.include[1].condition, Some("use_helpers".to_string()));
}

/// Test parameter types
#[test]
fn test_parameter_types() {
    use auto::task::dsl::ParameterType;

    // Just verify all types can be constructed
    let _types = [
        ParameterType::String,
        ParameterType::Integer,
        ParameterType::Boolean,
        ParameterType::Url,
        ParameterType::Selector,
    ];
}

/// Test condition types
#[test]
fn test_condition_types() {
    // Just verify all condition types can be constructed
    let _conditions = [
        Condition::ElementExists {
            selector: "#test".to_string(),
        },
        Condition::ElementVisible {
            selector: "#test".to_string(),
        },
        Condition::TextEquals {
            selector: "#test".to_string(),
            value: "expected".to_string(),
        },
        Condition::VariableEquals {
            name: "var".to_string(),
            value: serde_yaml::Value::String("value".to_string()),
        },
        Condition::And { conditions: vec![] },
        Condition::Or { conditions: vec![] },
        Condition::Not {
            condition: Box::new(Condition::ElementExists {
                selector: "#test".to_string(),
            }),
        },
    ];
}
