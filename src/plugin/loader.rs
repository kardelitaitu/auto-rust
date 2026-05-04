//! Plugin Loader
//!
//! Loads plugins from the filesystem and creates plugin instances.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::{manifest::PluginManifest, registry::PluginRegistry};

/// Configuration for plugin loading
#[derive(Debug, Clone)]
pub struct PluginLoaderConfig {
    /// Directory to scan for plugins
    pub plugin_dir: PathBuf,
    /// Whether to enable plugins
    pub enabled: bool,
    /// Specific plugins to load (empty = load all)
    pub allowlist: Vec<String>,
    /// Plugins to exclude
    pub denylist: Vec<String>,
    /// Whether to load plugins recursively
    pub recursive: bool,
    /// Maximum number of plugins to load
    pub max_plugins: usize,
}

impl Default for PluginLoaderConfig {
    fn default() -> Self {
        Self {
            plugin_dir: PathBuf::from(super::DEFAULT_PLUGIN_DIR),
            enabled: true,
            allowlist: Vec::new(),
            denylist: Vec::new(),
            recursive: false,
            max_plugins: 100,
        }
    }
}

impl PluginLoaderConfig {
    /// Create config with custom plugin directory
    pub fn with_dir(plugin_dir: impl AsRef<Path>) -> Self {
        Self {
            plugin_dir: plugin_dir.as_ref().to_path_buf(),
            ..Default::default()
        }
    }

    /// Disable plugins
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }
}

/// Plugin loader that scans directories and loads plugins
pub struct PluginLoader {
    config: PluginLoaderConfig,
}

impl PluginLoader {
    /// Create a new plugin loader with configuration
    pub fn new(config: PluginLoaderConfig) -> Self {
        Self { config }
    }

    /// Load all plugins from the configured directory
    pub async fn load_all(&self, registry: &mut PluginRegistry) -> Result<LoadResult> {
        if !self.config.enabled {
            log::info!("Plugin loading is disabled");
            return Ok(LoadResult::disabled());
        }

        if !self.config.plugin_dir.exists() {
            log::debug!(
                "Plugin directory '{}' does not exist, skipping load",
                self.config.plugin_dir.display()
            );
            return Ok(LoadResult::empty());
        }

        let mut result = LoadResult::new();

        // Find all manifest files
        let manifest_paths = self.find_manifests().await?;

        for manifest_path in manifest_paths {
            match self
                .load_plugin_from_manifest(&manifest_path, registry)
                .await
            {
                Ok(name) => {
                    result.loaded.push(name);
                }
                Err(e) => {
                    let name = manifest_path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    log::warn!(
                        "Failed to load plugin from '{}': {}",
                        manifest_path.display(),
                        e
                    );
                    result.failed.push((name, e.to_string()));
                }
            }

            // Check max plugins limit
            if result.loaded.len() >= self.config.max_plugins {
                log::warn!(
                    "Reached max plugin limit ({}), stopping load",
                    self.config.max_plugins
                );
                break;
            }
        }

        log::info!(
            "Plugin loading complete: {} loaded, {} failed",
            result.loaded.len(),
            result.failed.len()
        );

        Ok(result)
    }

    /// Find all manifest files in the plugin directory
    async fn find_manifests(&self) -> Result<Vec<PathBuf>> {
        let mut manifests = Vec::new();

        let entries = tokio::fs::read_dir(&self.config.plugin_dir)
            .await
            .with_context(|| {
                format!(
                    "Failed to read plugin directory: {}",
                    self.config.plugin_dir.display()
                )
            })?;

        let mut entries = entries;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir() && self.config.recursive {
                // Recursively search subdirectories
                let sub_manifests = self.find_manifests_in_dir(&path).await?;
                manifests.extend(sub_manifests);
            } else if self.is_manifest_file(&path) {
                // Check allowlist/denylist
                let name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();

                if self.should_load_plugin(&name) {
                    manifests.push(path);
                }
            }
        }

