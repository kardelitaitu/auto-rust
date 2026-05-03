//! Task file watcher for hot reload of external tasks.
//!
//! This module provides file system watching capabilities for external task
//! directories, allowing tasks to be reloaded automatically when files change.
//!
//! # Example
//!
//! ```ignore
//! use auto::task::watcher::TaskWatcher;
//! use auto::task::registry::TaskRegistry;
//! use auto::config::TaskDiscoveryConfig;
//! use std::sync::Arc;
//! use parking_lot::Mutex;
//!
//! async fn example(registry: Arc<Mutex<TaskRegistry>>) -> anyhow::Result<()> {
//!     let mut watcher = TaskWatcher::new(registry);
//!     let config = TaskDiscoveryConfig::default();
//!     watcher.watch("./tasks", &config).await?;
//!     Ok(())
//! }
//! ```

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::task::registry::TaskRegistry;

/// Watcher for external task files.
///
/// Monitors external task directories and reloads tasks when files change.
/// Designed to run in the background during task execution.
pub struct TaskWatcher {
    /// The registry to update when files change
    registry: Arc<Mutex<TaskRegistry>>,
    /// Channel sender for file events
    tx: Option<mpsc::Sender<FileEvent>>,
    /// Background watch task handle
    handle: Option<JoinHandle<()>>,
    /// Native file watcher handle
    watcher: Option<RecommendedWatcher>,
}

/// File system events relevant to task reloading.
#[derive(Debug, Clone)]
pub enum FileEvent {
    /// A task file was created
    Created(String),
    /// A task file was modified
    Modified(String),
    /// A task file was deleted
    Deleted(String),
    /// A task file was renamed
    Renamed(String, String),
}

impl TaskWatcher {
    /// Create a new task watcher.
    ///
    /// # Arguments
    /// * `registry` - The task registry to update when files change
    ///
    /// # Returns
    /// A new TaskWatcher instance
    pub fn new(registry: Arc<Mutex<TaskRegistry>>) -> Self {
        Self {
            registry,
            tx: None,
            handle: None,
            watcher: None,
        }
    }

    /// Start watching a directory for task file changes.
    ///
    /// # Arguments
    /// * `path` - Directory to watch
    /// * `config` - Task discovery configuration for reloading
    ///
    /// # Returns
    /// Ok(()) on success
    ///
    /// # Errors
    /// Returns error if watcher cannot be created
    pub async fn watch(
        &mut self,
        path: impl AsRef<Path>,
        task_config: &crate::config::TaskDiscoveryConfig,
    ) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(100);
        self.tx = Some(tx);

        let path = path.as_ref().to_path_buf();
        let registry = self.registry.clone();
        let config = task_config.clone();

