//! Command-line interface and task parsing module.
//!
//! This module handles:
//! - Command-line argument parsing using clap
//! - Task group parsing (handling "then" separators)
//! - Task definition structures
//! - Task existence validation
//! - Formatting task groups for display and logging

use clap::Parser;
use log::warn;
use std::collections::{HashMap, HashSet};
use std::path::Path;

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

/// Result of task validation
#[derive(Debug, Clone)]
pub struct TaskValidationResult {
    pub task_name: String,
    pub is_known: bool,
    pub file_exists: bool,
    pub warnings: Vec<String>,
}

/// Registry of known task names
/// This should match the tasks registered in `task/mod.rs`
const KNOWN_TASKS: &[&str] = &[
    "cookiebot",
    "pageview",
    "demo-keyboard",
    "demo-mouse",
    "demoqa",
    "twitterfollow",
    "twitterquote",
    "twitterreply",
    "twitteractivity",
];

/// Check if a task name is known (registered in the orchestrator)
pub fn is_known_task(task_name: &str) -> bool {
    let clean_name = task_name.strip_suffix(".js").unwrap_or(task_name);
    KNOWN_TASKS.contains(&clean_name)
}

/// Check if a task file exists in the task directory
pub fn task_file_exists(task_name: &str) -> bool {
    let clean_name = task_name.strip_suffix(".js").unwrap_or(task_name);

    // Check for .rs file in task/ directory
    let task_path = Path::new("task").join(format!("{}.rs", clean_name));
    if task_path.exists() {
        return true;
    }

    // Also check for .js fallback (for compatibility)
    let js_path = Path::new("task").join(format!("{}.js", clean_name));
    js_path.exists()
}

/// Validate a task name and return validation result
pub fn validate_task(task_name: &str) -> TaskValidationResult {
    let clean_name = task_name.strip_suffix(".js").unwrap_or(task_name);
    let mut warnings = Vec::new();

    let is_known = is_known_task(task_name);
    let file_exists = task_file_exists(task_name);

    if !is_known {
        warnings.push(format!(
            "Unknown task name '{}'. Known tasks: {}",
            clean_name,
            KNOWN_TASKS.join(", ")
        ));
    }

    if !file_exists {
        warnings.push(format!(
            "Task file for '{}' not found in task/ directory",
            clean_name
        ));
    }

    TaskValidationResult {
        task_name: clean_name.to_string(),
        is_known,
        file_exists,
        warnings,
    }
}

/// Validate all tasks in a group and log warnings for unknown tasks
pub fn validate_task_groups(groups: &[Vec<TaskDefinition>]) -> Vec<TaskValidationResult> {
    let mut results = Vec::new();
    let mut seen_tasks: HashSet<String> = HashSet::new();

    for group in groups {
        for task in group {
            if !seen_tasks.contains(&task.name) {
                let result = validate_task(&task.name);

                // Log warnings for unknown tasks
                for warning in &result.warnings {
                    warn!("{}", warning);
                }

                results.push(result);
                seen_tasks.insert(task.name.clone());
            }
        }
    }

    results
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

                let shorthand_task_name = if key.ends_with(".js") {
                    key.strip_suffix(".js").unwrap_or(key).to_string()
                } else {
                    key.to_string()
                };

                if current_task.is_none() {
                    let is_numeric = value.chars().all(|c| c.is_ascii_digit()) && !value.is_empty();
                    current_task = Some(shorthand_task_name);
                    if is_numeric {
                        current_payload.insert("value".to_string(), value.to_string());
                    } else if key == "url" {
                        current_payload.insert("url".to_string(), format_url(value));
                    } else if value.contains('=') {
                        if let Some(eq_pos) = value.find('=') {
                            let param_key = &value[..eq_pos];
                            let param_value = &value[eq_pos + 1..];
                            let formatted_value = if param_key == "url" {
                                format_url(param_value)
                            } else {
                                param_value.to_string()
                            };
                            current_payload.insert("url".to_string(), formatted_value);
                        }
                    } else {
                        current_payload.insert("url".to_string(), format_url(value));
                    }
                } else if key == current_task.as_ref().unwrap() {
                    if let Some(task_name) = current_task.take() {
                        current_group.push(TaskDefinition {
                            name: task_name,
                            payload: std::mem::take(&mut current_payload),
                        });
                    }
                    let is_numeric = value.chars().all(|c| c.is_ascii_digit()) && !value.is_empty();
                    current_task = Some(shorthand_task_name);
                    if is_numeric {
                        current_payload.insert("value".to_string(), value.to_string());
                    } else if key == "url" {
                        current_payload.insert("url".to_string(), format_url(value));
                    } else if value.contains('=') {
                        if let Some(eq_pos) = value.find('=') {
                            let param_key = &value[..eq_pos];
                            let param_value = &value[eq_pos + 1..];
                            let formatted_value = if param_key == "url" {
                                format_url(param_value)
                            } else {
                                param_value.to_string()
                            };
                            current_payload.insert("url".to_string(), formatted_value);
                        }
                    } else {
                        current_payload.insert("url".to_string(), format_url(value));
                    }
                } else {
                    let param_value = if key == "url" {
                        format_url(value)
                    } else {
                        value.to_string()
                    };
                    current_payload.insert(key.to_string(), param_value);
                }
            }
        } else {
            if let Some(task_name) = current_task.take() {
                current_group.push(TaskDefinition {
                    name: task_name,
                    payload: std::mem::take(&mut current_payload),
                });
            }
            current_task = Some(arg.strip_suffix(".js").unwrap_or(arg).to_string());
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
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0][0].name, "cookiebot");
        assert_eq!(
            result[0][0].payload.get("pageview"),
            Some(&"reddit.com".to_string())
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
            Some(&"www.reddit.com".to_string())
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
        assert_eq!(result[0][0].payload.get("value"), Some(&"42".to_string()));
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

    #[test]
    fn test_is_known_task_valid() {
        assert!(is_known_task("cookiebot"));
        assert!(is_known_task("pageview"));
        assert!(is_known_task("twitteractivity"));
        assert!(is_known_task("cookiebot.js"));
    }

    #[test]
    fn test_is_known_task_invalid() {
        assert!(!is_known_task("unknown_task"));
        assert!(!is_known_task("nonexistent"));
    }

    #[test]
    fn test_validate_task_known_task() {
        let result = validate_task("cookiebot");
        assert!(result.is_known);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_validate_task_unknown_task() {
        let result = validate_task("unknown_task");
        assert!(!result.is_known);
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].contains("Unknown task name"));
    }

    #[test]
    fn test_validate_task_groups_logs_warnings() {
        let groups = vec![vec![
            TaskDefinition {
                name: "cookiebot".to_string(),
                payload: HashMap::new(),
            },
            TaskDefinition {
                name: "unknown_task".to_string(),
                payload: HashMap::new(),
            },
        ]];

        let results = validate_task_groups(&groups);

        // Should have 2 results (cookiebot and unknown_task)
        assert_eq!(results.len(), 2);

        // cookiebot should have no warnings
        let cookiebot_result = results.iter().find(|r| r.task_name == "cookiebot").unwrap();
        assert!(cookiebot_result.is_known);
        assert!(cookiebot_result.warnings.is_empty());

        // unknown_task should have warnings
        let unknown_result = results
            .iter()
            .find(|r| r.task_name == "unknown_task")
            .unwrap();
        assert!(!unknown_result.is_known);
        assert!(!unknown_result.warnings.is_empty());
    }
}
