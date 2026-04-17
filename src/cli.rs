//! Command-line interface and task parsing module.
//!
//! This module handles:
//! - Command-line argument parsing using clap
//! - Task group parsing (handling "then" separators)
//! - Task definition structures
//! - Formatting task groups for display and logging

use clap::Parser;
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(name = "rust-orchestrator")]
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

    /// Comma-separated list of browser types to connect to
    #[arg(long)]
    pub browsers: Option<String>,
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

/// Represents a single task with its name and payload
#[derive(Debug, Clone)]
pub struct TaskDefinition {
    pub name: String,
    pub payload: HashMap<String, String>,
}

/// Parse CLI args into task groups for sequential execution
/// Mirrors the Node.js task-parser.js logic
pub fn parse_task_groups(task_args: &[String]) -> Vec<Vec<TaskDefinition>> {
    let mut groups: Vec<Vec<TaskDefinition>> = Vec::new();
    let mut current_group: Vec<TaskDefinition> = Vec::new();
    let mut current_task: Option<String> = None;
    let mut current_payload: HashMap<String, String> = HashMap::new();

    if task_args.is_empty() {
        return vec![];
    }

    for arg in task_args {
        let normalized = arg.to_lowercase();

        // Check for task separator
        if normalized == "then" {
            if let Some(task_name) = current_task.take() {
                current_group.push(TaskDefinition {
                    name: task_name,
                    payload: std::mem::take(&mut current_payload),
                });
            }
            if !current_group.is_empty() {
                groups.push(std::mem::take(&mut current_group));
            }
            continue;
        }

        // Parse key=value or standalone task name
        if let Some(eq_pos) = arg.find('=') {
            let key = &arg[..eq_pos];
            let mut value = &arg[eq_pos + 1..];

            // Handle quoted values
            if value.starts_with('"') && value.ends_with('"') {
                value = &value[1..value.len() - 1];
            }

            // Push any current task first
            if let Some(task_name) = current_task.take() {
                current_group.push(TaskDefinition {
                    name: task_name,
                    payload: std::mem::take(&mut current_payload),
                });
            }

            // Remove .js extension from task name if present
            let task_name = key.strip_suffix(".js").unwrap_or(key).to_string();

            // Start new task
            current_task = Some(task_name);

            // Handle different parameter types
            if key == "url" {
                // Explicit url parameter
                current_payload.insert("url".to_string(), format_url(value));
            } else if is_numeric(value) {
                current_payload.insert("value".to_string(), value.to_string());
            } else if let Some(eq_pos) = value.find('=') {
                // Value contains '=', parse as param=value
                let param_key = &value[..eq_pos];
                let param_value = &value[eq_pos + 1..];
                if param_key == "url" {
                    current_payload.insert("url".to_string(), format_url(param_value));
                } else {
                    current_payload.insert(param_key.to_string(), param_value.to_string());
                }
            } else if looks_like_url(value) {
                // Looks like a URL
                current_payload.insert("url".to_string(), format_url(value));
            } else {
                // Plain parameter value
                current_payload.insert(key.to_string(), value.to_string());
            }
        } else {
            // Standalone argument (task name without =)
            if let Some(task_name) = current_task.take() {
                current_group.push(TaskDefinition {
                    name: task_name,
                    payload: std::mem::take(&mut current_payload),
                });
            }
            let task_name = arg.strip_suffix(".js").unwrap_or(arg);
            current_task = Some(task_name.to_string());
        }
    }

    // Push remaining task
    if let Some(task_name) = current_task.take() {
        current_group.push(TaskDefinition {
            name: task_name,
            payload: current_payload,
        });
    }

    if !current_group.is_empty() {
        groups.push(current_group);
    }

    groups
}

fn is_numeric(value: &str) -> bool {
    value.chars().all(|c| c.is_ascii_digit()) && !value.is_empty()
}

fn looks_like_url(value: &str) -> bool {
    let trimmed = value.trim();

    // Already has protocol
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return true;
    }

    // Contains dot or is localhost (same logic as format_url)
    let before_port = trimmed.split(':').next().unwrap_or(trimmed);
    trimmed.contains('.') || before_port == "localhost"
}

