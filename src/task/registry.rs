//! Task registry module.
//!
//! Provides a unified registry abstraction for task discovery, validation,
//! and policy lookup. This is the foundation for the v0.0.5 task system.
//!
//! # Core Types
//!
//! - [`TaskDescriptor`] - Metadata describing a task
//! - [`TaskSource`] - Where the task comes from
//! - [`TaskRegistry`] - The registry itself
//!
//! # Design Principles
//!
//! 1. Single source of truth for task metadata
//! 2. Backward compatible - all existing tasks work unchanged
//! 3. Explicit error handling for unknown tasks and conflicts

use std::collections::HashMap;
use std::path::PathBuf;

/// Metadata describing a task in the registry.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskDescriptor {
    /// Task name (e.g., "cookiebot", "twitteractivity")
    pub name: String,
    /// Where the task comes from
    pub source: TaskSource,
    /// Policy name for permission/timeout configuration
    pub policy_name: &'static str,
}

/// Source of a task.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskSource {
    /// Built-in Rust task compiled into the binary
    BuiltInRust,
    /// Task from an external file path (configured)
    ConfiguredPath(PathBuf),
    /// Task not found - used for error reporting
    Unknown,
}

impl TaskSource {
    /// Returns true if this is a built-in Rust task.
    pub fn is_built_in(&self) -> bool {
        matches!(self, TaskSource::BuiltInRust)
    }

    /// Returns true if this is an external configured task.
    pub fn is_configured(&self) -> bool {
        matches!(self, TaskSource::ConfiguredPath(_))
    }

    /// Returns true if this represents an unknown task.
    pub fn is_unknown(&self) -> bool {
        matches!(self, TaskSource::Unknown)
    }

    /// Get the path if this is a ConfiguredPath variant.
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            TaskSource::ConfiguredPath(path) => Some(path),
            _ => None,
        }
    }
}

/// Error types for registry operations.
#[derive(Debug, Clone, PartialEq)]
pub enum RegistryError {
    /// Task not found in any source
    UnknownTask { name: String },
    /// Same task name exists in multiple sources (conflict)
    Conflict {
        name: String,
        sources: Vec<TaskSource>,
    },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::UnknownTask { name } => {
                write!(f, "Task '{}' not found", name)
            }
            RegistryError::Conflict { name, sources } => {
                write!(
                    f,
                    "Task '{}' exists in multiple sources: {:?}",
                    name, sources
                )
            }
        }
    }
}

impl std::error::Error for RegistryError {}

/// Task registry providing unified task discovery and metadata.
///
/// The registry is the single source of truth for:
/// - Task existence checks
/// - Task metadata (source, policy)
/// - Task listing and inspection
///
/// # Example
///
/// ```rust
/// use auto::task::registry::{TaskRegistry, TaskDescriptor, TaskSource};
///
/// let registry = TaskRegistry::with_built_in_tasks();
/// let cookiebot = registry.lookup("cookiebot").unwrap();
/// assert!(cookiebot.source.is_built_in());
/// ```
#[derive(Debug, Default)]
pub struct TaskRegistry {
    /// Map of task name to descriptor
    tasks: HashMap<String, TaskDescriptor>,
    /// Whether to allow external task sources (Phase 2)
    #[allow(dead_code)]
    allow_external: bool,
}

impl TaskRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            allow_external: false,
        }
    }

    /// Create a registry pre-populated with all built-in tasks.
    ///
    /// This is the default registry used by the application.
    pub fn with_built_in_tasks() -> Self {
        let mut registry = Self::new();
        registry.register_built_in_tasks();
        registry
    }

    /// Register all built-in Rust tasks.
    fn register_built_in_tasks(&mut self) {
        // Demo tasks
        self.register_built_in("cookiebot", "default");
        self.register_built_in("demo-keyboard", "default");
        self.register_built_in("demo-mouse", "default");
        self.register_built_in("demoqa", "default");
        self.register_built_in("pageview", "default");
        self.register_built_in("task-example", "default");

        // Twitter/X tasks
        self.register_built_in("twitteractivity", "default");
        self.register_built_in("twitterdive", "default");
        self.register_built_in("twitterfollow", "default");
        self.register_built_in("twitterintent", "default");
        self.register_built_in("twitterlike", "default");
        self.register_built_in("twitterquote", "default");
        self.register_built_in("twitterreply", "default");
        self.register_built_in("twitterretweet", "default");
        self.register_built_in("twittertest", "default");
    }

    /// Register a built-in Rust task.
    fn register_built_in(&mut self, name: &str, policy_name: &'static str) {
        let descriptor = TaskDescriptor {
            name: name.to_string(),
            source: TaskSource::BuiltInRust,
            policy_name,
        };
        self.tasks.insert(name.to_string(), descriptor);
    }

    /// Look up a task by name.
    ///
    /// Returns the task descriptor if found, or an error if unknown.
    ///
    /// # Errors
    ///
    /// - [`RegistryError::UnknownTask`] if task not found
    /// - [`RegistryError::Conflict`] if task exists in multiple sources (Phase 2)
    pub fn lookup(&self, name: &str) -> Result<TaskDescriptor, RegistryError> {
        match self.tasks.get(name) {
            Some(descriptor) => Ok(descriptor.clone()),
            None => Err(RegistryError::UnknownTask {
                name: name.to_string(),
            }),
        }
    }

    /// Check if a task is known (exists in registry).
    pub fn is_known(&self, name: &str) -> bool {
        self.tasks.contains_key(name)
    }

    /// List all registered tasks.
    ///
    /// Returns tasks sorted by name.
    pub fn list_tasks(&self) -> Vec<&TaskDescriptor> {
        let mut tasks: Vec<_> = self.tasks.values().collect();
        tasks.sort_by_key(|t| &t.name);
        tasks
    }

    /// Get the count of registered tasks.
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Get all task names.
    pub fn task_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.tasks.keys().cloned().collect();
        names.sort();
        names
    }
}

