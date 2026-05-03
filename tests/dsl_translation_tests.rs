use std::collections::HashMap;
use std::fs;

use tempfile::TempDir;

use auto::task::dsl::{
    validate_task_definition, Action, Condition, LogLevel, ParameterDef, ParameterType,
    TaskDefinition,
};

fn valid_task_def(actions: Vec<Action>) -> TaskDefinition {
    TaskDefinition {
        name: "dsl_task".to_string(),
        description: "dsl coverage".to_string(),
        policy: "default".to_string(),
        parameters: HashMap::new(),
        actions,
    }
}

#[test]
fn parse_task_file_falls_back_to_yaml_for_unknown_extension() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("sample.task");
    let yaml = r#"
name: sample_task
description: "yaml fallback"
actions:
  - action: wait
    duration_ms: 25
"#;
    fs::write(&path, yaml).unwrap();

    let def = auto::task::dsl::parse_task_file(&path).unwrap();

    assert_eq!(def.name, "sample_task");
    assert_eq!(def.description, "yaml fallback");
    assert_eq!(def.actions.len(), 1);
}

#[test]
fn parse_task_file_falls_back_to_toml_for_unknown_extension() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("sample.task");
    let toml = r#"
name = "sample_task"
description = "toml fallback"

[[actions]]
action = "wait"
duration_ms = 25
"#;
    fs::write(&path, toml).unwrap();

    let def = auto::task::dsl::parse_task_file(&path).unwrap();

    assert_eq!(def.name, "sample_task");
    assert_eq!(def.description, "toml fallback");
    assert_eq!(def.actions.len(), 1);
}

#[test]
fn parse_task_yaml_supports_nested_control_flow_actions() {
    let yaml = r##"
name: nested_task
description: "nested flow"
policy: default
actions:
  - action: if
    condition:
      type: element_exists
      selector: "#primary"
    then:
      - action: loop
        count: 2
        actions:
          - action: log
            message: "inside loop"
            level: info
    else:
      - action: call
        task: cookiebot
  - action: execute
    script: "console.log('done')"
"##;

    let def = auto::task::dsl::parse_task_yaml(yaml).unwrap();

    assert_eq!(def.name, "nested_task");
    assert_eq!(def.actions.len(), 2);

    match &def.actions[0] {
        Action::If {
            condition,
            then,
            r#else,
        } => {
            assert!(matches!(
                condition,
                Condition::ElementExists { selector } if selector == "#primary"
            ));
            assert_eq!(then.len(), 1);
            assert!(r#else.is_some());
        }
        other => panic!("expected If action, got {:?}", other),
    }
}

#[test]
fn validate_task_definition_reports_nested_control_flow_errors() {
    let def = valid_task_def(vec![
        Action::If {
            condition: Condition::ElementVisible {
                selector: "#root".to_string(),
            },
            then: vec![],
            r#else: Some(vec![]),
        },
        Action::Loop {
            count: None,
            condition: None,
            actions: vec![],
        },
    ]);

    let err = validate_task_definition(&def).unwrap_err();

    assert!(err
        .iter()
        .any(|msg| msg.contains("'if' block has empty 'then' branch")));
    assert!(err
        .iter()
        .any(|msg| msg.contains("'if' block has empty 'else' branch")));
    assert!(err
        .iter()
        .any(|msg| msg.contains("'loop' must have 'count' or 'condition'")));
    assert!(err
        .iter()
        .any(|msg| msg.contains("'loop' block has no actions")));
}

#[test]
fn format_task_definition_renders_parameters_and_actions() {
    let mut parameters = HashMap::new();
    parameters.insert(
        "target_url".to_string(),
        ParameterDef {
            r#type: ParameterType::Url,
            description: "Target URL".to_string(),
            default: None,
            required: true,
        },
    );

    let def = TaskDefinition {
        name: "format_task".to_string(),
        description: "format coverage".to_string(),
        policy: "default".to_string(),
        parameters,
        actions: vec![
            Action::Navigate {
                url: "https://example.com".to_string(),
            },
            Action::Log {
                message: "hello".to_string(),
                level: Some(LogLevel::Info),
            },
        ],
    };

    let output = auto::task::dsl::format_task_definition(&def);

    assert!(output.contains("Task: format_task"));
    assert!(output.contains("Policy: default"));
    assert!(output.contains("Description: format coverage"));
    assert!(output.contains("Parameters: 1"));
    assert!(output.contains("Actions: 2"));
    assert!(output.contains("Navigate"));
    assert!(output.contains("Log"));
}
