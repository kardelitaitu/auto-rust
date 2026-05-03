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
                let formatted_sources: Vec<String> = sources
                    .iter()
                    .map(|s| match s {
                        TaskSource::BuiltInRust => "built-in (rust)".to_string(),
                        TaskSource::ConfiguredPath(path) => {
                            format!("external file: {}", path.display())
                        }
                        TaskSource::Unknown => "unknown source".to_string(),
                    })
                    .collect();

                write!(
                    f,
                    "Task name conflict: '{}' found in multiple sources:\n  - {}",
                    name,
                    formatted_sources.join("\n  - ")
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
        for &name in crate::task::TASK_NAMES {
            self.register_built_in(name, name);
        }
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

    /// Register an external task from a file.
    ///
    /// # Arguments
    /// * `name` - Task name
    /// * `path` - Path to the task file
    /// * `policy_name` - Policy to use for this task
    ///
    /// # Errors
    /// Returns `RegistryError::Conflict` if task with same name already exists.
    pub fn register_external(
        &mut self,
        name: &str,
        path: PathBuf,
        policy_name: &'static str,
    ) -> Result<(), RegistryError> {
        // Check for conflicts
        if let Some(existing) = self.tasks.get(name) {
            return Err(RegistryError::Conflict {
                name: name.to_string(),
                sources: vec![existing.source.clone(), TaskSource::ConfiguredPath(path)],
            });
        }

        let descriptor = TaskDescriptor {
            name: name.to_string(),
            source: TaskSource::ConfiguredPath(path),
            policy_name,
        };
        self.tasks.insert(name.to_string(), descriptor);
        Ok(())
    }

    /// Load external tasks from configured discovery roots.
    ///
    /// Scans configured directories for task files and adds them to the registry.
    ///
    /// # Arguments
    /// * `config` - Task discovery configuration
    ///
    /// # Returns
    /// Number of external tasks successfully loaded
    pub fn load_external_tasks(&mut self, config: &crate::config::TaskDiscoveryConfig) -> usize {
        if !config.enabled {
            return 0;
        }

        let mut loaded_count = 0;

        for root in &config.roots {
            let root_path = std::path::Path::new(root);
            if !root_path.exists() || !root_path.is_dir() {
                log::warn!(
                    "Task discovery root '{}' does not exist or is not a directory",
                    root
                );
                continue;
            }

            for extension in &config.extensions {
                let pattern = format!("{}/*.{}", root, extension);
                if let Ok(entries) = glob::glob(&pattern) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_stem().and_then(|s| s.to_str()) {
                            // External tasks use "default" policy for now
                            // In future, policy could be specified in task file metadata
                            match self.register_external(name, entry.clone(), "default") {
                                Ok(()) => {
                                    log::info!("Loaded external task '{}' from {:?}", name, entry);
                                    loaded_count += 1;
                                }
                                Err(RegistryError::Conflict { name, sources }) => {
                                    log::warn!(
                                        "Skipping external task '{}': conflicts with existing task from {:?}",
                                        name,
                                        sources.first().map(|s| format!("{:?}", s)).unwrap_or_default()
                                    );
                                }
                                Err(_) => {}
                            }
                        }
                    }
                }
            }
        }

        loaded_count
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
        let normalized = crate::task::normalize_task_name(name);
        match self.tasks.get(normalized) {
            Some(descriptor) => Ok(descriptor.clone()),
            None => Err(RegistryError::UnknownTask {
                name: normalized.to_string(),
            }),
        }
    }

    /// Check if a task is known (exists in registry).
    pub fn is_known(&self, name: &str) -> bool {
        let normalized = crate::task::normalize_task_name(name);
        self.tasks.contains_key(normalized)
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

    /// Check for potential conflicts with external tasks.
    ///
    /// Returns a list of task names that would conflict if loaded from the given paths.
    /// This is useful for pre-flight checks before loading external tasks.
    ///
    /// # Arguments
    /// * `paths` - Iterator of (name, path) tuples to check
    ///
    /// # Returns
    /// Vector of task names that already exist in the registry
    pub fn check_conflicts<'a>(
        &self,
        paths: impl Iterator<Item = (&'a str, &'a std::path::Path)>,
    ) -> Vec<(String, TaskSource, PathBuf)> {
        let mut conflicts = Vec::new();

        for (name, path) in paths {
            if let Some(existing) = self.tasks.get(name) {
                conflicts.push((
                    name.to_string(),
                    existing.source.clone(),
                    path.to_path_buf(),
                ));
            }
        }

        conflicts
    }

    /// Generate a diagnostics report for the registry.
    ///
    /// Returns detailed information about tasks, sources, and any issues.
    pub fn diagnostics(&self) -> RegistryDiagnostics {
        let built_in_count = self
            .tasks
            .values()
            .filter(|t| t.source.is_built_in())
            .count();
        let external_count = self
            .tasks
            .values()
            .filter(|t| t.source.is_configured())
            .count();

        RegistryDiagnostics {
            total_tasks: self.tasks.len(),
            built_in_tasks: built_in_count,
            external_tasks: external_count,
            task_names: self.task_names(),
        }
    }
}

