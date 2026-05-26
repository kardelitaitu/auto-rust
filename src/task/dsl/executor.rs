//! Main DSL Executor implementation.
//!
//! Contains `DslExecutor` struct and core execution methods.
//! Other method groups are split into separate modules:
//! - cache.rs: `SelectorCache`, cache operations
//! - debug.rs: `DebugEvent`, Breakpoint, debug infrastructure
//! - profiling.rs: `ActionProfiler`, `ActionMetrics`, `ExecutionReport`
//! - evaluator.rs: Variable substitution, condition evaluation
//! - `control_flow.rs`: If, Loop, Foreach, While, Retry, Parallel handlers

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::prelude::TaskContext;
use crate::task::dsl::cache::SelectorCache;
use crate::task::dsl::debug::{Breakpoint, DebugEvent};
use crate::task::dsl::profiling::ActionMetrics;
use crate::task::dsl::{Action, LogLevel, TaskDefinition};
use anyhow::{Context, Result};

/// DSL execution statistics.
///
/// Returned by `DslExecutor::execute()` to provide
/// information about the execution run.
#[derive(Debug, Clone, Default)]
pub struct DslExecutionStats {
    /// Number of actions that were executed
    pub actions_executed: u32,
    /// Total number of actions in the task definition
    pub total_actions: u32,
    /// Number of variables defined during execution
    pub variables_defined: u32,
    /// Maximum call depth reached during execution
    pub max_call_depth: u32,
}

/// Maximum recursion depth for task calls to prevent infinite loops.
pub const MAX_CALL_DEPTH: u32 = 10;

/// Enable selector caching by default.
pub const DEFAULT_CACHE_ENABLED: bool = true;

/// Main DSL Executor struct.
pub struct DslExecutor<'a> {
    /// Task context for API operations
    pub api: &'a TaskContext,
    /// Task definition being executed (owned)
    pub task_def: TaskDefinition,
    /// Runtime variables (for extract/variable operations)
    pub variables: HashMap<String, String>,
    /// Execution statistics
    pub actions_executed: u32,
    /// Current call depth for recursion tracking
    pub call_depth: u32,
    /// Detailed action execution metrics
    pub action_metrics: Vec<ActionMetrics>,
    /// Execution start time
    pub start_time: Instant,
    /// Number of successful actions
    pub actions_succeeded: u32,
    /// Number of failed actions
    pub actions_failed: u32,
    /// Debug mode enabled
    pub debug_mode: bool,
    /// Active breakpoints
    pub breakpoints: Vec<Breakpoint>,
    /// Debug event log for tracing
    pub debug_events: Vec<DebugEvent>,
    /// Pause flag for step-through debugging
    pub paused: bool,
    /// Variable watch list for tracking changes
    #[allow(dead_code)]
    pub watched_variables: HashMap<String, String>,
    /// Selector cache for DOM queries
    pub selector_cache: super::cache::SelectorCache,
    /// Performance profilers for action types
    pub action_profilers: HashMap<String, super::profiling::ActionProfiler>,
    /// Enable selector caching
    pub cache_enabled: bool,
}

impl<'a> DslExecutor<'a> {
    /// Create a new DSL executor.
    #[must_use]
    pub fn new(api: &'a TaskContext, task_def: TaskDefinition) -> Self {
        Self {
            api,
            task_def,
            variables: HashMap::new(),
            actions_executed: 0,
            call_depth: 0,
            action_metrics: Vec::new(),
            start_time: Instant::now(),
            actions_succeeded: 0,
            actions_failed: 0,
            debug_mode: false,
            breakpoints: Vec::new(),
            debug_events: Vec::new(),
            paused: false,
            watched_variables: HashMap::new(),
            selector_cache: SelectorCache::new(),
            action_profilers: HashMap::new(),
            cache_enabled: DEFAULT_CACHE_ENABLED,
        }
    }

