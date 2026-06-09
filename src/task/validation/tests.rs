use super::*;
use crate::task::dsl::{Action, Condition, ForeachCollection, ParameterDef, TaskDefinition};
use std::collections::{HashMap, HashSet};

fn create_basic_task() -> TaskDefinition {
    TaskDefinition {
        name: "test_task".to_string(),
        description: "Test task".to_string(),
        policy: "default".to_string(),
        parameters: HashMap::new(),
        include: vec![],
        actions: vec![Action::Wait { duration_ms: 100 }],
    }
}

#[test]
fn test_validate_empty_task_name() {
    let mut task = create_basic_task();
    task.name = "".to_string();

    let report = validate_task(&task);
    assert!(!report.is_valid());
    assert!(report
        .issues
        .iter()
        .any(|i| i.message().contains("name cannot be empty")));
}

#[test]
fn test_validate_task_name_with_spaces() {
    let mut task = create_basic_task();
    task.name = "test task".to_string();

    let report = validate_task(&task);
    assert!(!report.is_valid());
    assert!(report.issues.iter().any(|i| i.message().contains("spaces")));
}

#[test]
fn test_validate_empty_actions() {
    let mut task = create_basic_task();
    task.actions = vec![];

    let report = validate_task(&task);
    assert!(!report.is_valid());
    assert!(report
        .issues
        .iter()
        .any(|i| i.message().contains("at least one action")));
}

#[test]
fn test_validate_empty_selector() {
    let mut task = create_basic_task();
    task.actions = vec![Action::Click {
        selector: "".to_string(),
    }];

    let report = validate_task(&task);
    assert!(!report.is_valid());
    assert!(report
        .issues
        .iter()
        .any(|i| i.message().contains("Selector cannot be empty")));
}

#[test]
fn test_validate_unbalanced_selector() {
    let mut task = create_basic_task();
    task.actions = vec![Action::Click {
        selector: "div[class='test'".to_string(),
    }];

    let report = validate_task(&task);
    assert!(!report.is_valid());
    assert!(report
        .issues
        .iter()
        .any(|i| i.message().contains("unbalanced")));
}

#[test]
fn test_validate_zero_wait_duration() {
    let mut task = create_basic_task();
    task.actions = vec![Action::Wait { duration_ms: 0 }];

    let report = validate_task(&task);
    assert!(report.is_valid());
    assert!(report.issues.iter().any(|i| i.message().contains("0ms")));
}

#[test]
fn test_validate_valid_task() {
    let task = create_basic_task();

    let report = validate_task(&task);
    assert!(report.is_valid());
    assert_eq!(report.error_count(), 0);
}

#[test]
fn test_validate_if_empty_then() {
    let mut task = create_basic_task();
    task.actions = vec![Action::If {
        condition: Condition::ElementExists {
            selector: "div".to_string(),
        },
        then: vec![],
        r#else: None,
    }];

    let report = validate_task(&task);
    assert!(report.is_valid());
    assert!(report
        .issues
        .iter()
        .any(|i| i.message().contains("'then' block has no actions")));
}

#[test]
fn test_validate_call_unknown_task() {
    let mut task = create_basic_task();
    task.actions = vec![Action::Call {
        task: "unknown_task".to_string(),
        parameters: None,
    }];

    let known: HashSet<String> = vec!["known_task".to_string()].into_iter().collect();
    let report = TaskValidator::new().with_known_tasks(known).validate(&task);

    assert!(report.is_valid());
    assert!(report
        .issues
        .iter()
        .any(|i| i.message().contains("not in the known task list")));
}

#[test]
fn test_validate_loop_without_count_or_condition() {
    let mut task = create_basic_task();
    task.actions = vec![Action::Loop {
        count: None,
        condition: None,
        actions: vec![Action::Wait { duration_ms: 100 }],
    }];

    let report = validate_task(&task);
    assert!(!report.is_valid());
    assert!(report
        .issues
        .iter()
        .any(|i| i.message().contains("must have either")));
}

#[test]
fn test_validate_retry_zero_attempts() {
    let mut task = create_basic_task();
    task.actions = vec![Action::Retry {
        actions: vec![Action::Wait { duration_ms: 100 }],
        max_attempts: Some(0),
        initial_delay_ms: None,
        max_delay_ms: None,
        backoff_multiplier: None,
        jitter: None,
        retry_on: None,
    }];

    let report = validate_task(&task);
    assert!(!report.is_valid());
    assert!(report
        .issues
        .iter()
        .any(|i| i.message().contains("cannot be 0")));
}

