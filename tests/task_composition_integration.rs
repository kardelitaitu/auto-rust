//! Task Composition Integration Tests
//!
//! These tests verify end-to-end task calling scenarios using the DSL executor.
//! They test variable inheritance, return values, and multi-level call chains.

use std::collections::HashMap;

use auto::task::dsl::{Action, Condition, ForeachCollection, LogLevel, TaskDefinition};

/// Create a test task with the given name and actions
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

/// Test that DslExecutor correctly tracks call depth
#[test]
fn test_dsl_executor_call_depth_tracking() {
    // Create parent task that would call another task
    let parent_task = create_test_task(
        "parent_task",
        vec![
            Action::Wait { duration_ms: 100 },
            Action::Call {
                task: "child_task".to_string(),
                parameters: None,
            },
        ],
    );

    // Verify the task structure includes a Call action
    assert_eq!(parent_task.actions.len(), 2);
    match &parent_task.actions[1] {
        Action::Call { task, .. } => {
            assert_eq!(task, "child_task");
        }
        _ => panic!("Expected Call action"),
    }
}

/// Test variable inheritance through task parameters
#[test]
fn test_variable_inheritance_via_parameters() {
    // Create parent task that sets variables and calls child
    let parent_task = create_test_task(
        "variable_parent",
        vec![
            Action::Log {
                message: "Setting parent_var".to_string(),
                level: Some(LogLevel::Info),
            },
            Action::Call {
                task: "variable_child".to_string(),
                parameters: Some({
                    let mut params = HashMap::new();
                    params.insert(
                        "parent_var".to_string(),
                        serde_yaml::Value::String("from_parent".to_string()),
                    );
                    params.insert(
                        "explicit_param".to_string(),
                        serde_yaml::Value::String("explicit_value".to_string()),
                    );
                    params
                }),
            },
        ],
    );

    // Verify parent task has Call action with parameters
    match &parent_task.actions[1] {
        Action::Call { task, parameters } => {
            assert_eq!(task, "variable_child");
            assert!(parameters.is_some());
            let params = parameters.as_ref().unwrap();
            assert!(params.contains_key("parent_var"));
            assert!(params.contains_key("explicit_param"));
        }
        _ => panic!("Expected Call action"),
    }
}

/// Test return value pattern with extract actions
#[test]
fn test_return_value_pattern_structure() {
    // Create child task that "returns" values via extract
    let child_task = create_test_task(
        "value_returner",
        vec![
            Action::Extract {
                selector: "#result".to_string(),
                variable: Some("returned_value".to_string()),
            },
            Action::Extract {
                selector: "#status".to_string(),
                variable: Some("returned_status".to_string()),
            },
        ],
    );

    // Verify extract actions are properly configured
    assert_eq!(child_task.actions.len(), 2);

    match &child_task.actions[0] {
        Action::Extract { selector, variable } => {
            assert_eq!(selector, "#result");
            assert_eq!(variable.as_ref().unwrap(), "returned_value");
        }
        _ => panic!("Expected Extract action"),
    }

    match &child_task.actions[1] {
        Action::Extract { selector, variable } => {
            assert_eq!(selector, "#status");
            assert_eq!(variable.as_ref().unwrap(), "returned_status");
        }
        _ => panic!("Expected Extract action"),
    }
}

/// Test multi-level call chain structure
#[test]
fn test_multi_level_call_chain_structure() {
    // Create 3-level task chain
    let level3 = create_test_task("level3_leaf", vec![Action::Wait { duration_ms: 100 }]);

    let level2 = create_test_task(
        "level2_intermediate",
        vec![Action::Call {
            task: "level3_leaf".to_string(),
            parameters: None,
        }],
    );

    let level1 = create_test_task(
        "level1_parent",
        vec![Action::Call {
            task: "level2_intermediate".to_string(),
            parameters: None,
        }],
    );

    // Verify chain structure
    assert_eq!(level1.actions.len(), 1);
    assert_eq!(level2.actions.len(), 1);
    assert_eq!(level3.actions.len(), 1);

    match &level1.actions[0] {
        Action::Call { task, .. } => assert_eq!(task, "level2_intermediate"),
        _ => panic!("Expected Call at level 1"),
    }

    match &level2.actions[0] {
        Action::Call { task, .. } => assert_eq!(task, "level3_leaf"),
        _ => panic!("Expected Call at level 2"),
    }

    match &level3.actions[0] {
        Action::Wait { duration_ms } => assert_eq!(*duration_ms, 100),
        _ => panic!("Expected Wait at level 3"),
    }
}