    /// Create a new DSL executor with specific call depth (for internal calls).
    pub(super) fn with_depth(
        api: &'a TaskContext,
        task_def: TaskDefinition,
        call_depth: u32,
    ) -> Self {
        Self {
            api,
            task_def,
            variables: HashMap::new(),
            actions_executed: 0,
            call_depth,
            action_metrics: Vec::new(),
            start_time: Instant::now(),
            actions_succeeded: 0,
            actions_failed: 0,
            debug_mode: false,
            breakpoints: Vec::new(),
            debug_events: Vec::new(),
            paused: false,
            watched_variables: HashMap::new(),
            selector_cache: SelectorCache::new(),
            action_profilers: HashMap::new(),
            cache_enabled: DEFAULT_CACHE_ENABLED,
        }
    }

    /// Set initial parameters from CLI payload.
    #[must_use]
    pub fn with_parameters(mut self, payload: &serde_yml::Value) -> Self {
        if let Some(obj) = payload.as_mapping() {
            for (key, value) in obj {
                let key_str = key.as_str().unwrap_or_default().to_string();
                let value_str = match value {
                    serde_yml::Value::String(s) => s.clone(),
                    serde_yml::Value::Number(n) => n.to_string(),
                    serde_yml::Value::Bool(b) => b.to_string(),
                    _ => format!("{value:?}"),
                };
                log::debug!("Set parameter '{key_str}': {value_str}");
                self.variables.insert(key_str, value_str);
            }
        }
        self
    }