#[test]
fn test_validate_foreach_invalid_range() {
    let mut task = create_basic_task();
    task.actions = vec![Action::Foreach {
        variable: "i".to_string(),
        collection: ForeachCollection::Range { start: 10, end: 5 },
        actions: vec![Action::Wait { duration_ms: 100 }],
        max_iterations: None,
    }];

    let report = validate_task(&task);
    assert!(!report.is_valid());
    assert!(report
        .issues
        .iter()
        .any(|i| i.message().contains("start") && i.message().contains("end")));
}

#[test]
fn test_extract_variables() {
    let validator = TaskValidator::new();
    let mut report = ValidationReport::new("test".to_string());

    validator.extract_variables("Hello ${name}, your id is ${id}", &mut report);

    assert!(report.variables_referenced.contains("name"));
    assert!(report.variables_referenced.contains("id"));
}

#[test]
fn test_count_actions() {
    let validator = TaskValidator::new();

    let actions = vec![
        Action::Wait { duration_ms: 100 },
        Action::If {
            condition: Condition::ElementExists {
                selector: "div".to_string(),
            },
            then: vec![
                Action::Click {
                    selector: "button".to_string(),
                },
                Action::Wait { duration_ms: 500 },
            ],
            r#else: Some(vec![Action::Wait { duration_ms: 200 }]),
        },
    ];

    let count = validator.count_actions(&actions);
    assert_eq!(count, 5);
}

#[test]
fn test_circular_reference_self_call() {
    let task = TaskDefinition {
        name: "self_calling".to_string(),
        description: "Calls itself".to_string(),
        policy: "default".to_string(),
        parameters: HashMap::new(),
        include: vec![],
        actions: vec![Action::Call {
            task: "self_calling".to_string(),
            parameters: None,
        }],
    };

    let report = validate_task(&task);

    assert!(!report.is_valid(), "Self-calling task should be invalid");
    assert!(
        report.issues.iter().any(|i| {
            i.message().contains("circular reference") || i.message().contains("calls itself")
        }),
        "Should report circular reference error"
    );
}

#[test]
fn test_no_false_circular_positive() {
    let task = TaskDefinition {
        name: "caller".to_string(),
        description: "Calls another task".to_string(),
        policy: "default".to_string(),
        parameters: HashMap::new(),
        include: vec![],
        actions: vec![Action::Call {
            task: "callee".to_string(),
            parameters: None,
        }],
    };

    let report = validate_task(&task);

    assert!(!report.issues.iter().any(|i| {
        i.message().contains("circular reference") || i.message().contains("calls itself")
    }));
}

#[test]
fn test_deep_nesting_limit() {
    fn create_nested_if(depth: usize) -> Action {
        if depth == 0 {
            Action::Wait { duration_ms: 100 }
        } else {
            Action::If {
                condition: Condition::ElementExists {
                    selector: format!("#level{}", depth),
                },
                then: vec![create_nested_if(depth - 1)],
                r#else: None,
            }
        }
    }

    let task = TaskDefinition {
        name: "deep_nested".to_string(),
        description: "Very deeply nested".to_string(),
        policy: "default".to_string(),
        parameters: HashMap::new(),
        include: vec![],
        actions: vec![create_nested_if(12)],
    };

    let report = validate_task(&task);

    assert!(!report.is_valid(), "Should fail due to nesting depth");
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.message().contains("nesting depth")),
        "Should report nesting depth error"
    );
}

#[test]
fn test_custom_nesting_limit() {
    fn create_nested_if(depth: usize) -> Action {
        if depth == 0 {
            Action::Wait { duration_ms: 100 }
        } else {
            Action::If {
                condition: Condition::ElementExists {
                    selector: format!("#level{}", depth),
                },
                then: vec![create_nested_if(depth - 1)],
                r#else: None,
            }
        }
    }

    let task = TaskDefinition {
        name: "medium_nested".to_string(),
        description: "Medium nesting".to_string(),
        policy: "default".to_string(),
        parameters: HashMap::new(),
        include: vec![],
        actions: vec![create_nested_if(8)],
    };

    let report = TaskValidator::new().validate(&task);
    assert!(report.is_valid(), "8 levels should pass with limit of 10");

    let report = TaskValidator::new()
        .with_max_nesting_depth(5)
        .validate(&task);
    assert!(!report.is_valid(), "8 levels should fail with limit of 5");
}

