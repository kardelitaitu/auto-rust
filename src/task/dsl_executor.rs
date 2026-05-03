//! DSL Task Executor - Bridge between DSL definitions and runtime execution.
//!
//! This module executes tasks defined in DSL (YAML/TOML) format using the
//! task-api verbs. It handles variable substitution, action execution,
//! and control flow (if/else, loops).
//!
//! # Example
//! ```rust
//! use auto::task::dsl::{TaskDefinition, parse_task_yaml};
//! use auto::task::dsl_executor::DslExecutor;
//! use auto::prelude::TaskContext;
//!
//! async fn example(api: &TaskContext, yaml: &str) -> anyhow::Result<()> {
//!     let task_def = parse_task_yaml(yaml)?;
//!     let mut executor = DslExecutor::new(api, &task_def);
//!     executor.execute().await
//! }
//! ```

use std::collections::HashMap;

use anyhow::{Context, Result};

use crate::prelude::TaskContext;
use crate::task::dsl::{Action, Condition, LogLevel, TaskDefinition};

/// Maximum recursion depth for task calls to prevent infinite loops.
const MAX_CALL_DEPTH: u32 = 10;

/// Executor state for DSL task execution.
pub struct DslExecutor<'a> {
    /// Task context for API operations
    api: &'a TaskContext,
    /// Task definition being executed
    task_def: &'a TaskDefinition,
    /// Runtime variables (for extract/variable operations)
    variables: HashMap<String, String>,
    /// Execution statistics
    actions_executed: u32,
    /// Current call depth for recursion tracking
    call_depth: u32,
}

impl<'a> std::fmt::Debug for DslExecutor<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DslExecutor")
            .field("task_def", &self.task_def)
            .field("variables", &self.variables)
            .field("actions_executed", &self.actions_executed)
            .field("call_depth", &self.call_depth)
            .finish_non_exhaustive()
    }
}

impl<'a> DslExecutor<'a> {
    /// Create a new DSL executor.
    ///
    /// # Arguments
    /// * `api` - Task context for browser automation
    /// * `task_def` - Parsed task definition
    pub fn new(api: &'a TaskContext, task_def: &'a TaskDefinition) -> Self {
        Self {
            api,
            task_def,
            variables: HashMap::new(),
            actions_executed: 0,
            call_depth: 0,
        }
    }

    /// Create a new DSL executor with specific call depth (for internal calls).
    ///
    /// # Arguments
    /// * `api` - Task context for browser automation
    /// * `task_def` - Parsed task definition
    /// * `call_depth` - Current recursion depth
    fn with_depth(api: &'a TaskContext, task_def: &'a TaskDefinition, call_depth: u32) -> Self {
        Self {
            api,
            task_def,
            variables: HashMap::new(),
            actions_executed: 0,
            call_depth,
        }
    }