        Ok(manifests)
    }

    /// Find manifests in a specific directory (iterative to avoid recursion)
    async fn find_manifests_in_dir(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut manifests = Vec::new();
        let mut queue = vec![dir.to_path_buf()];

        while let Some(current_dir) = queue.pop() {
            let mut entries = tokio::fs::read_dir(&current_dir).await?;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();

                if path.is_dir() && self.config.recursive {
                    queue.push(path);
                } else if self.is_manifest_file(&path) {
                    let name = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();

                    if self.should_load_plugin(&name) {
                        manifests.push(path);
                    }
                }
            }
        }

        Ok(manifests)
    }

    /// Check if a path is a manifest file
    fn is_manifest_file(&self, path: &Path) -> bool {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        matches!(ext.as_str(), "toml" | "yaml" | "yml" | "json")
    }

    /// Check if a plugin should be loaded based on allowlist/denylist
    fn should_load_plugin(&self, name: &str) -> bool {
        // Check denylist first
        if self.config.denylist.iter().any(|n| n == name) {
            return false;
        }

        // Check allowlist (if specified, only load allowed plugins)
        if !self.config.allowlist.is_empty() {
            return self.config.allowlist.iter().any(|n| n == name);
        }

        true
    }

    /// Load a single plugin from its manifest file
    async fn load_plugin_from_manifest(
        &self,
        manifest_path: &Path,
        registry: &mut PluginRegistry,
    ) -> Result<String> {
        // Read manifest file
        let content = tokio::fs::read_to_string(manifest_path)
            .await
            .with_context(|| format!("Failed to read manifest: {}", manifest_path.display()))?;

        // Parse manifest based on extension
        let manifest = if manifest_path
            .extension()
            .map(|e| e == "toml")
            .unwrap_or(false)
        {
            PluginManifest::from_toml(&content)?
        } else if manifest_path
            .extension()
            .map(|e| e == "json")
            .unwrap_or(false)
        {
            PluginManifest::from_json(&content)?
        } else {
            PluginManifest::from_yaml(&content)?
        };

        // Resolve WASM path
        let wasm_path = manifest_path
            .parent()
            .unwrap_or(&self.config.plugin_dir)
            .join(&manifest.entry_point);

        // Check that WASM file exists
        if !wasm_path.exists() {
            anyhow::bail!(
                "WASM file not found: {} (referenced from manifest: {})",
                wasm_path.display(),
                manifest_path.display()
            );
        }

        // Store name before moving manifest
        let plugin_name = manifest.name.clone();

        // For now, create a stub plugin (WASM loading would be implemented here)
        // In a full implementation, this would load and instantiate the WASM module
        let plugin = Box::new(StubPlugin {
            manifest: manifest.clone(),
        });

        // Register with registry
        registry.register(manifest, plugin)?;

        log::info!(
            "Loaded plugin '{}' from {}",
            plugin_name,
            manifest_path.display()
        );

        Ok(plugin_name)
    }
}

/// Result of a plugin loading operation
#[derive(Debug, Clone)]
pub struct LoadResult {
    /// Names of successfully loaded plugins
    pub loaded: Vec<String>,
    /// Names and errors of failed plugins
    pub failed: Vec<(String, String)>,
    /// Whether loading was disabled
    pub disabled: bool,
}

impl LoadResult {
    /// Create a new empty result
    fn new() -> Self {
        Self {
            loaded: Vec::new(),
            failed: Vec::new(),
            disabled: false,
        }
    }

    /// Create a disabled result
    fn disabled() -> Self {
        Self {
            loaded: Vec::new(),
            failed: Vec::new(),
            disabled: true,
        }
    }

    /// Create an empty result (no plugin directory)
    fn empty() -> Self {
        Self::new()
    }

    /// Get total number of plugins attempted
    pub fn total(&self) -> usize {
        self.loaded.len() + self.failed.len()
    }

    /// Check if all plugins loaded successfully
    pub fn all_succeeded(&self) -> bool {
        self.failed.is_empty()
    }

