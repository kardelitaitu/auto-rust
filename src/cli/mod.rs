//! Command-line interface and task parsing module.
//!
//! This module handles:
//! - Command-line argument parsing using clap
//! - Task group parsing (handling "then" separators)
//! - Task definition structures
//! - Formatting task groups for display and logging
//!
//! Task validation is handled by the `validation` module.

use clap::Parser;

// Re-export parser functions and types
pub mod parser;
pub use parser::{format_task_groups, parse_browser_filters, parse_task_groups, TaskDefinition};

// Re-export validation functions from validation layer
pub use crate::validation::{
    is_known_task, validate_task_groups, validate_task_groups_strict,
    validate_task_name as validate_task, TaskValidationResult,
};

#[derive(Parser, Debug)]
#[command(name = "auto")]
#[command(about = "Multi-browser automation orchestrator")]
pub struct Args {
    /// Tasks to run, separated by 'then' for sequential groups
    /// Examples:
    ///   cargo run cookiebot
    ///   cargo run cookiebot pageview=www.reddit.com
    ///   cargo run cookiebot then pageview
    ///   cargo run cookiebot.js pageview.js then cookiebot
    #[arg(required = false)]
    pub tasks: Vec<String>,

    /// Comma-separated list of browser names or types to connect to
    #[arg(long)]
    pub browsers: Option<String>,

    /// Clear all click learning data before starting
    #[arg(long, help = "Clear all click learning data and exit")]
    pub clear_learning: bool,

    /// List all available tasks and exit
    #[arg(
        long,
        help = "List all available tasks with source and policy information"
    )]
    pub list_tasks: bool,

    /// Show help for a specific task
    #[arg(
        long,
        help = "Show payload guidance for a specific task",
        value_name = "TASK"
    )]
    pub help_task: Option<String>,

    /// Simulate execution without running tasks
    #[arg(
        long,
        help = "Show what would be executed without actually running tasks"
    )]
    pub dry_run: bool,

    /// Validate all external tasks and exit
    #[arg(long, help = "Validate all external task files without executing them")]
    pub validate_tasks: bool,

    /// Watch external task directories for changes and auto-reload
    #[arg(
        long,
        help = "Watch external task directories for changes and auto-reload tasks"
    )]
    pub watch: bool,
}

/// Parses command-line arguments using clap.
/// Uses the Args struct definition to automatically parse and validate
/// command-line input. Exits with an error message if parsing fails.
///
/// # Returns
/// Parsed command-line arguments as an Args struct
pub fn parse_args() -> Args {
    Args::parse()
}

/// Get help text for a specific task.
///
/// Returns a description of the task and its expected payload structure,
/// sourced from the task registry metadata.
///
/// # Arguments
/// * `task_name` - The name of the task to get help for
///
/// # Returns
/// Option containing the help text if the task is known
pub fn get_task_help(task_name: &str) -> Option<String> {
    use crate::task::registry::{TaskRegistry, TaskSource};

    let registry = TaskRegistry::with_built_in_tasks();
    let descriptor = registry.lookup(task_name).ok()?;

    let source_info = match &descriptor.source {
        TaskSource::BuiltInRust => "built-in task".to_string(),
        TaskSource::ConfiguredPath(p) => format!("external task: {}", p.display()),
        TaskSource::Unknown => "unknown source".to_string(),
    };

    let policy_info = format!("policy: {}", descriptor.policy_name);

    let payload_guidance = get_payload_guidance(task_name);

    Some(format!(
        "Task: {}\nSource: {}\n{}\n\n{}",
        descriptor.name, source_info, policy_info, payload_guidance
    ))
}

/// Get payload guidance for a specific task.
///
/// Returns expected payload structure and examples for the task.
/// This is sourced from registry knowledge rather than hardcoded.
fn get_payload_guidance(task_name: &str) -> String {
    use crate::validation::task as task_validation;

    // Get validation info for the task
    let validation_info = task_validation::get_task_validation_info(task_name);

    match validation_info {
        Some(info) => format_payload_guidance(task_name, &info),
        None => format!(
            "Payload: Object with task-specific parameters\n\nExamples:\n  {}={{}}\n  {}={{\"key\": \"value\"}}",
            task_name, task_name
        ),
    }
}