fn format_url(value: &str) -> String {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return trimmed.to_string();
    }

    // Already has protocol
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }

    // Contains dot or is localhost → treat as URL, prepend https://
    let before_port = trimmed.split(':').next().unwrap_or(trimmed);
    if trimmed.contains('.') || before_port == "localhost" {
        return format!("https://{trimmed}");
    }

    // Not a URL, return as-is
    trimmed.to_string()
}

/// Formats parsed task groups into a human-readable string for logging.
/// Creates a summary showing the structure of task groups and total task count.
/// Useful for displaying execution plans to users.
///
/// # Arguments
/// * `groups` - Slice of task groups, where each group is a vector of tasks
///
/// # Returns
/// Formatted string describing the task groups (e.g., "3 tasks (1 then 2)")
///
/// # Examples
/// ```
/// # use rust_orchestrator::cli::{format_task_groups, TaskDefinition};
/// # use std::collections::HashMap;
/// let groups = vec![
///     vec![TaskDefinition { name: "cookiebot".to_string(), payload: HashMap::new() }],
///     vec![
///         TaskDefinition { name: "pageview".to_string(), payload: HashMap::new() },
///         TaskDefinition { name: "pageview".to_string(), payload: HashMap::new() }
///     ]
/// ];
/// assert_eq!(format_task_groups(&groups), "3 task(s) [Group 1: cookiebot | Group 2: pageview, pageview]");
/// ```
pub fn format_task_groups(groups: &[Vec<TaskDefinition>]) -> String {
    let total: usize = groups.iter().map(Vec::len).sum();

    if total == 0 {
        return "No tasks".to_string();
    }

    let parts: Vec<String> = groups
        .iter()
        .enumerate()
        .map(|(i, group)| {
            let names: Vec<&str> = group.iter().map(|t| t.name.as_str()).collect();
            if groups.len() > 1 {
                format!("Group {}: {}", i + 1, names.join(", "))
            } else {
                names.join(", ")
            }
        })
        .collect();

    format!("{} task(s) [{}]", total, parts.join(" | "))
}

#[cfg(test)]
mod tests {
    use super::*;

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
            Some(&"https://www.reddit.com".to_string())
        );
    }

    #[test]
    fn test_parse_task_groups_with_explicit_url() {
        let args = vec!["pageview=url=https://example.com".to_string()];
        let result = parse_task_groups(&args);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0][0].name, "pageview");
        // Should parse url=https://example.com as url parameter
        assert_eq!(
            result[0][0].payload.get("url"),
            Some(&"https://example.com".to_string())
        );
    }

    #[test]
    fn test_parse_task_groups_multiple_tasks_same_group() {
        let args = vec!["cookiebot".to_string(), "pageview=reddit.com".to_string()];
        let result = parse_task_groups(&args);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 2);
        assert_eq!(result[0][0].name, "cookiebot");
        assert_eq!(result[0][1].name, "pageview");
        assert_eq!(
            result[0][1].payload.get("url"),
            Some(&"https://reddit.com".to_string())
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
            Some(&"https://reddit.com".to_string())
        );
    }

    #[test]
    fn test_parse_task_groups_smoke_test() {
        // Test the exact smoke test pattern: cookiebot pageview=www.reddit.com then cookiebot
        let args = vec![
            "cookiebot".to_string(),
            "pageview=www.reddit.com".to_string(),
            "then".to_string(),
            "cookiebot".to_string(),
        ];
        let result = parse_task_groups(&args);

        assert_eq!(result.len(), 2);

        // Group 1: cookiebot, pageview
        assert_eq!(result[0].len(), 2);
        assert_eq!(result[0][0].name, "cookiebot");
        assert_eq!(result[0][1].name, "pageview");
        assert_eq!(
            result[0][1].payload.get("url"),
            Some(&"https://www.reddit.com".to_string())
        );

        // Group 2: cookiebot
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
        assert_eq!(result[0][0].payload.get("value"), Some(&"42".to_string()));
    }

    #[test]
    fn test_parse_task_groups_with_spaces() {
        let args = vec!["task=value with spaces".to_string()];
        let result = parse_task_groups(&args);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0][0].name, "task");
        // Spaces in values are treated as plain parameters
        assert_eq!(
            result[0][0].payload.get("task"),
            Some(&"value with spaces".to_string())
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
                    payload: [("url".to_string(), "https://reddit.com".to_string())].into(),
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
}