/// Format the task list for display (--list-tasks output).
///
/// Returns a formatted string with task names, sources, and policies.
pub fn format_task_list() -> String {
    let registry = TaskRegistry::with_built_in_tasks();
    let tasks = registry.list_tasks();

    let mut output = String::new();
    output.push_str("Available Tasks:\n");
    output.push_str("================\n\n");

    for task in &tasks {
        let source_str = match &task.source {
            TaskSource::BuiltInRust => "built-in",
            TaskSource::ConfiguredPath(_) => "external",
            TaskSource::Unknown => "unknown",
        };

        output.push_str(&format!(
            "  {:20}  {:10}  {}\n",
            task.name, source_str, task.policy_name
        ));
    }

    output.push_str(&format!("\nTotal: {} tasks\n", tasks.len()));
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_all_built_in_tasks() {
        let registry = TaskRegistry::with_built_in_tasks();
        assert_eq!(registry.task_count(), 15);
    }

    #[test]
    fn test_lookup_known_task() {
        let registry = TaskRegistry::with_built_in_tasks();
        let task = registry.lookup("cookiebot").unwrap();
        assert_eq!(task.name, "cookiebot");
        assert!(task.source.is_built_in());
        assert_eq!(task.policy_name, "default");
    }

    #[test]
    fn test_lookup_unknown_task() {
        let registry = TaskRegistry::with_built_in_tasks();
        let result = registry.lookup("nonexistent");
        assert!(matches!(
            result,
            Err(RegistryError::UnknownTask { name }) if name == "nonexistent"
        ));
    }

    #[test]
    fn test_is_known() {
        let registry = TaskRegistry::with_built_in_tasks();
        assert!(registry.is_known("cookiebot"));
        assert!(registry.is_known("twitteractivity"));
        assert!(!registry.is_known("unknown_task"));
    }

    #[test]
    fn test_list_tasks_sorted() {
        let registry = TaskRegistry::with_built_in_tasks();
        let tasks = registry.list_tasks();
        assert_eq!(tasks.len(), 15);

        // Check sorted order
        let names: Vec<_> = tasks.iter().map(|t| t.name.clone()).collect();
        let mut sorted_names = names.clone();
        sorted_names.sort();
        assert_eq!(names, sorted_names);
    }

    #[test]
    fn test_task_names() {
        let registry = TaskRegistry::with_built_in_tasks();
        let names = registry.task_names();
        assert_eq!(names.len(), 15);
        assert!(names.contains(&"cookiebot".to_string()));
        assert!(names.contains(&"twitteractivity".to_string()));
    }

    #[test]
    fn test_task_source_variants() {
        let built_in = TaskSource::BuiltInRust;
        assert!(built_in.is_built_in());
        assert!(!built_in.is_configured());
        assert!(!built_in.is_unknown());
        assert!(built_in.path().is_none());

        let path = PathBuf::from("/path/to/task");
        let configured = TaskSource::ConfiguredPath(path.clone());
        assert!(!configured.is_built_in());
        assert!(configured.is_configured());
        assert!(!configured.is_unknown());
        assert_eq!(configured.path(), Some(&path));

        let unknown = TaskSource::Unknown;
        assert!(!unknown.is_built_in());
        assert!(!unknown.is_configured());
        assert!(unknown.is_unknown());
        assert!(unknown.path().is_none());
    }
}