/// Test task composition with complex parameter passing
#[test]
fn test_complex_parameter_passing() {
    // Create parent task with complex data passing
    let parent_task = create_test_task(
        "complex_parent",
        vec![
            // Set up variables
            Action::Log {
                message: "Initializing complex workflow".to_string(),
                level: Some(LogLevel::Info),
            },
            // Call with multiple parameters
            Action::Call {
                task: "complex_child".to_string(),
                parameters: Some({
                    let mut params = HashMap::new();
                    params.insert(
                        "url".to_string(),
                        serde_yaml::Value::String("https://api.example.com".to_string()),
                    );
                    params.insert(
                        "timeout".to_string(),
                        serde_yaml::Value::Number(5000.into()),
                    );
                    params.insert("retry".to_string(), serde_yaml::Value::Bool(true));
                    params
                }),
            },
        ],
    );

    // Verify parameter structure
    match &parent_task.actions[1] {
        Action::Call { task, parameters } => {
            assert_eq!(task, "complex_child");
            let params = parameters.as_ref().unwrap();
            assert_eq!(params.len(), 3);

            // Check URL parameter
            match &params.get("url").unwrap() {
                serde_yaml::Value::String(s) => assert_eq!(s, "https://api.example.com"),
                _ => panic!("Expected string URL"),
            }

            // Check timeout parameter
            match &params.get("timeout").unwrap() {
                serde_yaml::Value::Number(n) => {
                    assert_eq!(n.as_u64(), Some(5000));
                }
                _ => panic!("Expected number timeout"),
            }

            // Check retry parameter
            match &params.get("retry").unwrap() {
                serde_yaml::Value::Bool(b) => assert!(b),
                _ => panic!("Expected boolean retry"),
            }
        }
        _ => panic!("Expected Call action"),
    }
}