        // Create notify watcher
        let watcher_tx = self.tx.as_ref().unwrap().clone();
        let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| match res {
            Ok(event) => {
                if let Err(e) = Self::handle_notify_event(&event, &watcher_tx) {
                    log::debug!("Failed to handle notify event: {}", e);
                }
            }
            Err(e) => log::warn!("File watcher error: {}", e),
        })
        .context("Failed to create file watcher")?;

        // Watch the directory
        watcher
            .watch(&path, RecursiveMode::NonRecursive)
            .context("Failed to watch directory")?;

        // Start background processing task
        let handle = tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    FileEvent::Created(path) | FileEvent::Modified(path) => {
                        log::info!("Task file changed: {}", path);
                        let mut reg = registry.lock();
                        // Reload the specific task
                        if let Some(name) = std::path::Path::new(&path)
                            .file_stem()
                            .and_then(|s| s.to_str())
                        {
                            if let Err(e) = Self::reload_task(&mut reg, name, &path, &config) {
                                log::warn!("Failed to reload task '{}': {}", name, e);
                            }
                        }
                    }
                    FileEvent::Deleted(path) => {
                        log::info!("Task file deleted: {}", path);
                        // Remove from registry
                        let mut reg = registry.lock();
                        if let Some(name) = std::path::Path::new(&path)
                            .file_stem()
                            .and_then(|s| s.to_str())
                        {
                            let _ = reg.remove(name);
                        }
                    }
                    FileEvent::Renamed(old, new) => {
                        log::info!("Task file renamed: {} -> {}", old, new);
                        let mut reg = registry.lock();
                        // Remove old, add new
                        if let Some(old_name) = std::path::Path::new(&old)
                            .file_stem()
                            .and_then(|s| s.to_str())
                        {
                            let _ = reg.remove(old_name);
                        }
                        if let Some(new_name) = std::path::Path::new(&new)
                            .file_stem()
                            .and_then(|s| s.to_str())
                        {
                            if let Err(e) = Self::reload_task(&mut reg, new_name, &new, &config) {
                                log::warn!("Failed to reload task '{}': {}", new_name, e);
                            }
                        }
                    }
                }
            }
        });

        self.handle = Some(handle);
        self.watcher = Some(watcher);
        log::info!("Started watching task directory: {}", path.display());

        Ok(())
    }

    /// Stop the watcher.
    pub fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        if self.watcher.take().is_some() {
            log::info!("Task watcher stopped");
        }
    }

    /// Handle a notify event and convert to FileEvent.
    fn handle_notify_event(event: &Event, tx: &mpsc::Sender<FileEvent>) -> Result<()> {
        use notify::EventKind;

        let paths: Vec<_> = event
            .paths
            .iter()
            .filter(|p| {
                p.extension()
                    .map(|e| e == "task" || e == "yaml" || e == "yml" || e == "toml")
                    .unwrap_or(false)
            })
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        if paths.is_empty() {
            return Ok(());
        }

        match &event.kind {
            EventKind::Create(_) => {
                for path in paths {
                    let _ = tx.try_send(FileEvent::Created(path));
                }
            }
            EventKind::Modify(_) => {
                for path in paths {
                    let _ = tx.try_send(FileEvent::Modified(path));
                }
            }
            EventKind::Remove(_) => {
                for path in paths {
                    let _ = tx.try_send(FileEvent::Deleted(path));
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Reload a single task.
    fn reload_task(
        registry: &mut TaskRegistry,
        name: &str,
        path: &str,
        _config: &crate::config::TaskDiscoveryConfig,
    ) -> Result<()> {
        let path = std::path::Path::new(path);
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "Task file does not exist: {}",
                path.display()
            ));
        }

        // Remove old version if exists
        registry.remove(name);

        // Try to load as DSL task
        match crate::task::dsl::parse_task_file(path) {
            Ok(mut task_def) => {
                // Validate
                if let Err(errors) = crate::task::dsl::validate_task_definition(&task_def) {
                    log::warn!(
                        "Reloaded task '{}' has validation errors: {:?}",
                        name,
                        errors
                    );
                }

                if task_def.name != name {
                    log::warn!(
                        "Task file '{}' declares name '{}' but watcher key is '{}'; using file stem as canonical name",
                        path.display(),
                        task_def.name,
                        name
                    );
                    task_def.name = name.to_string();
                }

                // Update registry
                use crate::task::registry::{TaskDescriptor, TaskSource};
                let descriptor = TaskDescriptor {
                    name: name.to_string(),
                    source: TaskSource::ConfiguredPath(path.to_path_buf()),
                    policy_name: Box::leak(task_def.policy.clone().into_boxed_str()),
                    task_def: Some(task_def),
                };
                registry.insert(name.to_string(), descriptor);
                log::info!("Reloaded task '{}' from {}", name, path.display());
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to parse task file: {}", e));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::registry::TaskRegistry;
    use notify::{event::CreateKind, event::ModifyKind, event::RemoveKind, EventKind};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::sync::mpsc;

    #[test]
    fn test_file_event_variants() {
        let created = FileEvent::Created("/path/task.task".to_string());
        let modified = FileEvent::Modified("/path/task.task".to_string());
        let deleted = FileEvent::Deleted("/path/task.task".to_string());
        let renamed =
            FileEvent::Renamed("/path/old.task".to_string(), "/path/new.task".to_string());

        // Just verify they can be created and debug formatted
        assert!(format!("{:?}", created).contains("Created"));
        assert!(format!("{:?}", modified).contains("Modified"));
        assert!(format!("{:?}", deleted).contains("Deleted"));
        assert!(format!("{:?}", renamed).contains("Renamed"));
    }

    #[test]
    fn test_task_registry_remove() {
        let mut registry = TaskRegistry::new();
        registry
            .register_external("test_task", PathBuf::from("/test.task"), "default")
            .unwrap();

        assert!(registry.is_known("test_task"));
        let removed = registry.remove("test_task");
        assert!(removed.is_some());
        assert!(!registry.is_known("test_task"));
    }

    #[test]
    fn test_task_registry_remove_unknown() {
        let mut registry = TaskRegistry::new();
        let removed = registry.remove("unknown_task");
        assert!(removed.is_none());
    }

    #[test]
    fn test_task_registry_insert() {
        let mut registry = TaskRegistry::new();
        use crate::task::registry::{TaskDescriptor, TaskSource};

        let descriptor = TaskDescriptor {
            name: "inserted_task".to_string(),
            source: TaskSource::ConfiguredPath(PathBuf::from("/inserted.task")),
            policy_name: "default",
            task_def: None,
        };

        registry.insert("inserted_task".to_string(), descriptor);
        assert!(registry.is_known("inserted_task"));
    }

    #[tokio::test]
    async fn test_handle_notify_event_maps_create_modify_remove() {
        let (tx, mut rx) = mpsc::channel(8);

        let mut event = notify::Event::new(EventKind::Create(CreateKind::File));
        event.paths.push(PathBuf::from("C:/tmp/sample.task"));
        TaskWatcher::handle_notify_event(&event, &tx).unwrap();
        assert!(
            matches!(rx.recv().await, Some(FileEvent::Created(path)) if path.contains("sample.task"))
        );

        let mut event = notify::Event::new(EventKind::Modify(ModifyKind::Data(
            notify::event::DataChange::Content,
        )));
        event.paths.push(PathBuf::from("C:/tmp/sample.task"));
        TaskWatcher::handle_notify_event(&event, &tx).unwrap();
        assert!(
            matches!(rx.recv().await, Some(FileEvent::Modified(path)) if path.contains("sample.task"))
        );

        let mut event = notify::Event::new(EventKind::Remove(RemoveKind::File));
        event.paths.push(PathBuf::from("C:/tmp/sample.task"));
        TaskWatcher::handle_notify_event(&event, &tx).unwrap();
        assert!(
            matches!(rx.recv().await, Some(FileEvent::Deleted(path)) if path.contains("sample.task"))
        );
    }

    #[test]
    fn test_handle_notify_event_ignores_non_task_files() {
        let (tx, mut rx) = mpsc::channel(8);
        let mut event = notify::Event::new(EventKind::Create(CreateKind::File));
        event.paths.push(PathBuf::from("C:/tmp/sample.txt"));

        TaskWatcher::handle_notify_event(&event, &tx).unwrap();
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_reload_task_succeeds_for_valid_task() {
        let dir = TempDir::new().unwrap();
        let task_file = dir.path().join("reload_me.task");
        fs::write(
            &task_file,
            r#"
name: reload_me
policy: default
actions:
  - action: wait
    duration_ms: 5
"#,
        )
        .unwrap();

        let mut registry = TaskRegistry::new();
        let config = crate::config::TaskDiscoveryConfig {
            enabled: true,
            roots: vec![dir.path().to_string_lossy().to_string()],
            extensions: vec!["task".to_string()],
        };

        TaskWatcher::reload_task(
            &mut registry,
            "reload_me",
            &task_file.to_string_lossy(),
            &config,
        )
        .unwrap();

        let descriptor = registry.lookup("reload_me").unwrap();
        assert!(descriptor.source.is_configured());
        assert!(descriptor.task_def.is_some());
    }

    #[test]
    fn test_reload_task_fails_for_missing_file() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("missing.task");
        let mut registry = TaskRegistry::new();
        let config = crate::config::TaskDiscoveryConfig::default();

        let err = TaskWatcher::reload_task(
            &mut registry,
            "missing",
            &missing.to_string_lossy(),
            &config,
        )
        .unwrap_err();

        assert!(err.to_string().contains("Task file does not exist"));
    }

    #[test]
    fn test_reload_task_fails_for_invalid_file() {
        let dir = TempDir::new().unwrap();
        let task_file = dir.path().join("broken.task");
        fs::write(&task_file, "not valid").unwrap();
        let mut registry = TaskRegistry::new();
        let config = crate::config::TaskDiscoveryConfig::default();

        let err = TaskWatcher::reload_task(
            &mut registry,
            "broken",
            &task_file.to_string_lossy(),
            &config,
        )
        .unwrap_err();

        assert!(err.to_string().contains("Failed to parse task file"));
    }

    #[tokio::test]
    async fn test_stop_is_safe_with_and_without_active_handle() {
        let registry = Arc::new(Mutex::new(TaskRegistry::new()));
        let mut watcher = TaskWatcher::new(registry);

        watcher.stop();

        watcher.handle = Some(tokio::spawn(async {}));
        watcher.stop();

        assert!(watcher.handle.is_none());
    }
}