#[test]
fn test_multiple_call_actions_tracked() {
    let task = TaskDefinition {
        name: "multi_caller".to_string(),
        description: "Calls multiple tasks".to_string(),
        policy: "default".to_string(),
        parameters: HashMap::new(),
        include: vec![],
        actions: vec![
            Action::Call {
                task: "task_a".to_string(),
                parameters: None,
            },
            Action::Call {
                task: "task_b".to_string(),
                parameters: None,
            },
            Action::Call {
                task: "task_c".to_string(),
                parameters: None,
            },
        ],
    };

    let report = validate_task(&task);

    assert!(report.tasks_called.contains("task_a"));
    assert!(report.tasks_called.contains("task_b"));
    assert!(report.tasks_called.contains("task_c"));
    assert_eq!(report.tasks_called.len(), 3);
}

#[test]
fn issue_error_message() {
    let e = ValidationIssue::Error("x".into());
    assert_eq!(e.message(), "x");
    assert!(e.is_error());
}
#[test]
fn issue_warning_message() {
    let w = ValidationIssue::Warning("y".into());
    assert_eq!(w.message(), "y");
    assert!(!w.is_error());
}
#[test]
fn report_new_empty() {
    let r = ValidationReport::new("t".into());
    assert!(r.is_valid());
    assert_eq!(r.error_count(), 0);
    assert_eq!(r.warning_count(), 0);
}
#[test]
fn report_add_error() {
    let mut r = ValidationReport::new("t".into());
    r.error("e");
    assert!(!r.is_valid());
    assert!(r.has_errors());
    assert_eq!(r.error_count(), 1);
}
#[test]
fn report_add_warning() {
    let mut r = ValidationReport::new("t".into());
    r.warning("w");
    assert!(r.is_valid());
    assert_eq!(r.warning_count(), 1);
}
#[test]
fn report_mixed_counts() {
    let mut r = ValidationReport::new("t".into());
    r.error("e1");
    r.error("e2");
    r.warning("w");
    assert_eq!(r.error_count(), 2);
    assert_eq!(r.warning_count(), 1);
}
#[test]
fn report_summary_valid() {
    let r = ValidationReport::new("t".into());
    assert!(r.summary().contains("is valid"));
}
#[test]
fn report_summary_warnings() {
    let mut r = ValidationReport::new("t".into());
    r.warning("w");
    assert!(r.summary().contains("warning(s)"));
}
#[test]
fn report_summary_errors() {
    let mut r = ValidationReport::new("t".into());
    r.error("e");
    assert!(r.summary().contains("error(s)"));
}
#[test]
fn report_summary_mixed() {
    let mut r = ValidationReport::new("t".into());
    r.error("e");
    r.warning("w");
    assert!(r.summary().contains("error(s)"));
    assert!(r.summary().contains("warning(s)"));
}
#[test]
fn report_action_count_default() {
    let r = ValidationReport::new("t".into());
    assert_eq!(r.action_count, 0);
}
#[test]
fn report_variables_empty() {
    let r = ValidationReport::new("t".into());
    assert!(r.variables_referenced.is_empty());
}
#[test]
fn report_tasks_called_empty() {
    let r = ValidationReport::new("t".into());
    assert!(r.tasks_called.is_empty());
}
#[test]
fn issue_partial_eq_error() {
    let a = ValidationIssue::Error("z".into());
    let b = ValidationIssue::Error("z".into());
    assert_eq!(a, b);
}
#[test]
fn issue_partial_eq_warning() {
    let a = ValidationIssue::Warning("z".into());
    let b = ValidationIssue::Warning("z".into());
    assert_eq!(a, b);
}
#[test]
fn report_clone() {
    let mut r = ValidationReport::new("t".into());
    r.error("e");
    let c = r.clone();
    assert_eq!(c.error_count(), 1);
}
#[test]
fn report_debug() {
    let r = ValidationReport::new("t".into());
    let _ = format!("{:?}", r);
}
#[test]
fn issue_debug() {
    let e = ValidationIssue::Error("d".into());
    let _ = format!("{:?}", e);
}
#[test]
fn report_multiple_warnings() {
    let mut r = ValidationReport::new("t".into());
    for i in 0..3 {
        r.warning(i.to_string());
    }
    assert_eq!(r.warning_count(), 3);
}
#[test]
fn report_set_action_count_direct() {
    let mut r = ValidationReport::new("t".into());
    r.action_count = 42;
    assert_eq!(r.action_count, 42);
}
#[test]
fn report_insert_variable() {
    let mut r = ValidationReport::new("t".into());
    r.variables_referenced.insert("v1".into());
    assert_eq!(r.variables_referenced.len(), 1);
}
#[test]
fn report_insert_task_call() {
    let mut r = ValidationReport::new("t".into());
    r.tasks_called.insert("sub".into());
    assert!(r.tasks_called.contains("sub"));
}
#[test]
fn issue_error_vs_warning_ne() {
    let e = ValidationIssue::Error("x".into());
    let w = ValidationIssue::Warning("x".into());
    assert_ne!(e, w);
}
#[test]
fn report_name_preserved() {
    let r = ValidationReport::new("my_task".into());
    assert_eq!(r.task_name, "my_task");
}
#[test]
fn report_error_twice() {
    let mut r = ValidationReport::new("t".into());
    r.error("e");
    r.error("e2");
    assert_eq!(r.error_count(), 2);
}
#[test]
fn report_warning_twice() {
    let mut r = ValidationReport::new("t".into());
    r.warning("w1");
    r.warning("w2");
    assert_eq!(r.warning_count(), 2);
}
#[test]
fn report_zero_actions_summary() {
    let r = ValidationReport::new("t".into());
    assert!(r.summary().contains("0 actions"));
}
#[test]
fn report_one_action_summary() {
    let mut r = ValidationReport::new("t".into());
    r.action_count = 1;
    assert!(r.summary().contains("1 actions"));
}
#[test]
fn report_large_error_count() {
    let mut r = ValidationReport::new("t".into());
    for _ in 0..10 {
        r.error("e");
    }
    assert_eq!(r.error_count(), 10);
}
#[test]
fn report_large_warning_count() {
    let mut r = ValidationReport::new("t".into());
    for _ in 0..10 {
        r.warning("w");
    }
    assert_eq!(r.warning_count(), 10);
}
#[test]
fn issue_message_long() {
    let e = ValidationIssue::Error("a".repeat(100));
    assert_eq!(e.message().len(), 100);
}
#[test]
fn report_clone_independent() {
    let mut r = ValidationReport::new("t".into());
    r.error("e");
    let mut c = r.clone();
    c.warning("w");
    assert_eq!(r.warning_count(), 0);
    assert_eq!(c.warning_count(), 1);
}
#[test]
fn report_debug_contains_name() {
    let r = ValidationReport::new("debug_t".into());
    assert!(format!("{:?}", r).contains("debug_t"));
}
#[test]
fn issue_partial_eq_diff_msg() {
    let a = ValidationIssue::Error("1".into());
    let b = ValidationIssue::Error("2".into());
    assert_ne!(a, b);
}
#[test]
fn report_empty_issues_vec() {
    let r = ValidationReport::new("t".into());
    assert!(r.issues.is_empty());
}
#[test]
fn report_issues_len_after_add() {
    let mut r = ValidationReport::new("t".into());
    r.error("e");
    r.warning("w");
    assert_eq!(r.issues.len(), 2);
}
#[test]
fn report_is_valid_after_only_warnings() {
    let mut r = ValidationReport::new("t".into());
    r.warning("w");
    assert!(r.is_valid());
}
#[test]
fn report_has_errors_false_initial() {
    let r = ValidationReport::new("t".into());
    assert!(!r.has_errors());
}
#[test]
fn report_has_errors_true() {
    let mut r = ValidationReport::new("t".into());
    r.error("e");
    assert!(r.has_errors());
}
#[test]
fn report_variables_len() {
    let mut r = ValidationReport::new("t".into());
    r.variables_referenced.insert("x".into());
    r.variables_referenced.insert("y".into());
    assert_eq!(r.variables_referenced.len(), 2);
}
#[test]
fn report_tasks_len() {
    let mut r = ValidationReport::new("t".into());
    r.tasks_called.insert("a".into());
    r.tasks_called.insert("b".into());
    assert_eq!(r.tasks_called.len(), 2);
}
#[test]
fn issue_message_empty() {
    let e = ValidationIssue::Error("".into());
    assert_eq!(e.message(), "");
}
#[test]
fn report_name_change() {
    let mut r = ValidationReport::new("old".into());
    r.task_name = "new".into();
    assert_eq!(r.task_name, "new");
}
#[test]
fn report_summary_contains_task_name() {
    let r = ValidationReport::new("special".into());
    assert!(r.summary().contains("special"));
}
#[test]
fn report_multiple_errors_summary() {
    let mut r = ValidationReport::new("t".into());
    r.error("e1");
    r.error("e2");
    assert!(r.summary().contains("2 error(s)"));
}
#[test]
fn report_action_and_error_mix() {
    let mut r = ValidationReport::new("t".into());
    r.action_count = 5;
    r.error("e");
    assert!(r.summary().contains("5 actions"));
}
#[test]
fn issue_clone() {
    let e = ValidationIssue::Error("c".into());
    let c = e.clone();
    assert_eq!(e, c);
}
#[test]
fn report_default_action_zero() {
    let r = ValidationReport::new("t".into());
    assert_eq!(r.action_count, 0);
}
#[test]
fn report_issues_push_direct() {
    let mut r = ValidationReport::new("t".into());
    r.issues.push(ValidationIssue::Error("p".into()));
    assert_eq!(r.issues.len(), 1);
}
#[test]
fn report_variables_clear() {
    let mut r = ValidationReport::new("t".into());
    r.variables_referenced.insert("v".into());
    r.variables_referenced.clear();
    assert!(r.variables_referenced.is_empty());
}
#[test]
fn report_tasks_clear() {
    let mut r = ValidationReport::new("t".into());
    r.tasks_called.insert("t".into());
    r.tasks_called.clear();
    assert!(r.tasks_called.is_empty());
}
#[test]
fn issue_eq_self() {
    let e = ValidationIssue::Error("s".into());
    assert_eq!(e, e);
}
#[test]
fn report_eq_name_only() {
    let r1 = ValidationReport::new("same".into());
    let r2 = ValidationReport::new("same".into());
    assert_eq!(r1.task_name, r2.task_name);
}
#[test]
fn report_summary_no_warnings_errors() {
    let r = ValidationReport::new("t".into());
    let s = r.summary();
    assert!(!s.contains("error"));
    assert!(!s.contains("warning"));
}
#[test]
fn report_error_count_zero() {
    let r = ValidationReport::new("t".into());
    assert_eq!(r.error_count(), 0);
}
#[test]
fn report_warning_count_zero() {
    let r = ValidationReport::new("t".into());
    assert_eq!(r.warning_count(), 0);
}
#[test]
fn report_issues_retain_errors() {
    let mut r = ValidationReport::new("t".into());
    r.error("e");
    r.warning("w");
    r.issues.retain(|i| i.is_error());
    assert_eq!(r.issues.len(), 1);
}
#[test]
fn issue_message_unicode() {
    let e = ValidationIssue::Error(" café ".into());
    assert!(e.message().contains("café"));
}
#[test]
fn report_tasks_contains_after_insert() {
    let mut r = ValidationReport::new("t".into());
    r.tasks_called.insert("subtask".into());
    assert!(r.tasks_called.contains("subtask"));
}
#[test]
fn report_variables_contains() {
    let mut r = ValidationReport::new("t".into());
    r.variables_referenced.insert("varX".into());
    assert!(r.variables_referenced.contains("varX"));
}
#[test]
fn report_summary_format_check() {
    let mut r = ValidationReport::new("fmt".into());
    r.error("e");
    let s = r.summary();
    assert!(s.starts_with("Task 'fmt'"));
}
#[test]
fn report_new_with_special_chars() {
    let r = ValidationReport::new("t@#$".into());
    assert_eq!(r.task_name, "t@#$");
}
#[test]
fn issue_warning_eq() {
    let w1 = ValidationIssue::Warning("w".into());
    let w2 = ValidationIssue::Warning("w".into());
    assert_eq!(w1, w2);
}
#[test]
fn report_issues_iter_errors() {
    let mut r = ValidationReport::new("t".into());
    r.error("e1");
    r.error("e2");
    let errs: Vec<_> = r.issues.iter().filter(|i| i.is_error()).collect();
    assert_eq!(errs.len(), 2);
}
#[test]
fn report_action_increment() {
    let mut r = ValidationReport::new("t".into());
    r.action_count += 1;
    assert_eq!(r.action_count, 1);
}
#[test]
fn report_variables_insert_many() {
    let mut r = ValidationReport::new("t".into());
    ["a", "b", "c"].iter().for_each(|v| {
        r.variables_referenced.insert(v.to_string());
    });
    assert_eq!(r.variables_referenced.len(), 3);
}
#[test]
fn report_debug_not_empty() {
    let r = ValidationReport::new("d".into());
    assert!(!format!("{:?}", r).is_empty());
}
#[test]
fn issue_debug_not_empty() {
    let e = ValidationIssue::Error("d".into());
    assert!(!format!("{:?}", e).is_empty());
}
#[test]
fn report_has_no_warnings_initial() {
    let r = ValidationReport::new("t".into());
    assert_eq!(r.warning_count(), 0);
}

