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
use serde_json::Value;
use std::collections::{HashMap, HashSet};

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
    pub payload: HashMap<String, Value>,
}

/// Parse the `--browsers` CLI filter into normalized tokens.
///
/// Normalizes browser names by converting to lowercase and removing duplicates.
/// Used to filter which browser sessions should receive tasks.
///
/// # Arguments
/// * `value` - Comma-separated browser names from CLI (e.g., "chrome,firefox")
///
/// # Returns
/// Vector of normalized browser name tokens
///
/// # Examples
/// ```
/// use auto::cli::parse_browser_filters;
///
/// // Parse multiple browsers
/// let filters = parse_browser_filters(Some("chrome,firefox,chrome"));
/// assert_eq!(filters, vec!["chrome", "firefox"]);
///
/// // Empty filter matches all browsers
/// let all = parse_browser_filters(None);
/// assert!(all.is_empty());
/// ```
pub fn parse_browser_filters(value: Option<&str>) -> Vec<String> {
    let mut seen = HashSet::new();

    value
        .unwrap_or("")
        .split(',')
        .map(|item| item.trim().to_lowercase())
        .filter(|item| !item.is_empty())
        .filter(|item| seen.insert(item.clone()))
        .collect()
}

/// Parse CLI args into task groups for sequential execution.
///
/// Mirrors the Node.js task-parser.js logic, handling complex CLI patterns:
/// - Tasks with parameters: `task1 key=value`
/// - Sequential groups: `task1 then task2`
/// - Multiple tasks per group: `task1 task2 then task3`
///
/// # Arguments
/// * `task_args` - Raw CLI arguments (e.g., from `std::env::args()`)
///
/// # Returns
/// Vector of task groups, where each group is a vector of task definitions.
/// Groups are executed sequentially, tasks within a group run in parallel.
///
/// # Examples
/// ```
/// use auto::cli::parse_task_groups;
///
/// // Single task
/// let groups = parse_task_groups(&["cookiebot".to_string()]);
/// assert_eq!(groups.len(), 1);
/// assert_eq!(groups[0][0].name, "cookiebot");
///
/// // Sequential groups with "then"
/// let groups = parse_task_groups(&[
///     "cookiebot".to_string(),
///     "then".to_string(),
///     "pageview".to_string()
/// ]);
/// assert_eq!(groups.len(), 2);
///
/// // Task with parameters
/// let groups = parse_task_groups(&["pageview=www.example.com".to_string()]);
/// assert_eq!(groups[0][0].name, "pageview");
/// ```
pub fn parse_task_groups(task_args: &[String]) -> Vec<Vec<TaskDefinition>> {
    let mut groups: Vec<Vec<TaskDefinition>> = Vec::new();
    let mut current_group: Vec<TaskDefinition> = Vec::new();
    let mut current_task: Option<String> = None;
    let mut current_payload: HashMap<String, Value> = HashMap::new();

    if task_args.is_empty() {
        return vec![];
    }

    for arg in task_args {
        if arg.is_empty() {
            continue;
        }

        let normalized = arg.to_lowercase();

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

        let first_equal_index = arg.find('=');

        if let Some(eq_pos) = first_equal_index {
            if eq_pos > 0 {
                let key = &arg[..eq_pos];
                let mut value = &arg[eq_pos + 1..];

                if value.starts_with('"') && value.ends_with('"') {
                    value = &value[1..value.len() - 1];
                }

                let shorthand_task_name = crate::task::normalize_task_name(key).to_string();

                if current_task.is_none() {
                    let is_numeric = value.chars().all(|c| c.is_ascii_digit()) && !value.is_empty();
                    current_task = Some(shorthand_task_name);
                    if is_numeric {
                        current_payload.insert("value".to_string(), parse_scalar_value(value));
                    } else if key == "url" {
                        current_payload.insert("url".to_string(), Value::String(format_url(value)));
                    } else if value.contains('=') {
                        if let Some(eq_pos) = value.find('=') {
                            let param_key = &value[..eq_pos];
                            let param_value = &value[eq_pos + 1..];
                            let formatted_value = if param_key == "url" {
                                format_url(param_value)
                            } else {
                                param_value.to_string()
                            };
                            current_payload
                                .insert("url".to_string(), Value::String(formatted_value));
                        }
                    } else {
                        current_payload.insert("url".to_string(), Value::String(format_url(value)));
                    }
                } else if key == current_task.as_ref().expect("current_task should be Some") {
                    if let Some(task_name) = current_task.take() {
                        current_group.push(TaskDefinition {
                            name: task_name,
                            payload: std::mem::take(&mut current_payload),
                        });
                    }
                    let is_numeric = value.chars().all(|c| c.is_ascii_digit()) && !value.is_empty();
                    current_task = Some(shorthand_task_name);
                    if is_numeric {
                        current_payload.insert("value".to_string(), parse_scalar_value(value));
                    } else if key == "url" {
                        current_payload.insert("url".to_string(), Value::String(format_url(value)));
                    } else if value.contains('=') {
                        if let Some(eq_pos) = value.find('=') {
                            let param_key = &value[..eq_pos];
                            let param_value = &value[eq_pos + 1..];
                            let formatted_value = if param_key == "url" {
                                format_url(param_value)
                            } else {
                                param_value.to_string()
                            };
                            current_payload
                                .insert("url".to_string(), Value::String(formatted_value));
                        }
                    } else {
                        current_payload.insert("url".to_string(), Value::String(format_url(value)));
                    }
                } else {
                    if key == "url" {
                        current_payload.insert(key.to_string(), Value::String(format_url(value)));
                    } else {
                        current_payload.insert(key.to_string(), parse_scalar_value(value));
                    }
                }
            }
        } else {
            if let Some(task_name) = current_task.take() {
                current_group.push(TaskDefinition {
                    name: task_name,
                    payload: std::mem::take(&mut current_payload),
                });
            }
            current_task = Some(crate::task::normalize_task_name(arg).to_string());
            current_payload.clear();
        }
    }

    if let Some(task_name) = current_task.take() {
        current_group.push(TaskDefinition {
            name: task_name,
            payload: std::mem::take(&mut current_payload),
        });
    }

    if !current_group.is_empty() {
        groups.push(current_group);
    }

    groups
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

fn parse_scalar_value(value: &str) -> Value {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Value::String(String::new());
    }
    if let Ok(boolean) = trimmed.parse::<bool>() {
        return Value::Bool(boolean);
    }
    if let Ok(integer) = trimmed.parse::<i64>() {
        return Value::Number(integer.into());
    }
    if let Ok(float_value) = trimmed.parse::<f64>() {
        if let Some(number) = serde_json::Number::from_f64(float_value) {
            return Value::Number(number);
        }
    }
    Value::String(trimmed.to_string())
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
/// # use auto::cli::{format_task_groups, TaskDefinition};
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
}
