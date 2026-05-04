//! Plugin trait and context definitions
//!
//! Plugins are WASM modules that implement the Plugin trait
//! and can hook into various lifecycle events.

use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Context passed to plugins during execution
#[derive(Debug, Clone)]
pub struct PluginContext {
    /// Current task name
    pub task_name: String,
    /// Task parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Current page URL (if applicable)
    pub current_url: Option<String>,
    /// Session ID
    pub session_id: String,
    /// Execution start time
    pub started_at: std::time::Instant,
}

impl PluginContext {
    /// Create a new plugin context
    pub fn new(task_name: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            task_name: task_name.into(),
            parameters: HashMap::new(),
            current_url: None,
            session_id: session_id.into(),
            started_at: std::time::Instant::now(),
        }
    }

    /// Add a parameter to the context
    pub fn with_parameter(mut self, key: impl Into<String>, value: impl Serialize) -> Result<Self> {
        self.parameters.insert(
            key.into(),
            serde_json::to_value(value)?,
        );
        Ok(self)
    }
}

/// Hook points where plugins can intercept execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginHook {
    /// Called before task execution starts
    BeforeTask,
    /// Called after task execution completes
    AfterTask,
    /// Called before each action
    BeforeAction,
    /// Called after each action
    AfterAction,
    /// Called on error
    OnError,
    /// Called for custom validation
    Validate,
    /// Called for custom action execution
    CustomAction,
}

/// Result of a plugin hook execution
#[derive(Debug, Clone)]
pub enum HookResult {
    /// Continue with normal execution
    Continue,
    /// Skip the current operation
    Skip,
    /// Replace the current operation with custom logic
    Replace(serde_json::Value),
    /// Abort execution with error
    Abort(String),
}

/// Core trait for all plugins
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Plugin name (must be unique)
    fn name(&self) -> &str;

    /// Plugin version
    fn version(&self) -> &str;

    /// Plugin author
    fn author(&self) -> Option<&str> {
        None
    }

    /// Plugin description
    fn description(&self) -> Option<&str> {
        None
    }

    /// Initialize the plugin with configuration
    async fn initialize(&mut self, config: &serde_json::Value) -> Result<()> {
        let _ = config;
        Ok(())
    }

    /// Shutdown the plugin
    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }

    /// Check if plugin supports a specific hook
    fn supports_hook(&self, hook: PluginHook) -> bool {
        let _ = hook;
        false
    }

    /// Execute a hook
    async fn execute_hook(
        &self,
        hook: PluginHook,
        context: &PluginContext,
        data: &serde_json::Value,
    ) -> Result<HookResult> {
        let _ = (hook, context, data);
        Ok(HookResult::Continue)
    }

    /// Get custom action names this plugin provides
    fn custom_actions(&self) -> Vec<String> {
        vec![]
    }

    /// Execute a custom action
    async fn execute_custom_action(
        &self,
        action_name: &str,
        context: &PluginContext,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let _ = (action_name, context, params);
        Err(anyhow::anyhow!("Custom action not implemented"))
    }
}

/// Boxed plugin type for storage
pub type BoxedPlugin = Box<dyn Plugin>;

/// Plugin metadata without the plugin instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Plugin description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Supported hooks
    pub supported_hooks: Vec<PluginHook>,
    /// Custom actions provided
    pub custom_actions: Vec<String>,
}

impl PluginInfo {
    /// Create plugin info from a plugin instance
    pub fn from_plugin(plugin: &dyn Plugin) -> Self {
        Self {
            name: plugin.name().to_string(),
            version: plugin.version().to_string(),
            author: plugin.author().map(|s| s.to_string()),
            description: plugin.description().map(|s| s.to_string()),
            supported_hooks: vec![], // Would need to query plugin
            custom_actions: plugin.custom_actions(),
        }
    }
}