    /// Check if any plugins loaded successfully
    pub fn any_succeeded(&self) -> bool {
        !self.loaded.is_empty()
    }
}

/// Stub plugin for development (replaced with WASM loading in production)
struct StubPlugin {
    manifest: PluginManifest,
}

#[async_trait::async_trait]
impl super::Plugin for StubPlugin {
    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn version(&self) -> &str {
        &self.manifest.version
    }

    fn author(&self) -> Option<&str> {
        self.manifest.author.as_deref()
    }

    fn description(&self) -> Option<&str> {
        self.manifest.description.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_loader_config_default() {
        let config = PluginLoaderConfig::default();
        assert_eq!(
            config.plugin_dir,
            PathBuf::from(super::super::DEFAULT_PLUGIN_DIR)
        );
        assert!(config.enabled);
        assert!(config.allowlist.is_empty());
        assert!(config.denylist.is_empty());
    }

    #[test]
    fn test_loader_config_disabled() {
        let config = PluginLoaderConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_loader_config_custom_dir() {
        let config = PluginLoaderConfig::with_dir("/custom/plugins");
        assert_eq!(config.plugin_dir, PathBuf::from("/custom/plugins"));
    }

    #[test]
    fn test_should_load_plugin_no_filters() {
        let config = PluginLoaderConfig::default();
        let loader = PluginLoader::new(config);

        assert!(loader.should_load_plugin("any-plugin"));
    }

    #[test]
    fn test_should_load_plugin_with_denylist() {
        let config = PluginLoaderConfig {
            denylist: vec!["blocked".to_string()],
            ..Default::default()
        };
        let loader = PluginLoader::new(config);

        assert!(loader.should_load_plugin("allowed"));
        assert!(!loader.should_load_plugin("blocked"));
    }

    #[test]
    fn test_should_load_plugin_with_allowlist() {
        let config = PluginLoaderConfig {
            allowlist: vec!["allowed".to_string()],
            ..Default::default()
        };
        let loader = PluginLoader::new(config);

        assert!(loader.should_load_plugin("allowed"));
        assert!(!loader.should_load_plugin("not-allowed"));
    }

    #[test]
    fn test_is_manifest_file() {
        let loader = PluginLoader::new(PluginLoaderConfig::default());

        assert!(loader.is_manifest_file(Path::new("plugin.toml")));
        assert!(loader.is_manifest_file(Path::new("plugin.yaml")));
        assert!(loader.is_manifest_file(Path::new("plugin.yml")));
        assert!(loader.is_manifest_file(Path::new("plugin.json")));
        assert!(!loader.is_manifest_file(Path::new("plugin.wasm")));
        assert!(!loader.is_manifest_file(Path::new("plugin.txt")));
    }

    #[test]
    fn test_load_result_helpers() {
        let mut result = LoadResult::new();
        result.loaded.push("plugin1".to_string());
        result.loaded.push("plugin2".to_string());

        assert_eq!(result.total(), 2);
        assert!(result.all_succeeded());
        assert!(result.any_succeeded());

        result
            .failed
            .push(("plugin3".to_string(), "error".to_string()));
        assert_eq!(result.total(), 3);
        assert!(!result.all_succeeded());
        assert!(result.any_succeeded());
    }

    #[tokio::test]
    async fn test_load_all_disabled() {
        let config = PluginLoaderConfig::disabled();
        let loader = PluginLoader::new(config);
        let mut registry = PluginRegistry::new();

        let result = loader.load_all(&mut registry).await.unwrap();

        assert!(result.disabled);
        assert!(result.loaded.is_empty());
    }

    #[tokio::test]
    async fn test_load_all_no_directory() {
        let config = PluginLoaderConfig::with_dir("/nonexistent/path");
        let loader = PluginLoader::new(config);
        let mut registry = PluginRegistry::new();

        let result = loader.load_all(&mut registry).await.unwrap();

        assert!(result.loaded.is_empty());
        assert!(result.failed.is_empty());
    }
}