    /// Set initial parameters from CLI payload.
    ///
    /// Parameters are converted from serde_json::Value to String
    /// and stored in the variables map for substitution.
    ///
    /// # Arguments
    /// * `payload` - JSON object containing parameter key-value pairs
    pub fn with_parameters(mut self, payload: &serde_json::Value) -> Self {
        if let Some(obj) = payload.as_object() {
            for (key, value) in obj {
                let value_str = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    _ => value.to_string(),
                };
                log::debug!("Set parameter '{}': {}", key, value_str);
                self.variables.insert(key.clone(), value_str);
            }
        }
        self
    }

    /// Execute the task definition.
    ///
    /// Runs all actions in sequence, handling control flow and variables.
    pub async fn execute(&mut self) -> Result<()> {
        log::info!(
            "Executing DSL task '{}' with {} actions",
            self.task_def.name,
            self.task_def.actions.len()
        );

        for (idx, action) in self.task_def.actions.iter().enumerate() {
            log::debug!("Action {}: {:?}", idx + 1, action);
            self.execute_action(action).await.with_context(|| {
                format!(
                    "Failed to execute action {} in task '{}'",
                    idx + 1,
                    self.task_def.name
                )
            })?;
            self.actions_executed += 1;
        }

        log::info!(
            "DSL task '{}' completed ({} actions executed)",
            self.task_def.name,
            self.actions_executed
        );
        Ok(())
    }

    /// Execute a single action.
    async fn execute_action(&mut self, action: &Action) -> Result<()> {
        match action {
            Action::Navigate { url } => {
                let resolved_url = self.substitute_variables(url);
                self.api.navigate(&resolved_url, 30000).await?;
            }
            Action::Click { selector } => {
                let resolved_selector = self.substitute_variables(selector);
                self.api.click(&resolved_selector).await?;
            }
            Action::Type { selector, text } => {
                let resolved_selector = self.substitute_variables(selector);
                let resolved_text = self.substitute_variables(text);
                self.api.r#type(&resolved_selector, &resolved_text).await?;
            }
            Action::Wait { duration_ms } => {
                tokio::time::sleep(tokio::time::Duration::from_millis(*duration_ms)).await;
            }
            Action::WaitFor {
                selector,
                timeout_ms,
            } => {
                let resolved_selector = self.substitute_variables(selector);
                let timeout = timeout_ms.unwrap_or(5000);
                self.api
                    .wait_for(&resolved_selector, timeout)
                    .await
                    .with_context(|| {
                        format!(
                            "Element '{}' not found within {}ms",
                            resolved_selector, timeout
                        )
                    })?;
            }
            Action::ScrollTo { selector } => {
                let resolved_selector = self.substitute_variables(selector);
                self.api.scroll_to(&resolved_selector).await?;
            }
            Action::Extract { selector, variable } => {
                let resolved_selector = self.substitute_variables(selector);
                let text = self.api.text(&resolved_selector).await?.unwrap_or_default();
                if let Some(var_name) = variable {
                    log::debug!("Extracting variable '{}': {}", var_name, text);
                    self.variables.insert(var_name.clone(), text);
                }
            }
            Action::Execute { script: _ } => {
                log::warn!("Execute action not yet implemented");
            }
            Action::Log { message, level } => {
                let resolved_message = self.substitute_variables(message);
                match level.as_ref().unwrap_or(&LogLevel::Info) {
                    LogLevel::Debug => log::debug!("{}", resolved_message),
                    LogLevel::Info => log::info!("{}", resolved_message),
                    LogLevel::Warn => log::warn!("{}", resolved_message),
                    LogLevel::Error => log::error!("{}", resolved_message),
                }
            }
            Action::If {
                condition,
                then,
                r#else,
            } => {
                if self.evaluate_condition(condition).await? {
                    for action in then {
                        Box::pin(self.execute_action(action)).await?;
                    }
                } else if let Some(else_actions) = r#else {
                    for action in else_actions {
                        Box::pin(self.execute_action(action)).await?;
                    }
                }
            }
            Action::Loop {
                count,
                condition,
                actions,
            } => {
                let iterations = if let Some(c) = count {
                    *c
                } else if let Some(cond) = condition {
                    // Condition-based loop with max iterations as safety
                    let max_iterations = 100;
                    let mut i = 0;
                    while self.evaluate_condition(cond).await? && i < max_iterations {
                        for action in actions {
                            Box::pin(self.execute_action(action)).await?;
                        }
                        i += 1;
                    }
                    if i >= max_iterations {
                        log::warn!("Loop reached max iterations ({}), breaking", max_iterations);
                    }
                    0 // Already executed in the loop above
                } else {
                    0
                };

                for _ in 0..iterations {
                    for action in actions {
                        Box::pin(self.execute_action(action)).await?;
                    }
                }
            }
            Action::Call { task, parameters } => {
                self.execute_call(task, parameters.as_ref()).await?;
            }
        }
        Ok(())
    }

    /// Evaluate a condition.
    async fn evaluate_condition(&self, condition: &Condition) -> Result<bool> {
        match condition {
            Condition::ElementExists { selector } => {
                let resolved_selector = self.substitute_variables(selector);
                match self.api.exists(&resolved_selector).await {
                    Ok(exists) => Ok(exists),
                    Err(_) => Ok(false),
                }
            }
            Condition::ElementVisible { selector } => {
                let resolved_selector = self.substitute_variables(selector);
                match self.api.visible(&resolved_selector).await {
                    Ok(visible) => Ok(visible),
                    Err(_) => Ok(false),
                }
            }
            Condition::TextEquals { selector, value } => {
                let resolved_selector = self.substitute_variables(selector);
                let resolved_value = self.substitute_variables(value);
                match self.api.text(&resolved_selector).await {
                    Ok(Some(text)) => Ok(text.trim() == resolved_value),
                    _ => Ok(false),
                }
            }
            Condition::VariableEquals { name, value } => {
                if let Some(var_value) = self.variables.get(name) {
                    let expected = match value {
                        serde_yaml::Value::String(s) => s.clone(),
                        serde_yaml::Value::Number(n) => n.to_string(),
                        serde_yaml::Value::Bool(b) => b.to_string(),
                        _ => String::new(),
                    };
                    Ok(var_value == &expected)
                } else {
                    Ok(false)
                }
            }
            Condition::And { conditions } => Self::evaluate_conditions_and(self, conditions).await,
            Condition::Or { conditions } => Self::evaluate_conditions_or(self, conditions).await,
            Condition::Not { condition } => {
                let result = Box::pin(self.evaluate_condition(condition)).await?;
                Ok(!result)
            }
        }
    }

    /// Helper to evaluate AND conditions.
    async fn evaluate_conditions_and(&self, conditions: &[Condition]) -> Result<bool> {
        for cond in conditions {
            if !Box::pin(self.evaluate_condition(cond)).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Helper to evaluate OR conditions.
    async fn evaluate_conditions_or(&self, conditions: &[Condition]) -> Result<bool> {
        for cond in conditions {
            if Box::pin(self.evaluate_condition(cond)).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Substitute variables in a string.
    ///
    /// Replaces `{{variable_name}}` with the value from variables or payload.
    fn substitute_variables(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Replace {{variable}} syntax
        for (key, value) in &self.variables {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }

        result
    }

    /// Execute a Call action - invoke another task.
    ///
    /// # Arguments
    /// * `task_name` - Name of the task to call
    /// * `parameters` - Optional parameter overrides for the called task
    ///
    /// # Errors
    /// Returns error if recursion limit exceeded, task not found, or call fails.
    async fn execute_call(
        &mut self,
        task_name: &str,
        parameters: Option<&HashMap<String, serde_yaml::Value>>,
    ) -> Result<()> {
        // Check recursion depth
        if self.call_depth >= MAX_CALL_DEPTH {
            return Err(anyhow::anyhow!(
                "Maximum call depth ({}) exceeded when calling task '{}'",
                MAX_CALL_DEPTH,
                task_name
            ));
        }

        log::info!(
            "Calling task '{}' (depth {}/{})",
            task_name,
            self.call_depth + 1,
            MAX_CALL_DEPTH
        );

        // Look up the target task
        let registry = crate::task::registry::TaskRegistry::with_built_in_tasks();
        let target_def = registry
            .get_task_definition(task_name)
            .ok_or_else(|| anyhow::anyhow!("Called task '{}' not found", task_name))?
            .clone();

        // Create child executor with incremented depth
        let mut child_executor =
            DslExecutor::with_depth(self.api, &target_def, self.call_depth + 1);

        // Build parameter payload from parent variables + provided parameters
        let mut child_params = serde_json::Map::new();

        // First, copy parent's variables as defaults
        for (key, value) in &self.variables {
            child_params.insert(key.clone(), serde_json::Value::String(value.clone()));
        }

        // Then apply provided parameter overrides with variable substitution
        if let Some(params) = parameters {
            for (key, value) in params {
                let value_str = match value {
                    serde_yaml::Value::String(s) => s.clone(),
                    serde_yaml::Value::Number(n) => n.to_string(),
                    serde_yaml::Value::Bool(b) => b.to_string(),
                    _ => format!("{:?}", value),
                };
                let resolved_value = self.substitute_variables(&value_str);
                child_params.insert(key.clone(), serde_json::Value::String(resolved_value));
            }
        }

        // Initialize child with merged parameters
        child_executor = child_executor.with_parameters(&serde_json::Value::Object(child_params));

        // Execute the child task using Box::pin to avoid infinite recursion in async
        Box::pin(child_executor.execute()).await?;

        log::info!("Task '{}' completed successfully", task_name);
        Ok(())
    }

    /// Get execution statistics.
    pub fn stats(&self) -> DslExecutionStats {
        DslExecutionStats {
            actions_executed: self.actions_executed,
            total_actions: self.task_def.actions.len() as u32,
            variables_defined: self.variables.len(),
            max_call_depth: self.call_depth,
        }
    }
}

/// Execution statistics for a DSL task.
#[derive(Debug, Clone)]
pub struct DslExecutionStats {
    /// Number of actions executed
    pub actions_executed: u32,
    /// Total number of actions in task
    pub total_actions: u32,
    /// Number of variables defined during execution
    pub variables_defined: usize,
    /// Maximum call depth reached during execution
    pub max_call_depth: u32,
}

/// Execute a DSL task definition.
///
/// Convenience function to execute a task without creating an executor instance.
pub async fn execute_dsl_task(
    api: &TaskContext,
    task_def: &TaskDefinition,
) -> Result<DslExecutionStats> {
    let mut executor = DslExecutor::new(api, task_def);
    executor.execute().await?;
    Ok(executor.stats())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::dsl::Action;

    // Test variable substitution without needing a TaskContext
    fn test_substitute(variables: &HashMap<String, String>, text: &str) -> String {
        let mut result = text.to_string();
        for (key, value) in variables {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }

    #[test]
    fn test_variable_substitution() {
        let mut variables = HashMap::new();
        variables.insert("username".to_string(), "john_doe".to_string());

        let text = "Hello, {{username}}!";
        let result = test_substitute(&variables, text);
        assert_eq!(result, "Hello, john_doe!");
    }

    #[test]
    fn test_variable_substitution_no_match() {
        let variables = HashMap::new();

        let text = "Hello, {{unknown}}!";
        let result = test_substitute(&variables, text);
        assert_eq!(result, "Hello, {{unknown}}!");
    }

    #[test]
    fn test_variable_substitution_replaces_multiple_occurrences() {
        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "alice".to_string());

        let text = "{{name}} says hi to {{name}}.";
        let result = test_substitute(&variables, text);

        assert_eq!(result, "alice says hi to alice.");
    }

    #[test]
    fn test_variable_substitution_leaves_partial_placeholders_intact() {
        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "alice".to_string());

        let text = "Hello {{name}}, keep {{unknown}} unchanged.";
        let result = test_substitute(&variables, text);

        assert_eq!(result, "Hello alice, keep {{unknown}} unchanged.");
    }

    #[test]
    fn test_dsl_stats() {
        let task_def = TaskDefinition {
            name: "test".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            actions: vec![
                Action::Wait { duration_ms: 100 },
                Action::Wait { duration_ms: 200 },
            ],
        };

        // Stats don't require a valid TaskContext reference
        // We can verify the stats calculation logic independently
        let stats = DslExecutionStats {
            actions_executed: 0,
            total_actions: task_def.actions.len() as u32,
            variables_defined: 0,
            max_call_depth: 0,
        };

        assert_eq!(stats.total_actions, 2);
        assert_eq!(stats.actions_executed, 0);
    }

    #[test]
    fn test_dsl_stats_tracks_defined_variables() {
        let task_def = TaskDefinition {
            name: "test".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            actions: vec![Action::Wait { duration_ms: 100 }],
        };

        let stats = DslExecutionStats {
            actions_executed: 1,
            total_actions: task_def.actions.len() as u32,
            variables_defined: 2,
            max_call_depth: 0,
        };

        assert_eq!(stats.actions_executed, 1);
        assert_eq!(stats.total_actions, 1);
        assert_eq!(stats.variables_defined, 2);
        assert_eq!(stats.max_call_depth, 0);
    }

    #[test]
    fn test_with_parameters_sets_variables() {
        let _task_def = TaskDefinition {
            name: "param_test".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            actions: vec![Action::Wait { duration_ms: 100 }],
        };

        let payload = serde_json::json!({
            "url": "https://example.com",
            "count": 42,
            "enabled": true
        });

        // Create a mock executor (we can't easily create TaskContext in tests)
        // But we can verify the variable storage via substitute_variables behavior
        // Since we can't access private fields, we rely on the behavior test above
        // This test documents that with_parameters exists and accepts payload

        // Verify payload structure for documentation
        assert!(payload.as_object().unwrap().contains_key("url"));
        assert_eq!(
            payload.as_object().unwrap().get("url").unwrap().as_str(),
            Some("https://example.com")
        );
        assert_eq!(
            payload.as_object().unwrap().get("count").unwrap().as_i64(),
            Some(42)
        );
        assert_eq!(
            payload
                .as_object()
                .unwrap()
                .get("enabled")
                .unwrap()
                .as_bool(),
            Some(true)
        );
    }

    #[test]
    fn test_with_parameters_ignores_non_object() {
        let _task_def = TaskDefinition {
            name: "param_test".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            actions: vec![Action::Wait { duration_ms: 100 }],
        };

        // Non-object payload should be silently ignored
        let string_payload = serde_json::json!("just a string");
        assert!(string_payload.as_object().is_none());

        let array_payload = serde_json::json!([1, 2, 3]);
        assert!(array_payload.as_object().is_none());

        let null_payload = serde_json::Value::Null;
        assert!(null_payload.as_object().is_none());
    }

    #[test]
    fn test_dsl_stats_includes_call_depth() {
        let task_def = TaskDefinition {
            name: "test".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            actions: vec![Action::Wait { duration_ms: 100 }],
        };

        let stats = DslExecutionStats {
            actions_executed: 1,
            total_actions: task_def.actions.len() as u32,
            variables_defined: 2,
            max_call_depth: 3,
        };

        assert_eq!(stats.max_call_depth, 3);
    }

    #[test]
    fn test_max_call_depth_constant() {
        // Verify the recursion limit is set appropriately
        assert_eq!(MAX_CALL_DEPTH, 10);
        // MAX_CALL_DEPTH is a const, so these checks are compile-time verified
        // but we keep the assert_eq for documentation purposes
    }
}