#[test]
fn validate_task_empty_name_errors() {
    let def = TaskDefinition {
        name: String::new(),
        description: String::new(),
        policy: String::new(),
        actions: vec![Action::Wait { duration_ms: 1 }],
        include: Vec::new(),
        parameters: HashMap::new(),
    };
    let report = TaskValidator::new().validate(&def);
    assert!(report.has_errors());
    assert!(report
        .issues
        .iter()
        .any(|i: &ValidationIssue| { i.message().contains("Task name cannot be empty") }));
}

#[test]
fn validate_task_name_with_spaces_errors() {
    let def = TaskDefinition {
        name: "bad name".into(),
        description: String::new(),
        policy: String::new(),
        actions: vec![Action::Wait { duration_ms: 1 }],
        include: Vec::new(),
        parameters: HashMap::new(),
    };
    let report = TaskValidator::new().validate(&def);
    assert!(report
        .issues
        .iter()
        .any(|i: &ValidationIssue| { i.message().contains("cannot contain spaces") }));
}

#[test]
fn validate_task_empty_actions_and_includes_errors() {
    let def = TaskDefinition {
        name: "ok".into(),
        description: String::new(),
        policy: String::new(),
        actions: Vec::new(),
        include: Vec::new(),
        parameters: HashMap::new(),
    };
    let report = TaskValidator::new().validate(&def);
    assert!(report
        .issues
        .iter()
        .any(|i: &ValidationIssue| { i.message().contains("at least one action or include") }));
}