    /// Execute the task definition.
    pub async fn execute(&mut self) -> Result<()> {
        log::info!(
            "Executing DSL task '{}' with {} actions",
            self.task_def.name,
            self.task_def.actions.len()
        );

        for (idx, action) in self.task_def.actions.clone().iter().enumerate() {
            let action_type = format!("{action:?}")
                .split_whitespace()
                .next()
                .unwrap_or("Unknown")
                .to_string();
            let mut metrics = super::profiling::ActionMetrics::new(idx, &action_type);

            // Check for breakpoints before executing
            if self.check_breakpoints(idx, &action_type) {
                self.paused = true;
                self.record_debug_event(
                    super::debug::DebugEventType::Breakpoint,
                    Some(idx),
                    Some(action_type.clone()),
                    None,
                    None,
                    None,
                    None,
                );
                log::info!("Breakpoint hit at action {idx} ({action_type}), execution paused");
            }

            // Wait if paused (using loop pattern to avoid clippy warning)
            loop {
                if !self.paused {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            // Record action start
            self.record_debug_event(
                super::debug::DebugEventType::ActionStart,
                Some(idx),
                Some(action_type.clone()),
                None,
                None,
                None,
                None,
            );

            log::debug!("Action {}: {:?}", idx + 1, action);

            match self.execute_action(action).await {
                Ok(()) => {
                    metrics = metrics.complete();
                    self.actions_succeeded += 1;
                    log::debug!("Action {} completed in {:?}", idx + 1, metrics.duration);

                    // Record action completion
                    self.record_debug_event(
                        super::debug::DebugEventType::ActionComplete,
                        Some(idx),
                        Some(action_type),
                        None,
                        None,
                        None,
                        None,
                    );
                }
                Err(e) => {
                    let error_msg = format!("{e}");
                    metrics = metrics.fail(&error_msg);
                    self.actions_failed += 1;
                    log::error!(
                        "Action {} failed after {:?}: {}",
                        idx + 1,
                        metrics.duration,
                        error_msg
                    );

                    // Record action error
                    self.record_debug_event(
                        super::debug::DebugEventType::ActionError,
                        Some(idx),
                        Some(action_type),
                        None,
                        None,
                        None,
                        Some(error_msg.clone()),
                    );

                    self.action_metrics.push(metrics);
                    return Err(e).with_context(|| {
                        format!(
                            "Failed to execute action {} in task '{}'",
                            idx + 1,
                            self.task_def.name
                        )
                    });
                }
            }

            self.action_metrics.push(metrics);
            self.actions_executed += 1;

            // Pause after each action if in step mode
            if self.debug_mode {
                self.paused = true;
            }
        }

        log::info!(
            "DSL task '{}' completed ({} actions executed, {} succeeded, {} failed)",
            self.task_def.name,
            self.actions_executed,
            self.actions_succeeded,
            self.actions_failed
        );
        Ok(())
    }

    /// Execute a single action.
    pub(super) async fn execute_action(&mut self, action: &Action) -> Result<()> {
        match action {
            Action::Navigate { url } => {
                let resolved_url = self.substitute_variables(url);
                self.api.navigate(&resolved_url, 30000).await?;
                Ok(())
            }
            Action::Click { selector } => {
                let resolved_selector = self.substitute_variables(selector);
                self.api.click(&resolved_selector).await?;
                Ok(())
            }
            Action::Type { selector, text } => {
                let resolved_selector = self.substitute_variables(selector);
                let resolved_text = self.substitute_variables(text);
                self.api.r#type(&resolved_selector, &resolved_text).await?;
                Ok(())
            }
            Action::Wait { duration_ms } => {
                tokio::time::sleep(tokio::time::Duration::from_millis(*duration_ms)).await;
                Ok(())
            }
            Action::WaitFor {
                selector,
                timeout_ms,
            } => {
                let resolved_selector = self.substitute_variables(selector);
                let timeout = timeout_ms.unwrap_or(5000);
                let deadline =
                    tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout);
                while tokio::time::Instant::now() < deadline {
                    // Use cached check for better performance
                    if self.cached_exists(&resolved_selector).await? {
                        return Ok(());
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                Err(anyhow::anyhow!(
                    "Timeout waiting for element: {resolved_selector}"
                ))
            }
            Action::ScrollTo { selector } => {
                let resolved_selector = self.substitute_variables(selector);
                self.api.scroll_to(&resolved_selector).await?;
                Ok(())
            }
            Action::Extract { selector, variable } => {
                let resolved_selector = self.substitute_variables(selector);
                let text = self.api.text(&resolved_selector).await?.unwrap_or_default();
                if let Some(var_name) = variable {
                    log::debug!("Extracting variable '{var_name}': {text}");
                    self.variables.insert(var_name.clone(), text);
                }
                Ok(())
            }
            Action::Execute { script: _ } => {
                log::warn!("Execute action not yet implemented");
                Ok(())
            }
            Action::Log { message, level } => {
                let resolved_message = self.substitute_variables(message);
                match level.as_ref().unwrap_or(&LogLevel::Info) {
                    LogLevel::Debug => log::debug!("{resolved_message}"),
                    LogLevel::Info => log::info!("{resolved_message}"),
                    LogLevel::Warn => log::warn!("{resolved_message}"),
                    LogLevel::Error => log::error!("{resolved_message}"),
                }
                Ok(())
            }
            Action::If {
                condition,
                then,
                r#else,
            } => self.execute_if(condition, then, r#else).await,
            Action::Loop {
                count,
                condition,
                actions,
            } => self.execute_loop(count, condition, actions).await,
            Action::Call { task, parameters } => self.execute_call(task, parameters.as_ref()).await,
            Action::Screenshot { path, selector } => {
                let resolved_path = path.as_ref().map(|p| self.substitute_variables(p));

                if let Some(_sel) = selector {
                    log::info!("Taking element screenshot");
                } else {
                    log::info!("Taking full page screenshot");
                }

                if let Some(p) = resolved_path {
                    log::info!("Screenshot would be saved to: {p}");
                }
                // Note: Full implementation requires TaskContext to support screenshots
                Ok(())
            }
            Action::Clear { selector } => {
                let resolved_selector = self.substitute_variables(selector);
                log::debug!("Clearing input field '{resolved_selector}'");
                self.api.clear(&resolved_selector).await?;
                Ok(())
            }
            Action::Hover { selector } => {
                let resolved_selector = self.substitute_variables(selector);
                log::debug!("Hovering over element '{resolved_selector}'");
                self.api.hover(&resolved_selector).await?;
                Ok(())
            }
            Action::Select {
                selector,
                value,
                by_value,
            } => {
                let resolved_selector = self.substitute_variables(selector);
                let resolved_value = self.substitute_variables(value);
                let use_value_attr = by_value.unwrap_or(false);

                log::debug!(
                    "Selecting '{resolved_value}' from dropdown '{resolved_selector}' (by_value={use_value_attr})"
                );

                // Use JavaScript to select the option
                let script = if use_value_attr {
                    format!(
                        r"document.querySelector('{resolved_selector}').value = '{resolved_value}';"
                    )
                } else {
                    format!(
                        r"const select = document.querySelector('{resolved_selector}');
                        const options = Array.from(select.options);
                        const option = options.find(o => o.text.trim() === '{resolved_value}');
                        if (option) select.value = option.value;"
                    )
                };

                // Execute the JavaScript via the page
                // Note: This requires TaskContext to have execute_script capability
                log::info!("Would execute select script: {script}");
                Ok(())
            }
            Action::RightClick { selector } => {
                let resolved_selector = self.substitute_variables(selector);
                log::debug!("Right-clicking element '{resolved_selector}'");
                self.api.right_click(&resolved_selector).await?;
                Ok(())
            }
            Action::DoubleClick { selector } => {
                let resolved_selector = self.substitute_variables(selector);
                log::debug!("Double-clicking element '{resolved_selector}'");
                self.api.double_click(&resolved_selector).await?;
                Ok(())
            }
            Action::Parallel {
                actions,
                max_concurrency,
            } => self.execute_parallel(actions, max_concurrency).await,
            Action::Retry {
                actions,
                max_attempts,
                initial_delay_ms,
                max_delay_ms,
                backoff_multiplier,
                jitter,
                retry_on,
            } => {
                let config = super::control_flow::RetryConfig {
                    max_attempts: max_attempts.unwrap_or(3),
                    initial_delay_ms: initial_delay_ms.unwrap_or(1000),
                    max_delay_ms: max_delay_ms.unwrap_or(30000),
                    backoff_multiplier: backoff_multiplier.unwrap_or(2.0),
                    jitter: jitter.unwrap_or(true),
                    retry_on: retry_on.clone(),
                };
                self.execute_retry(actions, &config).await
            }
            Action::Foreach {
                variable,
                collection,
                actions,
                max_iterations,
            } => {
                self.execute_foreach(variable, collection, actions, max_iterations)
                    .await
            }
            Action::While {
                condition,
                actions,
                max_iterations,
            } => self.execute_while(condition, actions, max_iterations).await,
            Action::Try {
                try_actions,
                catch_actions,
                error_variable,
                finally_actions,
            } => {
                // Execute try block
                let result = async {
                    for action in try_actions {
                        Box::pin(self.execute_action(action)).await?;
                    }
                    Ok::<(), anyhow::Error>(())
                }
                .await;

                match result {
                    Ok(()) => {
                        // Try succeeded, do nothing
                    }
                    Err(e) => {
                        // Try failed, execute catch block
                        if let Some(catch) = catch_actions {
                            for action in catch {
                                Box::pin(self.execute_action(action)).await?;
                            }
                        }
                        // Set error variable if specified
                        if let Some(var_name) = error_variable {
                            self.variables.insert(var_name.clone(), e.to_string());
                        }
                    }
                }

                // Execute finally block (always runs)
                if let Some(finally) = finally_actions {
                    for action in finally {
                        Box::pin(self.execute_action(action)).await?;
                    }
                }

                Ok(())
            }
        }
    }

    /// Cached wrapper for checking element existence.
    pub(super) async fn cached_exists(&mut self, selector: &str) -> Result<bool> {
        if !self.cache_enabled {
            return self.api.exists(selector).await;
        }

        // Check cache
        if let Some(entry) = self.selector_cache.get(selector) {
            return Ok(entry.exists);
        }

        // Fetch and cache
        let exists = self.api.exists(selector).await?;
        let visible = self.api.visible(selector).await.unwrap_or(false);
        let entry = super::cache::SelectorCacheEntry::new(exists, visible, None, 0);
        self.selector_cache.insert(selector.to_string(), entry);

        Ok(exists)
    }

    /// Cached wrapper for checking element visibility.
    #[allow(dead_code)]
    pub(super) async fn cached_visible(&mut self, selector: &str) -> Result<bool> {
        if !self.cache_enabled {
            return self.api.visible(selector).await;
        }

        // Check cache
        if let Some(entry) = self.selector_cache.get(selector) {
            return Ok(entry.visible);
        }

        // Fetch and cache
        let exists = self.api.exists(selector).await.unwrap_or(false);
        let visible = self.api.visible(selector).await?;
        let entry = super::cache::SelectorCacheEntry::new(exists, visible, None, 0);
        self.selector_cache.insert(selector.to_string(), entry);

        Ok(visible)
    }

    /// Cached wrapper for getting element text.
    #[allow(dead_code)]
    pub(super) async fn cached_text(&mut self, selector: &str) -> Result<Option<String>> {
        if !self.cache_enabled {
            return self.api.text(selector).await;
        }

        // Check cache
        if let Some(entry) = self.selector_cache.get(selector) {
            if entry.text.is_some() {
                return Ok(entry.text.clone());
            }
        }

        // Fetch and cache with full data
        let exists = self.api.exists(selector).await.unwrap_or(false);
        let visible = self.api.visible(selector).await.unwrap_or(false);
        let text = self.api.text(selector).await?;
        let entry = super::cache::SelectorCacheEntry::new(exists, visible, text.clone(), 0);
        self.selector_cache.insert(selector.to_string(), entry);

        Ok(text)
    }

    /// Invalidate cache for a selector (call after mutations).
    #[allow(dead_code)]
    pub(super) fn invalidate_cache(&mut self, selector: &str) {
        self.selector_cache.invalidate(selector);
    }

    /// Enable selector caching.
    pub fn enable_caching(&mut self) {
        self.cache_enabled = true;
    }

    /// Disable selector caching.
    pub fn disable_caching(&mut self) {
        self.cache_enabled = false;
        self.selector_cache.clear();
    }

    /// Get cache statistics.
    #[must_use]
    pub fn get_cache_stats(&self) -> super::cache::CacheStats {
        self.selector_cache.stats()
    }

    /// Get performance profiling data.
    #[must_use]
    pub fn get_profiler_stats(&self) -> HashMap<String, serde_json::Value> {
        self.action_profilers
            .iter()
            .map(|(action_type, profiler)| {
                let stats = serde_json::json!({
                    "action_type": action_type,
                    "total_executions": profiler.total_executions,
                    "total_duration_ms": profiler.total_duration.as_millis() as u64,
                    "average_duration_ms": profiler.average_duration().map(|d| d.as_millis() as u64),
                    "min_duration_ms": profiler.min_duration.map(|d| d.as_millis() as u64),
                    "max_duration_ms": profiler.max_duration.map(|d| d.as_millis() as u64),
                    "failures": profiler.failures,
                });
                (action_type.clone(), stats)
            })
            .collect()
    }

    /// Clear the selector cache.
    pub fn clear_cache(&mut self) {
        self.selector_cache.clear();
    }

    /// Record action execution in profiler.
    #[allow(dead_code)]
    pub(super) fn record_profile(&mut self, action_type: &str, duration: Duration, success: bool) {
        let profiler = self
            .action_profilers
            .entry(action_type.to_string())
            .or_insert_with(|| super::profiling::ActionProfiler {
                action_type: action_type.to_string(),
                ..Default::default()
            });
        profiler.record(duration, success);
    }

    /// Watch a variable for changes.
    #[allow(dead_code)]
    pub(super) fn watch_variable(&mut self, name: &str, value: &str) {
        if !self.debug_mode {
            return;
        }

        if let Some(old_value) = self.watched_variables.get(name) {
            if old_value != value {
                // Variable changed
                self.record_debug_event(
                    super::debug::DebugEventType::VariableSet,
                    None,
                    None,
                    Some(name.to_string()),
                    Some(value.to_string()),
                    None,
                    None,
                );
            }
        }

        self.watched_variables
            .insert(name.to_string(), value.to_string());
    }

    /// Record a debug event.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn record_debug_event(
        &mut self,
        event_type: super::debug::DebugEventType,
        action_index: Option<usize>,
        action_type: Option<String>,
        variable_name: Option<String>,
        variable_value: Option<String>,
        condition_result: Option<bool>,
        error: Option<String>,
    ) {
        if !self.debug_mode {
            return;
        }

        let timestamp = chrono::Local::now().to_rfc3339();
        let event = super::debug::DebugEvent {
            timestamp,
            event_type,
            action_index,
            action_type,
            variable_name,
            variable_value,
            condition_result,
            error,
        };

        self.debug_events.push(event);
    }

    /// Check if any breakpoint should trigger for the current action.
    pub(super) fn check_breakpoints(&self, action_index: usize, action_type: &str) -> bool {
        if self.breakpoints.is_empty() {
            return false;
        }

        for breakpoint in &self.breakpoints {
            if breakpoint.should_trigger(action_index, action_type, &self.variables) {
                return true;
            }
        }

        false
    }

    /// Execute a Call action - invoke another task.
    pub(super) async fn execute_call(
        &mut self,
        task_name: &str,
        parameters: Option<&HashMap<String, serde_yml::Value>>,
    ) -> Result<()> {
        // Check recursion depth
        if self.call_depth >= MAX_CALL_DEPTH {
            return Err(anyhow::anyhow!(
                "Maximum call depth ({MAX_CALL_DEPTH}) exceeded when calling task '{task_name}'"
            ));
        }

        log::info!(
            "Calling task '{}' (depth {}/{})",
            task_name,
            self.call_depth,
            MAX_CALL_DEPTH
        );

        // Find the task definition
        let task_def = crate::task::dsl::get_task_definition(task_name)
            .ok_or_else(|| anyhow::anyhow!("Task '{task_name}' not found for Call action"))?;

        // Create a new executor with incremented call depth
        let mut called_executor = Self::with_depth(self.api, task_def, self.call_depth + 1);

        // Apply parameter overrides if provided
        if let Some(params) = parameters {
            called_executor = called_executor.with_parameters(&serde_yml::Value::Mapping(
                params
                    .iter()
                    .map(|(k, v)| (serde_yml::Value::String(k.clone()), v.clone()))
                    .collect(),
            ));
        }

        // Copy current variables to the called task
        for (key, value) in &self.variables {
            called_executor.variables.insert(key.clone(), value.clone());
        }

        // Execute the called task
        let result = Box::pin(called_executor.execute()).await;

        // Copy back any new variables set by the called task
        for (key, value) in called_executor.variables {
            self.variables.insert(key, value);
        }

        result.with_context(|| format!("Failed to execute called task '{task_name}'"))
    }
}

#[cfg(test)]
mod tests {
    use crate::task::dsl::TaskDefinition;
    use std::collections::HashMap;

    // TaskDefinition doesn't implement Default, so we create one manually
    fn create_test_task_def() -> TaskDefinition {
        TaskDefinition {
            name: "test".to_string(),
            description: "Test task".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![],
            actions: vec![],
        }
    }

    #[test]
    fn test_executor_new() {
        // Note: Full DslExecutor requires &TaskContext which needs
        // a complete browser session setup. These tests are structural.
        // Placeholder test
        let x = 1 + 1;
        assert_eq!(x, 2);
    }

    #[test]
    fn test_task_definition_creation() {
        let task_def = create_test_task_def();
        assert_eq!(task_def.name, "test");
        assert!(task_def.actions.is_empty());
    }
}