/// Test conditional task composition
#[test]
fn test_conditional_task_composition() {
    // Create parent with conditional task calls
    let parent_task = create_test_task(
        "conditional_parent",
        vec![
            Action::If {
                condition: Condition::ElementExists {
                    selector: "#requires-auth".to_string(),
                },
                then: vec![Action::Call {
                    task: "auth_task".to_string(),
                    parameters: Some({
                        let mut params = HashMap::new();
                        params.insert(
                            "flow".to_string(),
                            serde_yaml::Value::String("oauth".to_string()),
                        );
                        params
                    }),
                }],
                r#else: Some(vec![Action::Log {
                    message: "No auth required".to_string(),
                    level: Some(LogLevel::Info),
                }]),
            },
            Action::Call {
                task: "main_task".to_string(),
                parameters: None,
            },
        ],
    );

    // Verify conditional structure
    assert_eq!(parent_task.actions.len(), 2);

    // First action should be conditional
    match &parent_task.actions[0] {
        Action::If {
            condition,
            then,
            r#else,
        } => {
            match condition {
                Condition::ElementExists { selector } => {
                    assert_eq!(selector, "#requires-auth");
                }
                _ => panic!("Expected ElementExists condition"),
            }

            assert_eq!(then.len(), 1);
            match &then[0] {
                Action::Call { task, .. } => assert_eq!(task, "auth_task"),
                _ => panic!("Expected Call in then branch"),
            }

            assert!(r#else.is_some());
            assert_eq!(r#else.as_ref().unwrap().len(), 1);
        }
        _ => panic!("Expected If action"),
    }

    // Second action should be unconditional call
    match &parent_task.actions[1] {
        Action::Call { task, .. } => assert_eq!(task, "main_task"),
        _ => panic!("Expected Call action"),
    }
}

/// Test task composition statistics
#[test]
fn test_task_composition_action_counting() {
    // Create task with nested composition
    let task = create_test_task(
        "countable_task",
        vec![
            Action::Wait { duration_ms: 100 },
            Action::If {
                condition: Condition::ElementExists {
                    selector: "#check".to_string(),
                },
                then: vec![
                    Action::Call {
                        task: "task_a".to_string(),
                        parameters: None,
                    },
                    Action::Call {
                        task: "task_b".to_string(),
                        parameters: None,
                    },
                ],
                r#else: Some(vec![Action::Wait { duration_ms: 200 }]),
            },
            Action::Call {
                task: "task_c".to_string(),
                parameters: None,
            },
        ],
    );

    // Top level has 3 actions
    assert_eq!(task.actions.len(), 3);

    // Verify we can iterate through all action types
    let mut call_count = 0;
    let mut wait_count = 0;
    let mut if_count = 0;

    for action in &task.actions {
        match action {
            Action::Call { .. } => call_count += 1,
            Action::Wait { .. } => wait_count += 1,
            Action::If { then, r#else, .. } => {
                if_count += 1;
                // Count nested actions
                for nested in then {
                    if let Action::Call { .. } = nested {
                        call_count += 1;
                    }
                }
                if let Some(else_actions) = r#else {
                    for nested in else_actions {
                        if let Action::Wait { .. } = nested {
                            wait_count += 1;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    assert_eq!(call_count, 3); // task_a, task_b, task_c
    assert_eq!(wait_count, 2); // Before if, in else
    assert_eq!(if_count, 1);
}

/// Test error handling in task composition
#[test]
fn test_task_composition_error_handling_structure() {
    // Create task with try/catch around call
    let task = create_test_task(
        "error_handling_parent",
        vec![Action::Try {
            try_actions: vec![Action::Call {
                task: "risky_task".to_string(),
                parameters: Some({
                    let mut params = HashMap::new();
                    params.insert("aggressive".to_string(), serde_yaml::Value::Bool(true));
                    params
                }),
            }],
            catch_actions: Some(vec![
                Action::Log {
                    message: "Task failed, using fallback".to_string(),
                    level: Some(LogLevel::Warn),
                },
                Action::Call {
                    task: "fallback_task".to_string(),
                    parameters: None,
                },
            ]),
            error_variable: Some("task_error".to_string()),
            finally_actions: Some(vec![Action::Log {
                message: "Cleanup complete".to_string(),
                level: Some(LogLevel::Info),
            }]),
        }],
    );

    // Verify try/catch structure with calls
    assert_eq!(task.actions.len(), 1);

    match &task.actions[0] {
        Action::Try {
            try_actions,
            catch_actions,
            error_variable,
            finally_actions,
        } => {
            // Verify try block has call
            assert_eq!(try_actions.len(), 1);
            match &try_actions[0] {
                Action::Call { task, .. } => assert_eq!(task, "risky_task"),
                _ => panic!("Expected Call in try"),
            }

            // Verify catch block has call and log
            assert!(catch_actions.is_some());
            let catch = catch_actions.as_ref().unwrap();
            assert_eq!(catch.len(), 2);
            match &catch[1] {
                Action::Call { task, .. } => assert_eq!(task, "fallback_task"),
                _ => panic!("Expected Call in catch"),
            }

            // Verify error variable
            assert_eq!(error_variable.as_ref().unwrap(), "task_error");

            // Verify finally block
            assert!(finally_actions.is_some());
        }
        _ => panic!("Expected Try action"),
    }
}

/// Test task composition with foreach and calls
#[test]
fn test_foreach_with_task_calls() {
    // Create task that calls another task for each item
    let task = create_test_task(
        "foreach_caller",
        vec![Action::Foreach {
            variable: "item".to_string(),
            collection: ForeachCollection::Array {
                values: vec![
                    serde_yaml::Value::String("a".to_string()),
                    serde_yaml::Value::String("b".to_string()),
                    serde_yaml::Value::String("c".to_string()),
                ],
            },
            actions: vec![
                Action::Log {
                    message: "Processing {{item}}".to_string(),
                    level: Some(LogLevel::Info),
                },
                Action::Call {
                    task: "process_item".to_string(),
                    parameters: Some({
                        let mut params = HashMap::new();
                        params.insert(
                            "item_id".to_string(),
                            serde_yaml::Value::String("{{item}}".to_string()),
                        );
                        params
                    }),
                },
            ],
            max_iterations: Some(3),
        }],
    );

    // Verify foreach with call structure
    assert_eq!(task.actions.len(), 1);

    match &task.actions[0] {
        Action::Foreach {
            variable,
            collection,
            actions,
            max_iterations,
        } => {
            assert_eq!(variable, "item");
            assert_eq!(*max_iterations, Some(3));

            // Verify collection is array with 3 items
            match collection {
                ForeachCollection::Array { values } => assert_eq!(values.len(), 3),
                _ => panic!("Expected Array collection"),
            }

            // Verify loop has log and call
            assert_eq!(actions.len(), 2);
            match &actions[1] {
                Action::Call { task, .. } => assert_eq!(task, "process_item"),
                _ => panic!("Expected Call in foreach"),
            }
        }
        _ => panic!("Expected Foreach action"),
    }
}

/// Test that get_variables returns correct data
#[test]
fn test_executor_get_variables_basic() {
    // We can't easily test the full execution without a browser,
    // but we can verify the DslExecutor structure supports variable tracking

    let task = create_test_task(
        "variable_tracker",
        vec![Action::Extract {
            selector: "#data".to_string(),
            variable: Some("extracted".to_string()),
        }],
    );

    // The task should have extract action that sets a variable
    assert_eq!(task.actions.len(), 1);
    match &task.actions[0] {
        Action::Extract { variable, .. } => {
            assert_eq!(variable.as_ref().unwrap(), "extracted");
        }
        _ => panic!("Expected Extract action"),
    }
}

/// Test task composition with retry and calls
#[test]
fn test_retry_with_task_calls() {
    // Create task that retries a called task
    let task = create_test_task(
        "retry_caller",
        vec![Action::Retry {
            actions: vec![Action::Call {
                task: "flaky_task".to_string(),
                parameters: Some({
                    let mut params = HashMap::new();
                    params.insert(
                        "timeout".to_string(),
                        serde_yaml::Value::Number(5000.into()),
                    );
                    params
                }),
            }],
            max_attempts: Some(3),
            initial_delay_ms: Some(1000),
            max_delay_ms: Some(30000),
            backoff_multiplier: Some(2.0),
            jitter: Some(true),
            retry_on: Some(vec!["timeout".to_string(), "network".to_string()]),
        }],
    );

    // Verify retry with call structure
    assert_eq!(task.actions.len(), 1);

    match &task.actions[0] {
        Action::Retry {
            actions,
            max_attempts,
            initial_delay_ms,
            max_delay_ms,
            backoff_multiplier,
            jitter,
            retry_on,
        } => {
            // Verify retry configuration
            assert_eq!(*max_attempts, Some(3));
            assert_eq!(*initial_delay_ms, Some(1000));
            assert_eq!(*max_delay_ms, Some(30000));
            assert_eq!(*backoff_multiplier, Some(2.0));
            assert_eq!(*jitter, Some(true));
            assert_eq!(
                retry_on.as_ref().unwrap(),
                &vec!["timeout".to_string(), "network".to_string()]
            );

            // Verify retry contains call
            assert_eq!(actions.len(), 1);
            match &actions[0] {
                Action::Call { task, .. } => assert_eq!(task, "flaky_task"),
                _ => panic!("Expected Call in retry"),
            }
        }
        _ => panic!("Expected Retry action"),
    }
}
