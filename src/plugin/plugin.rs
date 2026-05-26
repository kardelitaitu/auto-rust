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
        self.parameters
            .insert(key.into(), serde_json::to_value(value)?);
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
            author: plugin.author().map(std::string::ToString::to_string),
            description: plugin.description().map(std::string::ToString::to_string),
            supported_hooks: vec![], // Would need to query plugin
            custom_actions: plugin.custom_actions(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ========================================================================
    // PluginContext Tests
    // ========================================================================

    #[test]
    fn test_plugin_context_new() {
        let ctx = PluginContext::new("test-task", "session-1");
        assert_eq!(ctx.task_name, "test-task");
        assert_eq!(ctx.session_id, "session-1");
        assert!(ctx.parameters.is_empty());
        assert!(ctx.current_url.is_none());
    }

    #[test]
    fn test_plugin_context_new_from_strings() {
        let task = String::from("pageview");
        let session = String::from("brave-9001");
        let ctx = PluginContext::new(task, session);
        assert_eq!(ctx.task_name, "pageview");
        assert_eq!(ctx.session_id, "brave-9001");
    }

    #[test]
    fn test_plugin_context_with_parameter() {
        let ctx = PluginContext::new("test", "s1")
            .with_parameter("key1", "value1")
            .expect("add parameter");
        assert_eq!(ctx.parameters.len(), 1);
        assert_eq!(ctx.parameters.get("key1"), Some(&json!("value1")));
    }

    #[test]
    fn test_plugin_context_with_multiple_parameters() {
        let ctx = PluginContext::new("test", "s1")
            .with_parameter("name", "test-plugin")
            .expect("add name")
            .with_parameter("count", 42)
            .expect("add count")
            .with_parameter("enabled", true)
            .expect("add enabled");
        assert_eq!(ctx.parameters.len(), 3);
        assert_eq!(ctx.parameters.get("name"), Some(&json!("test-plugin")));
        assert_eq!(ctx.parameters.get("count"), Some(&json!(42)));
        assert_eq!(ctx.parameters.get("enabled"), Some(&json!(true)));
    }

    #[test]
    fn test_plugin_context_with_parameter_overwrites() {
        let ctx = PluginContext::new("test", "s1")
            .with_parameter("key", "first")
            .expect("add first")
            .with_parameter("key", "second")
            .expect("overwrite");
        assert_eq!(ctx.parameters.len(), 1);
        assert_eq!(ctx.parameters.get("key"), Some(&json!("second")));
    }

    #[test]
    fn test_plugin_context_debug() {
        let ctx = PluginContext::new("test", "s1");
        let debug = format!("{:?}", ctx);
        assert!(debug.contains("test"));
        assert!(debug.contains("s1"));
    }

    #[test]
    fn test_plugin_context_clone() {
        let ctx = PluginContext::new("task", "session")
            .with_parameter("key", "value")
            .expect("add param");
        let cloned = ctx.clone();
        assert_eq!(cloned.task_name, ctx.task_name);
        assert_eq!(cloned.parameters, ctx.parameters);
        assert_eq!(cloned.current_url, ctx.current_url);
    }

    // ========================================================================
    // PluginHook Tests
    // ========================================================================

    #[test]
    fn test_plugin_hook_variants() {
        assert_eq!(PluginHook::BeforeTask as u8, 0);
        assert_eq!(PluginHook::AfterTask as u8, 1);
        assert_eq!(PluginHook::BeforeAction as u8, 2);
        assert_eq!(PluginHook::AfterAction as u8, 3);
        assert_eq!(PluginHook::OnError as u8, 4);
        assert_eq!(PluginHook::Validate as u8, 5);
        assert_eq!(PluginHook::CustomAction as u8, 6);
    }

    #[test]
    fn test_plugin_hook_debug() {
        assert_eq!(format!("{:?}", PluginHook::BeforeTask), "BeforeTask");
        assert_eq!(format!("{:?}", PluginHook::OnError), "OnError");
    }

    #[test]
    fn test_plugin_hook_partial_eq() {
        assert_eq!(PluginHook::BeforeTask, PluginHook::BeforeTask);
        assert_ne!(PluginHook::BeforeTask, PluginHook::AfterTask);
    }

    #[test]
    fn test_plugin_hook_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PluginHook::BeforeTask);
        set.insert(PluginHook::AfterTask);
        set.insert(PluginHook::BeforeTask); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_plugin_hook_serialize_snake_case() {
        let json = serde_json::to_string(&PluginHook::BeforeTask).unwrap();
        assert_eq!(json, "\"before_task\"");

        let json = serde_json::to_string(&PluginHook::CustomAction).unwrap();
        assert_eq!(json, "\"custom_action\"");
    }

    #[test]
    fn test_plugin_hook_deserialize() {
        let hook: PluginHook = serde_json::from_str("\"before_task\"").unwrap();
        assert_eq!(hook, PluginHook::BeforeTask);

        let hook: PluginHook = serde_json::from_str("\"after_action\"").unwrap();
        assert_eq!(hook, PluginHook::AfterAction);
    }

    // ========================================================================
    // HookResult Tests
    // ========================================================================

    #[test]
    fn test_hook_result_continue() {
        let result = HookResult::Continue;
        assert!(matches!(result, HookResult::Continue));
    }

    #[test]
    fn test_hook_result_skip() {
        let result = HookResult::Skip;
        assert!(matches!(result, HookResult::Skip));
    }

    #[test]
    fn test_hook_result_replace() {
        let result = HookResult::Replace(json!({"action": "custom"}));
        if let HookResult::Replace(value) = result {
            assert_eq!(value, json!({"action": "custom"}));
        } else {
            panic!("Expected Replace variant");
        }
    }

    #[test]
    fn test_hook_result_abort() {
        let result = HookResult::Abort("fatal error".to_string());
        if let HookResult::Abort(msg) = result {
            assert_eq!(msg, "fatal error");
        } else {
            panic!("Expected Abort variant");
        }
    }

    // ========================================================================
    // Plugin Trait Implementation Tests (via TestPlugin)
    // ========================================================================

    struct TestPlugin {
        name: String,
        version: String,
        custom_actions: Vec<String>,
    }

    #[async_trait]
    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> &str {
            &self.version
        }

        fn author(&self) -> Option<&str> {
            Some("test-author")
        }

        fn description(&self) -> Option<&str> {
            Some("A test plugin")
        }

        fn custom_actions(&self) -> Vec<String> {
            self.custom_actions.clone()
        }

        fn supports_hook(&self, hook: PluginHook) -> bool {
            matches!(hook, PluginHook::BeforeTask | PluginHook::AfterTask)
        }
    }

    #[test]
    fn test_plugin_trait_methods() {
        let plugin = TestPlugin {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            custom_actions: vec!["action1".to_string()],
        };

        assert_eq!(plugin.name(), "test-plugin");
        assert_eq!(plugin.version(), "1.0.0");
        assert_eq!(plugin.author(), Some("test-author"));
        assert_eq!(plugin.description(), Some("A test plugin"));
        assert!(plugin.supports_hook(PluginHook::BeforeTask));
        assert!(!plugin.supports_hook(PluginHook::OnError));
    }

    #[tokio::test]
    async fn test_plugin_initialize_default() {
        let mut plugin = TestPlugin {
            name: "test".to_string(),
            version: "1.0".to_string(),
            custom_actions: vec![],
        };
        let result = plugin.initialize(&json!({"key": "value"})).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_plugin_shutdown_default() {
        let mut plugin = TestPlugin {
            name: "test".to_string(),
            version: "1.0".to_string(),
            custom_actions: vec![],
        };
        let result = plugin.shutdown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_plugin_execute_hook_default() {
        let plugin = TestPlugin {
            name: "test".to_string(),
            version: "1.0".to_string(),
            custom_actions: vec![],
        };
        let ctx = PluginContext::new("task", "session");
        let result = plugin
            .execute_hook(PluginHook::BeforeTask, &ctx, &json!({}))
            .await;
        assert!(matches!(result.unwrap(), HookResult::Continue));
    }

    #[tokio::test]
    async fn test_plugin_custom_action_default_error() {
        let plugin = TestPlugin {
            name: "test".to_string(),
            version: "1.0".to_string(),
            custom_actions: vec![],
        };
        let ctx = PluginContext::new("task", "session");
        let result = plugin
            .execute_custom_action("nonexistent", &ctx, &json!({}))
            .await;
        assert!(result.is_err());
    }

    // ========================================================================
    // PluginInfo Tests
    // ========================================================================

    #[test]
    fn test_plugin_info_from_plugin() {
        let plugin = TestPlugin {
            name: "my-plugin".to_string(),
            version: "2.0.0".to_string(),
            custom_actions: vec!["action1".to_string(), "action2".to_string()],
        };
        let info = PluginInfo::from_plugin(&plugin);
        assert_eq!(info.name, "my-plugin");
        assert_eq!(info.version, "2.0.0");
        assert_eq!(info.author, Some("test-author".to_string()));
        assert_eq!(info.description, Some("A test plugin".to_string()));
        assert_eq!(info.custom_actions.len(), 2);
    }

    #[test]
    fn test_plugin_info_serialize() {
        let info = PluginInfo {
            name: "test".to_string(),
            version: "1.0".to_string(),
            author: Some("author".to_string()),
            description: None,
            supported_hooks: vec![],
            custom_actions: vec![],
        };
        let json = serde_json::to_string(&info).expect("serialize");
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"version\":\"1.0\""));
        assert!(json.contains("\"author\":\"author\""));
        // description is None and skip_serializing_if, so it should be absent
        assert!(!json.contains("\"description\""));
    }

    #[test]
    fn test_plugin_info_deserialize() {
        let json =
            r#"{"name":"p","version":"1","author":"a","supported_hooks":[],"custom_actions":[]}"#;
        let info: PluginInfo = serde_json::from_str(json).expect("deserialize");
        assert_eq!(info.name, "p");
        assert_eq!(info.version, "1");
        assert_eq!(info.author, Some("a".to_string()));
    }

    #[test]
    fn test_plugin_info_debug() {
        let info = PluginInfo {
            name: "test".to_string(),
            version: "1.0".to_string(),
            author: None,
            description: None,
            supported_hooks: vec![],
            custom_actions: vec![],
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_boxed_plugin_type() {
        let plugin = TestPlugin {
            name: "boxed".to_string(),
            version: "1.0".to_string(),
            custom_actions: vec![],
        };
        let boxed: BoxedPlugin = Box::new(plugin);
        assert_eq!(boxed.name(), "boxed");
    }
}