#[test]
fn validate_parameter_redundant_required_default_warns() {
    use crate::task::dsl::types::ParameterType;
    let mut params = HashMap::new();
    params.insert(
        "p".into(),
        ParameterDef {
            required: true,
            default: Some(serde_yml::Value::String("x".into())),
            description: String::new(),
            r#type: ParameterType::String,
        },
    );
    let def = TaskDefinition {
        name: "t".into(),
        description: String::new(),
        policy: String::new(),
        actions: vec![Action::Wait { duration_ms: 1 }],
        include: Vec::new(),
        parameters: params,
    };
    let report = TaskValidator::new().validate(&def);
    assert!(report
        .issues
        .iter()
        .any(|i: &ValidationIssue| { i.message().contains("is required but has a default") }));
}

#[test]
fn validate_optional_parameter_without_default_warns() {
    use crate::task::dsl::types::ParameterType;
    let mut params = HashMap::new();
    params.insert(
        "p".into(),
        ParameterDef {
            required: false,
            default: None,
            description: String::new(),
            r#type: ParameterType::String,
        },
    );
    let def = TaskDefinition {
        name: "t".into(),
        description: String::new(),
        policy: String::new(),
        actions: vec![Action::Wait { duration_ms: 1 }],
        include: Vec::new(),
        parameters: params,
    };
    let report = TaskValidator::new().validate(&def);
    assert!(report
        .issues
        .iter()
        .any(|i: &ValidationIssue| { i.message().contains("optional but has no default") }));
}

