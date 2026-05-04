//! Plugin Registry
//!
//! Manages loaded plugins and routes hook executions.

use std::collections::HashMap;

use anyhow::Result;

use super::{plugin::{BoxedPlugin, HookResult, PluginContext, PluginHook, PluginInfo}, manifest::PluginManifest};

/// Errors that can occur in the plugin registry
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    /// Plugin with this name already exists
    #[error("Plugin '{0}' is already registered")]
    DuplicatePlugin(String),

    /// Plugin not found
    #[error("Plugin '{0}' not found")]
    PluginNotFound(String),

    /// Hook execution failed
    #[error("Hook execution failed: {0}")]
    HookExecutionFailed(String),

    /// Invalid plugin configuration
    #[error("Invalid plugin configuration: {0}")]
    InvalidConfiguration(String),

    /// Dependency not satisfied
    #[error("Plugin dependency '{0}' not satisfied")]
    DependencyNotSatisfied(String),
}

/// Registry for managing loaded plugins
pub struct PluginRegistry {
    /// Loaded plugins by name
    plugins: HashMap<String, BoxedPlugin>,
    /// Plugin manifests by name
    manifests: HashMap<String, PluginManifest>,
    /// Plugin load order
    load_order: Vec<String>,
    /// Whether registry is initialized
    initialized: bool,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    /// Create a new empty plugin registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            manifests: HashMap::new(),
            load_order: Vec::new(),
            initialized: false,
        }
    }

    /// Register a plugin with its manifest
    pub fn register(
        &mut self,
        manifest: PluginManifest,
        plugin: BoxedPlugin,
    ) -> Result<(), RegistryError> {
        let name = manifest.name.clone();

        if self.plugins.contains_key(&name) {
            return Err(RegistryError::DuplicatePlugin(name));
        }

        // Check dependencies
        for dep in &manifest.dependencies {
            if !self.plugins.contains_key(&dep.name) {
                return Err(RegistryError::DependencyNotSatisfied(dep.name.clone()));
            }
        }

        self.manifests.insert(name.clone(), manifest);
        self.plugins.insert(name.clone(), plugin);
        self.load_order.push(name);

        log::info!("Registered plugin: {}", name);
        Ok(())
    }

    /// Unregister a plugin
    pub fn unregister(&mut self, name: &str) -> Result<(), RegistryError> {
        if !self.plugins.contains_key(name) {
            return Err(RegistryError::PluginNotFound(name.to_string()));
        }

        self.plugins.remove(name);
        self.manifests.remove(name);
        self.load_order.retain(|n| n != name);

        log::info!("Unregistered plugin: {}", name);
        Ok(())
    }

    /// Get a plugin by name
    pub fn get(&self, name: &str) -> Option<&dyn super::Plugin> {
        self.plugins.get(name).map(|p| p.as_ref())
    }

    /// Get a mutable plugin by name
    pub fn get_mut(&mut self, name: &str) -> Option<&mut BoxedPlugin> {
        self.plugins.get_mut(name)
    }

    /// Check if a plugin is registered
    pub fn contains(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }

    /// Get all registered plugin names
    pub fn plugin_names(&self) -> Vec<String> {
        self.load_order.clone()
    }

    /// Get plugin info for all registered plugins
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.load_order
            .iter()
            .filter_map(|name| {
                self.plugins.get(name).map(|p| PluginInfo::from_plugin(p.as_ref()))
            })
            .collect()
    }

    /// Get manifest for a plugin
    pub fn get_manifest(&self, name: &str) -> Option<&PluginManifest> {
        self.manifests.get(name)
    }

    /// Initialize all plugins
    pub async fn initialize_all(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        for name in &self.load_order {
            if let Some(manifest) = self.manifests.get(name) {
                let config = manifest.config.clone().unwrap_or(serde_json::Value::Null);

                if let Some(plugin) = self.plugins.get_mut(name) {
                    plugin.initialize(&config).await.map_err(|e| {
                        RegistryError::InvalidConfiguration(format!(
                            "Failed to initialize plugin '{}': {}",
                            name, e
                        ))
                    })?;
                }
            }
        }

        self.initialized = true;
        log::info!("Initialized {} plugins", self.plugins.len());
        Ok(())
    }

    /// Shutdown all plugins
    pub async fn shutdown_all(&mut self) -> Result<()> {
        // Shutdown in reverse load order
        for name in self.load_order.iter().rev() {
            if let Some(plugin) = self.plugins.get_mut(name) {
                if let Err(e) = plugin.shutdown().await {
                    log::warn!("Error shutting down plugin '{}': {}", name, e);
                }
            }
        }

        self.initialized = false;
        log::info!("Shutdown all plugins");
        Ok(())
    }

    /// Execute a hook on all plugins that support it
    pub async fn execute_hook(
        &self,
        hook: PluginHook,
        context: &PluginContext,
        data: &serde_json::Value,
    ) -> Result<HookResult, RegistryError> {
        for name in &self.load_order {
            if let Some(plugin) = self.plugins.get(name) {
                if !plugin.supports_hook(hook) {
                    continue;
                }

                match plugin.execute_hook(hook, context, data).await {
                    Ok(HookResult::Continue) => continue,
                    Ok(HookResult::Skip) => {
                        log::debug!("Plugin '{}' requested skip", name);
                        return Ok(HookResult::Skip);
                    }
                    Ok(HookResult::Replace(replacement)) => {
                        log::debug!("Plugin '{}' requested replace", name);
                        return Ok(HookResult::Replace(replacement));
                    }
                    Ok(HookResult::Abort(reason)) => {
                        log::warn!("Plugin '{}' requested abort: {}", name, reason);
                        return Ok(HookResult::Abort(reason));
                    }
                    Err(e) => {
                        log::error!("Plugin '{}' hook execution failed: {}", name, e);
                        return Err(RegistryError::HookExecutionFailed(format!(
                            "Plugin '{}': {}",
                            name, e
                        )));
                    }
                }
            }
        }

        Ok(HookResult::Continue)
    }

    /// Execute a custom action from a plugin
    pub async fn execute_custom_action(
        &self,
        plugin_name: &str,
        action_name: &str,
        context: &PluginContext,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, RegistryError> {
        let plugin = self
            .plugins
            .get(plugin_name)
            .ok_or_else(|| RegistryError::PluginNotFound(plugin_name.to_string()))?;

        plugin
            .execute_custom_action(action_name, context, params)
            .await
            .map_err(|e| RegistryError::HookExecutionFailed(e.to_string()))
    }

    /// Get the number of registered plugins
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Check if registry is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::plugin::{Plugin, HookResult, PluginContext, PluginHook};

    struct TestPlugin {
        name: String,
        version: String,
    }

    #[async_trait::async_trait]
    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> &str {
            &self.version
        }

        fn supports_hook(&self, hook: PluginHook) -> bool {
            matches!(hook, PluginHook::BeforeTask | PluginHook::AfterTask)
        }

        async fn execute_hook(
            &self,
            _hook: PluginHook,
            _context: &PluginContext,
            _data: &serde_json::Value,
        ) -> anyhow::Result<HookResult> {
            Ok(HookResult::Continue)
        }
    }

    #[test]
    fn test_registry_new_empty() {
        let registry = PluginRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = PluginRegistry::new();

        let manifest = PluginManifest {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            api_version: super::super::PLUGIN_API_VERSION.to_string(),
            author: None,
            description: None,
            entry_point: "test.wasm".to_string(),
            dependencies: vec![],
            capabilities: vec![],
            config: None,
        };

        let plugin = Box::new(TestPlugin {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
        });

        registry.register(manifest, plugin as BoxedPlugin).unwrap();
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test-plugin"));
    }

    #[test]
    fn test_registry_duplicate_plugin_error() {
        let mut registry = PluginRegistry::new();

        let manifest1 = PluginManifest {
            name: "duplicate".to_string(),
            version: "1.0.0".to_string(),
            api_version: super::super::PLUGIN_API_VERSION.to_string(),
            author: None,
            description: None,
            entry_point: "a.wasm".to_string(),
            dependencies: vec![],
            capabilities: vec![],
            config: None,
        };

        let manifest2 = PluginManifest {
            name: "duplicate".to_string(),
            version: "2.0.0".to_string(),
            api_version: super::super::PLUGIN_API_VERSION.to_string(),
            author: None,
            description: None,
            entry_point: "b.wasm".to_string(),
            dependencies: vec![],
            capabilities: vec![],
            config: None,
        };

        let plugin1 = Box::new(TestPlugin {
            name: "duplicate".to_string(),
            version: "1.0.0".to_string(),
        });

        let plugin2 = Box::new(TestPlugin {
            name: "duplicate".to_string(),
            version: "2.0.0".to_string(),
        });

        registry.register(manifest1, plugin1 as BoxedPlugin).unwrap();
        let result = registry.register(manifest2, plugin2 as BoxedPlugin);
        assert!(matches!(result, Err(RegistryError::DuplicatePlugin(_))));
    }
}