/// Diagnostics information about the registry state.
#[derive(Debug, Clone)]
pub struct RegistryDiagnostics {
    /// Total number of registered tasks
    pub total_tasks: usize,
    /// Number of built-in Rust tasks
    pub built_in_tasks: usize,
    /// Number of external configured tasks
    pub external_tasks: usize,
    /// All task names (sorted)
    pub task_names: Vec<String>,
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
            TaskSource::BuiltInRust => "BuiltInRust".to_string(),
            TaskSource::ConfiguredPath(path) => format!("ConfiguredPath({})", path.display()),
            TaskSource::Unknown => "Unknown".to_string(),
        };

        output.push_str(&format!(
            "  {:20}  {:30}  policy={}\n",
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
        assert_eq!(task.policy_name, "cookiebot");
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

    #[test]
    fn test_register_external_task_success() {
        let mut registry = TaskRegistry::new();
        let path = PathBuf::from("/external/my_task.task");

        registry
            .register_external("my_task", path.clone(), "default")
            .unwrap();

        assert!(registry.is_known("my_task"));
        let task = registry.lookup("my_task").unwrap();
        assert_eq!(task.name, "my_task");
        assert!(task.source.is_configured());
        assert_eq!(task.source.path(), Some(&path));
    }

    #[test]
    fn test_register_external_task_conflict_with_builtin() {
        let mut registry = TaskRegistry::with_built_in_tasks();
        let path = PathBuf::from("/external/cookiebot.task");

        let result = registry.register_external("cookiebot", path, "default");
        assert!(matches!(result, Err(RegistryError::Conflict { name, .. }) if name == "cookiebot"));
    }

    #[test]
    fn test_register_external_task_conflict_with_external() {
        let mut registry = TaskRegistry::new();
        let path1 = PathBuf::from("/external/task1.task");
        let path2 = PathBuf::from("/external/task2.task");

        registry
            .register_external("my_task", path1, "default")
            .unwrap();

        let result = registry.register_external("my_task", path2, "default");
        assert!(matches!(result, Err(RegistryError::Conflict { name, .. }) if name == "my_task"));
    }

    #[test]
    fn test_load_external_tasks_disabled() {
        let mut registry = TaskRegistry::new();
        let config = crate::config::TaskDiscoveryConfig {
            enabled: false,
            roots: vec!["./tasks".to_string()],
            extensions: vec!["task".to_string()],
        };

        let loaded = registry.load_external_tasks(&config);
        assert_eq!(loaded, 0);
    }

    #[test]
    fn test_load_external_tasks_empty_roots() {
        let mut registry = TaskRegistry::new();
        let config = crate::config::TaskDiscoveryConfig {
            enabled: true,
            roots: vec![],
            extensions: vec!["task".to_string()],
        };

        let loaded = registry.load_external_tasks(&config);
        assert_eq!(loaded, 0);
    }

    #[test]
    fn test_load_external_tasks_nonexistent_root() {
        let mut registry = TaskRegistry::new();
        let config = crate::config::TaskDiscoveryConfig {
            enabled: true,
            roots: vec!["/nonexistent/path".to_string()],
            extensions: vec!["task".to_string()],
        };

        let loaded = registry.load_external_tasks(&config);
        assert_eq!(loaded, 0);
    }

    #[test]
    fn test_registry_diagnostics() {
        let registry = TaskRegistry::with_built_in_tasks();
        let diag = registry.diagnostics();

        assert_eq!(diag.total_tasks, 15);
        assert_eq!(diag.built_in_tasks, 15);
        assert_eq!(diag.external_tasks, 0);
        assert_eq!(diag.task_names.len(), 15);
        assert!(diag.task_names.contains(&"cookiebot".to_string()));
    }

    #[test]
    fn test_check_conflicts_with_builtin() {
        let registry = TaskRegistry::with_built_in_tasks();
        let external_path = PathBuf::from("/external/cookiebot.task");

        let conflicts =
            registry.check_conflicts([("cookiebot", external_path.as_path())].into_iter());

        assert_eq!(conflicts.len(), 1);
        let (name, source, path) = &conflicts[0];
        assert_eq!(name, "cookiebot");
        assert!(source.is_built_in());
        assert_eq!(path, &external_path);
    }

    #[test]
    fn test_check_conflicts_no_conflict() {
        let registry = TaskRegistry::with_built_in_tasks();
        let external_path = PathBuf::from("/external/new_task.task");

        let conflicts =
            registry.check_conflicts([("new_task", external_path.as_path())].into_iter());

        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_conflict_error_display_format() {
        let err = RegistryError::Conflict {
            name: "my_task".to_string(),
            sources: vec![
                TaskSource::BuiltInRust,
                TaskSource::ConfiguredPath(PathBuf::from("/external/my_task.task")),
            ],
        };

        let err_msg = err.to_string();
        assert!(err_msg.contains("Task name conflict: 'my_task'"));
        assert!(err_msg.contains("built-in (rust)"));
        assert!(err_msg.contains("external file:"));
    }

    #[test]
    fn test_unknown_task_error_display() {
        let err = RegistryError::UnknownTask {
            name: "nonexistent".to_string(),
        };
        assert_eq!(err.to_string(), "Task 'nonexistent' not found");
    }
}