/// Format payload guidance from validation info
fn format_payload_guidance(
    task_name: &str,
    info: &crate::validation::task::TaskValidationInfo,
) -> String {
    let mut lines = vec![
        format!("Payload: {}", info.description),
        String::new(),
        "Examples:".to_string(),
    ];

    for example in &info.examples {
        lines.push(format!("  {}", example));
    }

    if info.required_fields.is_empty() && info.optional_fields.is_empty() {
        lines.push(format!("  {}={{}}", task_name));
    }

    if !info.required_fields.is_empty() {
        lines.push(String::new());
        lines.push("Required fields:".to_string());
        for field in &info.required_fields {
            lines.push(format!("  - {}", field));
        }
    }

    if !info.optional_fields.is_empty() {
        lines.push(String::new());
        lines.push("Optional fields:".to_string());
        for field in &info.optional_fields {
            lines.push(format!("  - {}", field));
        }
    }

    lines.join("\n")
}

/// Render help output for a task.
///
/// Returns formatted help text suitable for CLI display.
/// If the task is not found, returns an error message.
pub fn render_task_help(task_name: &str) -> String {
    match get_task_help(task_name) {
        Some(help) => help,
        None => format!(
            "Unknown task: '{}'\nRun with --list-tasks to see available tasks.",
            task_name
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use serde_json::json;

    #[test]
    fn test_parse_task_groups_empty() {
        let result = parse_task_groups(&[]);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_task_groups_single_task() {
        let args = vec!["cookiebot".to_string()];
        let result = parse_task_groups(&args);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0][0].name, "cookiebot");
        assert!(result[0][0].payload.is_empty());
    }

    #[test]
    fn test_parse_task_groups_with_js_extension() {
        let args = vec!["cookiebot.js".to_string()];
        let result = parse_task_groups(&args);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0][0].name, "cookiebot");
    }

    #[test]
    fn test_parse_task_groups_with_url() {
        let args = vec!["pageview=www.reddit.com".to_string()];
        let result = parse_task_groups(&args);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0][0].name, "pageview");
        assert_eq!(
            result[0][0].payload.get("url"),
            Some(&json!("https://www.reddit.com"))
        );
    }

    #[test]
    fn test_parse_task_groups_with_explicit_url() {
        let args = vec!["pageview=url=https://example.com".to_string()];
        let result = parse_task_groups(&args);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0][0].name, "pageview");
        assert_eq!(
            result[0][0].payload.get("url"),
            Some(&json!("https://example.com"))
        );
    }

    #[test]
    fn test_parse_task_groups_multiple_tasks_same_group() {
        let args = vec!["cookiebot".to_string(), "pageview=reddit.com".to_string()];
        let result = parse_task_groups(&args);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0][0].name, "cookiebot");
        assert_eq!(
            result[0][0].payload.get("pageview"),
            Some(&json!("reddit.com"))
        );
    }

    #[test]
    fn test_parse_task_groups_with_then_separator() {
        let args = vec![
            "cookiebot".to_string(),
            "then".to_string(),
            "pageview=reddit.com".to_string(),
        ];
        let result = parse_task_groups(&args);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0][0].name, "cookiebot");
        assert_eq!(result[1].len(), 1);
        assert_eq!(result[1][0].name, "pageview");
        assert_eq!(
            result[1][0].payload.get("url"),
            Some(&json!("https://reddit.com"))
        );
    }

    #[test]
    fn test_parse_task_groups_smoke_test() {
        let args = vec![
            "cookiebot".to_string(),
            "pageview=www.reddit.com".to_string(),
            "then".to_string(),
            "cookiebot".to_string(),
        ];
        let result = parse_task_groups(&args);

        assert_eq!(result.len(), 2);

        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0][0].name, "cookiebot");
        assert_eq!(
            result[0][0].payload.get("pageview"),
            Some(&json!("www.reddit.com"))
        );

        assert_eq!(result[1].len(), 1);
        assert_eq!(result[1][0].name, "cookiebot");
    }

    #[test]
    fn test_parse_task_groups_with_numeric_value() {
        let args = vec!["taskname=42".to_string()];
        let result = parse_task_groups(&args);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0][0].name, "taskname");
        assert_eq!(result[0][0].payload.get("value"), Some(&json!(42)));
    }

    #[test]
    fn test_parse_task_groups_with_spaces() {
        let args = vec!["task=value with spaces".to_string()];
        let result = parse_task_groups(&args);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0][0].name, "task");
        assert_eq!(
            result[0][0].payload.get("url"),
            Some(&json!("value with spaces"))
        );
    }

    #[test]
    fn test_format_task_groups() {
        let groups = vec![
            vec![
                TaskDefinition {
                    name: "cookiebot".to_string(),
                    payload: std::collections::HashMap::new(),
                },
                TaskDefinition {
                    name: "pageview".to_string(),
                    payload: [("url".to_string(), json!("https://reddit.com"))].into(),
                },
            ],
            vec![TaskDefinition {
                name: "cookiebot".to_string(),
                payload: std::collections::HashMap::new(),
            }],
        ];

        let formatted = format_task_groups(&groups);
        assert_eq!(
            formatted,
            "3 task(s) [Group 1: cookiebot, pageview | Group 2: cookiebot]"
        );
    }

    #[test]
    fn test_parse_browser_filters() {
        let filters = parse_browser_filters(Some(" Brave , roxybrowser, brave "));
        assert_eq!(
            filters,
            vec!["brave".to_string(), "roxybrowser".to_string()]
        );
    }

    #[test]
    fn test_args_parse_list_tasks_flag() {
        let args = Args::try_parse_from(["auto", "--list-tasks"]).unwrap();

        assert!(args.list_tasks);
        assert!(!args.dry_run);
        assert!(args.tasks.is_empty());
        assert!(args.browsers.is_none());
        assert!(args.help_task.is_none());
    }

    #[test]
    fn test_args_parse_dry_run_flag() {
        let args = Args::try_parse_from(["auto", "--dry-run"]).unwrap();

        assert!(args.dry_run);
        assert!(!args.list_tasks);
        assert!(args.tasks.is_empty());
        assert!(args.browsers.is_none());
        assert!(args.help_task.is_none());
    }

    #[test]
    fn test_args_parse_list_tasks_and_dry_run_flags() {
        let args = Args::try_parse_from(["auto", "--list-tasks", "--dry-run"]).unwrap();

        assert!(args.list_tasks);
        assert!(args.dry_run);
        assert!(args.tasks.is_empty());
        assert!(args.browsers.is_none());
        assert!(args.help_task.is_none());
    }

    #[test]
    fn test_args_parse_positional_tasks_with_flags() {
        let args =
            Args::try_parse_from(["auto", "cookiebot", "pageview=www.example.com", "--dry-run"])
                .unwrap();

        assert!(args.dry_run);
        assert!(!args.list_tasks);
        assert_eq!(
            args.tasks,
            vec![
                "cookiebot".to_string(),
                "pageview=www.example.com".to_string()
            ]
        );
        assert!(args.help_task.is_none());
    }

    #[test]
    fn test_args_parse_help_task_flag() {
        let args = Args::try_parse_from(["auto", "--help-task", "cookiebot"]).unwrap();

        assert_eq!(args.help_task, Some("cookiebot".to_string()));
        assert!(!args.list_tasks);
        assert!(!args.dry_run);
        assert!(args.tasks.is_empty());
    }

    #[test]
    fn test_get_task_help_known_task() {
        let help = get_task_help("cookiebot");
        assert!(help.is_some());
        let help_text = help.unwrap();
        assert!(help_text.contains("Task: cookiebot"));
        assert!(help_text.contains("built-in task"));
        assert!(help_text.contains("policy:"));
    }

    #[test]
    fn test_get_task_help_unknown_task() {
        let help = get_task_help("nonexistent_task_xyz");
        assert!(help.is_none());
    }

    #[test]
    fn test_render_task_help_known_task() {
        let help_text = render_task_help("pageview");
        assert!(help_text.contains("Task: pageview"));
        assert!(help_text.contains("built-in task"));
    }

    #[test]
    fn test_render_task_help_unknown_task() {
        let help_text = render_task_help("unknown_task_12345");
        assert!(help_text.contains("Unknown task:"));
        assert!(help_text.contains("Run with --list-tasks"));
    }
}