#[test]
fn validate_extract_empty_variable_errors() {
    let def = TaskDefinition {
        name: "t".into(),
        description: String::new(),
        policy: String::new(),
        actions: vec![Action::Extract {
            selector: "#x".into(),
            variable: Some(String::new()),
        }],
        include: Vec::new(),
        parameters: HashMap::new(),
    };
    let report = TaskValidator::new().validate(&def);
    assert!(report
        .issues
        .iter()
        .any(|i: &ValidationIssue| { i.message().contains("Variable name cannot be empty") }));
}

#[test]
fn validate_loop_no_count_or_condition_errors() {
    let def = TaskDefinition {
        name: "t".into(),
        description: String::new(),
        policy: String::new(),
        actions: vec![Action::Loop {
            count: None,
            condition: None,
            actions: vec![],
        }],
        include: Vec::new(),
        parameters: HashMap::new(),
    };
    let report = TaskValidator::new().validate(&def);
    assert!(report.issues.iter().any(|i: &ValidationIssue| {
        i.message()
            .contains("Loop must have either 'count' or 'condition'")
    }));
}

#[test]
fn validate_call_empty_task_name_errors() {
    let def = TaskDefinition {
        name: "t".into(),
        description: String::new(),
        policy: String::new(),
        actions: vec![Action::Call {
            task: String::new(),
            parameters: None,
        }],
        include: Vec::new(),
        parameters: HashMap::new(),
    };
    let report = TaskValidator::new().validate(&def);
    assert!(report
        .issues
        .iter()
        .any(|i: &ValidationIssue| { i.message().contains("Task name cannot be empty") }));
}

#[test]
fn validate_call_self_circular_reference_errors() {
    let def = TaskDefinition {
        name: "self".into(),
        description: String::new(),
        policy: String::new(),
        actions: vec![Action::Call {
            task: "self".into(),
            parameters: None,
        }],
        include: Vec::new(),
        parameters: HashMap::new(),
    };
    let report = TaskValidator::new()
        .with_current_task("self")
        .validate(&def);
    assert!(report
        .issues
        .iter()
        .any(|i: &ValidationIssue| { i.message().contains("circular reference") }));
}

#[test]
fn validate_parallel_zero_concurrency_errors() {
    let def = TaskDefinition {
        name: "t".into(),
        description: String::new(),
        policy: String::new(),
        actions: vec![Action::Parallel {
            actions: vec![Action::Wait { duration_ms: 1 }],
            max_concurrency: Some(0),
        }],
        include: Vec::new(),
        parameters: HashMap::new(),
    };
    let report = TaskValidator::new().validate(&def);
    assert!(report
        .issues
        .iter()
        .any(|i: &ValidationIssue| { i.message().contains("max_concurrency cannot be 0") }));
}

#[test]
fn validate_wait_zero_duration_warns() {
    let def = TaskDefinition {
        name: "t".into(),
        description: String::new(),
        policy: String::new(),
        actions: vec![Action::Wait { duration_ms: 0 }],
        include: Vec::new(),
        parameters: HashMap::new(),
    };
    let report = TaskValidator::new().validate(&def);
    assert!(report
        .issues
        .iter()
        .any(|i: &ValidationIssue| { i.message().contains("Wait duration is 0ms (no-op)") }));
}

#[test]
fn extract_variables_updates_report() {
    let mut r = ValidationReport::new("t".into());
    let validator = TaskValidator::new();
    validator.extract_variables("hello ${name} world", &mut r);
    assert!(r.variables_referenced.contains("name"));
}

#[test]
fn count_actions_returns_action_count() {
    let def = TaskDefinition {
        name: "t".into(),
        description: String::new(),
        policy: String::new(),
        actions: vec![
            Action::Wait { duration_ms: 1 },
            Action::Wait { duration_ms: 2 },
        ],
        include: Vec::new(),
        parameters: HashMap::new(),
    };
    let report = TaskValidator::new().validate(&def);
    assert_eq!(report.action_count, 2);
}

#[test]
fn validator_unknown_task_warns() {
    let tasks = vec!["known".to_string()];
    let def = TaskDefinition {
        name: "t".into(),
        description: String::new(),
        policy: String::new(),
        actions: vec![Action::Call {
            task: "unknown".into(),
            parameters: None,
        }],
        include: Vec::new(),
        parameters: HashMap::new(),
    };
    let report = TaskValidator::new().with_known_tasks(tasks).validate(&def);
    assert!(report
        .issues
        .iter()
        .any(|i: &ValidationIssue| { i.message().contains("not in the known task list") }));
}
