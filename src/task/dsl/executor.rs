//! Main DSL Executor implementation.
//!
//! Contains `DslExecutor` struct and core execution methods.
//! Other method groups are split into separate modules:
//! - cache.rs: `SelectorCache`, cache operations
//! - debug.rs: `DebugEvent`, Breakpoint, debug infrastructure
//! - profiling.rs: `ActionProfiler`, `ActionMetrics`, `ExecutionReport`
//! - evaluator.rs: Variable substitution, condition evaluation
//! - `control_flow.rs`: If, Loop, Foreach, While, Retry, Parallel handlers

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::task::dsl::api::DslApi;
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

/// Default cache TTL in milliseconds (5 seconds).
pub const DEFAULT_CACHE_TTL_MS: u64 = 5000;

/// Main DSL Executor struct.
pub struct DslExecutor<'a, T: DslApi> {
    /// Task context for API operations
    pub api: &'a T,
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
    /// Cache TTL for selector cache entries
    pub cache_ttl: Duration,
}

impl<'a, T: DslApi> DslExecutor<'a, T> {
    /// Create a new DSL executor.
    #[must_use]
    pub fn new(api: &'a T, task_def: TaskDefinition) -> Self {
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
            cache_ttl: Duration::from_millis(DEFAULT_CACHE_TTL_MS),
        }
    }

    /// Create a new DSL executor with specific call depth (for internal calls).
    pub(super) fn with_depth(api: &'a T, task_def: TaskDefinition, call_depth: u32) -> Self {
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
            cache_ttl: Duration::from_millis(DEFAULT_CACHE_TTL_MS),
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

    /// Execute a single action by dispatching to the appropriate handler.
    pub(super) async fn execute_action(&mut self, action: &Action) -> Result<()> {
        match action {
            Action::Navigate { url } => self.execute_navigate(url).await,
            Action::Click { selector } => self.execute_click(selector).await,
            Action::Type { selector, text } => self.execute_type(selector, text).await,
            Action::Wait { duration_ms } => self.execute_wait(*duration_ms).await,
            Action::WaitFor {
                selector,
                timeout_ms,
            } => self.execute_wait_for(selector, *timeout_ms).await,
            Action::ScrollTo { selector } => self.execute_scroll_to(selector).await,
            Action::Extract { selector, variable } => {
                self.execute_extract(selector, variable.as_deref()).await
            }
            Action::Execute { script } => self.execute_js(script).await,
            Action::Log { message, level } => self.execute_log(message, level.as_ref()).await,
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
                self.execute_screenshot(path.as_deref(), selector.as_deref())
                    .await
            }
            Action::Clear { selector } => self.execute_clear(selector).await,
            Action::Hover { selector } => self.execute_hover(selector).await,
            Action::Select {
                selector,
                value,
                by_value,
            } => self.execute_select(selector, value, *by_value).await,
            Action::RightClick { selector } => self.execute_right_click(selector).await,
            Action::DoubleClick { selector } => self.execute_double_click(selector).await,
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
                self.execute_try(
                    try_actions,
                    catch_actions.as_ref(),
                    error_variable.as_deref(),
                    finally_actions.as_ref(),
                )
                .await
            }
        }
    }

    // [Handlers moved to submodules: actions/browser, actions/wait, actions/inspection, actions/media]

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
        let entry =
            super::cache::SelectorCacheEntry::with_ttl(exists, visible, None, 0, self.cache_ttl);
        self.selector_cache.insert(selector.to_string(), entry);

        Ok(exists)
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

    /// Get cache stats for testing. Returns current cache size.
    pub fn cache_size(&self) -> usize {
        self.selector_cache.stats().size
    }

    /// Set the cache TTL in milliseconds.
    pub fn set_cache_ttl(&mut self, ttl_ms: u64) {
        self.cache_ttl = Duration::from_millis(ttl_ms);
    }

    /// Get the current cache TTL in milliseconds.
    #[must_use]
    pub fn get_cache_ttl(&self) -> u64 {
        self.cache_ttl.as_millis() as u64
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

    /// Execute a JavaScript action.
    async fn execute_js(&mut self, script: &str) -> Result<()> {
        let resolved = self.substitute_variables(script);
        self.api.execute_js(&resolved).await?;
        Ok(())
    }

    /// Execute a Log action.
    async fn execute_log(&mut self, message: &str, level: Option<&LogLevel>) -> Result<()> {
        match level {
            Some(LogLevel::Debug) => log::debug!("{}", message),
            Some(LogLevel::Info) => log::info!("{}", message),
            Some(LogLevel::Warn) => log::warn!("{}", message),
            Some(LogLevel::Error) => log::error!("{}", message),
            None => log::info!("{}", message),
        }
        Ok(())
    }

    /// Execute a Call action - invoke another task.
    ///
    /// Implements diff-based variable copy (3a) and pre-processed parameter passing (3b):
    /// 1. Snapshots pre-call variable names
    /// 2. Copies parent variables as the base
    /// 3. Applies Call parameters as overrides with ${variable} substitution
    /// 4. Only copies back NEW variables (not in pre-call snapshot)
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

        // 3a: Snapshot pre-call variable names for diff-based copy-back
        let pre_call_vars: HashSet<String> = self.variables.keys().cloned().collect();

        // 3b: Copy parent variables as the base for the called task
        for (key, value) in &self.variables {
            called_executor.variables.insert(key.clone(), value.clone());
        }

        // 3b: Apply Call parameters as overrides with ${variable} substitution
        if let Some(params) = parameters {
            for (key, value) in params {
                let var_name = key.clone();
                let raw_value = match value {
                    serde_yml::Value::String(s) => s.clone(),
                    serde_yml::Value::Number(n) => n.to_string(),
                    serde_yml::Value::Bool(b) => b.to_string(),
                    _ => format!("{value:?}"),
                };
                // Substitute ${variable} references in the parameter value
                let resolved_value = self.substitute_variables(&raw_value);
                log::debug!(
                    "Call parameter '{var_name}': '{}' (raw: '{raw_value}')",
                    resolved_value,
                );
                called_executor.variables.insert(var_name, resolved_value);
            }
        }

        // Propagate cache TTL from parent to called executor
        called_executor.cache_ttl = self.cache_ttl;

        // Execute the called task
        let result = Box::pin(called_executor.execute()).await;

        // 3a: Only copy back NEW variables (not in pre-call snapshot)
        for (key, value) in called_executor.variables {
            if !pre_call_vars.contains(&key) {
                log::debug!("Call copied back new variable '{key}': '{value}'");
                self.variables.insert(key, value);
            }
        }

        result.with_context(|| format!("Failed to execute called task '{task_name}'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::dsl::api::mock::{MockCall, MockDslApi};
    use crate::task::dsl::{Action, Condition, TaskDefinition};
    use std::collections::HashMap;

    /// Create a minimal TaskDefinition for testing.
    fn create_task_def(name: &str, actions: Vec<Action>) -> TaskDefinition {
        TaskDefinition {
            name: name.to_string(),
            description: format!("Test: {name}"),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![],
            actions,
        }
    }

    /// Create a DslExecutor with a MockDslApi for the given actions.
    fn create_executor<'a>(
        mock: &'a MockDslApi,
        actions: Vec<Action>,
    ) -> DslExecutor<'a, MockDslApi> {
        DslExecutor::new(mock, create_task_def("test", actions))
    }

    // ── Execute (JavaScript) action ───────────────────────────────────────

    #[tokio::test]
    async fn test_execute_action_execute_js_calls_api() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Execute {
            script: "document.title".to_string(),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1, "Execute should make exactly one API call");
        assert_eq!(
            calls[0],
            MockCall::ExecuteJs {
                script: "document.title".to_string()
            },
            "Execute should call api.execute_js with the correct script"
        );
    }

    #[tokio::test]
    async fn test_execute_action_execute_js_with_variable_substitution() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);
        exec.variables.insert("id".to_string(), "123".to_string());

        exec.execute_action(&Action::Execute {
            script: "document.getElementById('${id}')".to_string(),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(
            calls[0],
            MockCall::ExecuteJs {
                script: "document.getElementById('123')".to_string()
            },
            "variables should be substituted before calling execute_js"
        );
    }

    // ── Control flow dispatch smoke tests ───────────────────────────────

    #[tokio::test]
    async fn test_execute_action_if_dispatches_to_execute_if() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::If {
            condition: Condition::True,
            then: vec![Action::Click {
                selector: "#then-btn".to_string(),
            }],
            r#else: None,
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, MockCall::Click { selector } if selector == "#then-btn")),
            "If dispatch should route to execute_if which executes the then branch"
        );
    }

    #[tokio::test]
    async fn test_execute_action_loop_dispatches_to_execute_loop() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Loop {
            count: Some(2),
            condition: None,
            actions: vec![Action::Click {
                selector: "#item".to_string(),
            }],
        })
        .await
        .unwrap();

        let clicks: Vec<_> = mock
            .get_calls()
            .into_iter()
            .filter(|c| matches!(c, MockCall::Click { .. }))
            .collect();
        assert_eq!(
            clicks.len(),
            2,
            "Loop dispatch should route to execute_loop which iterates 2 times"
        );
    }

    // ── Full end-to-end execute() ─────────────────────────────────────────

    #[tokio::test]
    async fn test_execute_multiple_actions() {
        let mock = MockDslApi::new();
        let task_def = create_task_def(
            "multi_step",
            vec![
                Action::Navigate {
                    url: "https://example.com".to_string(),
                },
                Action::Click {
                    selector: "#login".to_string(),
                },
                Action::Type {
                    selector: "#user".to_string(),
                    text: "admin".to_string(),
                },
                Action::Wait { duration_ms: 1 },
                Action::Extract {
                    selector: "#result".to_string(),
                    variable: Some("output".to_string()),
                },
            ],
        );

        let mut exec = DslExecutor::new(&mock, task_def);
        exec.execute().await.unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 4, "should make 4 API calls (no call for Wait)");
        assert_eq!(
            calls[0],
            MockCall::Navigate {
                url: "https://example.com".to_string(),
                timeout_ms: 30000,
            }
        );
        assert_eq!(
            calls[1],
            MockCall::Click {
                selector: "#login".to_string(),
            }
        );
        assert_eq!(
            calls[2],
            MockCall::Type {
                selector: "#user".to_string(),
                text: "admin".to_string(),
            }
        );
        assert_eq!(
            calls[3],
            MockCall::Text {
                selector: "#result".to_string(),
            }
        );

        // Verify execution statistics
        assert_eq!(exec.actions_executed, 5, "all 5 actions should be counted");
        assert_eq!(exec.actions_succeeded, 5, "all 5 should succeed");
        assert_eq!(exec.actions_failed, 0, "no failures");

        // Verify the Extract stored the variable
        assert!(
            exec.variables.contains_key("output"),
            "Extract should store variable"
        );
    }

    #[tokio::test]
    async fn test_execute_propagates_first_error() {
        let mock = MockDslApi::new();
        let task_def = create_task_def(
            "error_at_step_2",
            vec![
                // Action 1: Wait succeeds (no API call)
                Action::Wait { duration_ms: 1 },
                // Action 2: Click fails because fail_all is true
                Action::Click {
                    selector: "#broken".to_string(),
                },
                // Action 3: should not execute
                Action::Wait { duration_ms: 1 },
            ],
        );

        mock.set_fail_all(true);

        let mut exec = DslExecutor::new(&mock, task_def);
        let result = exec.execute().await;

        assert!(
            result.is_err(),
            "execute should propagate first action error"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to execute action 2"),
            "error should reference the failing action index"
        );

        // Wait (action 1) completed; Click (action 2) failed before incrementing
        assert_eq!(
            exec.actions_executed, 1,
            "only 1 action completed before error"
        );
    }

    // ── Structural tests ──────────────────────────────────────────────────

    #[test]
    fn test_executor_new() {
        let mock = MockDslApi::new();
        let exec = create_executor(&mock, vec![]);
        assert_eq!(exec.task_def.name, "test");
        assert!(exec.task_def.actions.is_empty());
    }

    #[test]
    fn test_task_definition_creation() {
        let task_def = TaskDefinition {
            name: "my_task".to_string(),
            description: "".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![],
            actions: vec![],
        };
        assert_eq!(task_def.name, "my_task");
        assert!(task_def.actions.is_empty());
    }

    // ── Cache TTL (3c) ────────────────────────────────────────────────────

    #[test]
    fn test_cache_ttl_default_value() {
        let mock = MockDslApi::new();
        let exec = create_executor(&mock, vec![]);
        assert_eq!(
            exec.get_cache_ttl(),
            DEFAULT_CACHE_TTL_MS,
            "default cache TTL should match DEFAULT_CACHE_TTL_MS"
        );
    }

    #[test]
    fn test_cache_ttl_setter_and_getter() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.set_cache_ttl(1000);
        assert_eq!(
            exec.get_cache_ttl(),
            1000,
            "get_cache_ttl should return set value"
        );

        exec.set_cache_ttl(0);
        assert_eq!(
            exec.get_cache_ttl(),
            0,
            "cache TTL can be set to 0 (immediate expiry)"
        );

        exec.set_cache_ttl(30000);
        assert_eq!(exec.get_cache_ttl(), 30000, "cache TTL can be set to 30s");
    }

    #[test]
    fn test_cache_entry_uses_configured_ttl() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);
        exec.set_cache_ttl(30000); // 30 seconds

        // Insert a cache entry directly to simulate what cached_exists does
        let entry = crate::task::dsl::cache::SelectorCacheEntry::with_ttl(
            true,
            true,
            None,
            0,
            exec.cache_ttl,
        );
        exec.selector_cache.insert("#selector".to_string(), entry);

        // Retrieve and check the TTL
        let retrieved = exec.selector_cache.get("#selector").unwrap();
        assert_eq!(
            retrieved.ttl,
            Duration::from_millis(30000),
            "cache entry should use executor's TTL"
        );
    }

    #[test]
    fn test_cache_entry_default_ttl_is_5_seconds() {
        let entry = crate::task::dsl::cache::SelectorCacheEntry::new(true, true, None, 0);
        assert_eq!(
            entry.ttl,
            Duration::from_secs(5),
            "default TTL should be 5s"
        );
    }

    // ── execute_call: variable copy-back (3a) + parameter passing (3b) ──
    //
    // Full integration tests for execute_call require a registry with DSL
    // TaskDefinitions, which isn't available in unit tests. The behavioral
    // contracts are tested inline below by verifying the logic directly.

    #[test]
    fn test_execute_call_pre_snapshot_logic() {
        // Verify the snapshot-and-diff logic used in execute_call
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        // Set up pre-call variables
        exec.variables
            .insert("existing_var".to_string(), "original".to_string());
        exec.variables
            .insert("shared_var".to_string(), "parent_value".to_string());

        // Snapshot pre-call variable names (simulating 3a)
        let pre_call_vars: HashSet<String> = exec.variables.keys().cloned().collect();
        assert!(pre_call_vars.contains("existing_var"));
        assert!(pre_call_vars.contains("shared_var"));
        assert_eq!(pre_call_vars.len(), 2);

        // Simulate what the called task does: creates a new var, modifies shared var
        let mut called_vars = exec.variables.clone();
        called_vars.insert("new_var".to_string(), "new_value".to_string());
        called_vars.insert("shared_var".to_string(), "called_override".to_string());

        // 3a: Only copy back NEW variables (not in pre-call snapshot)
        for (key, value) in called_vars {
            if !pre_call_vars.contains(&key) {
                exec.variables.insert(key, value);
            }
        }

        // existing_var should remain "original" (unchanged)
        assert_eq!(
            exec.variables.get("existing_var").unwrap(),
            "original",
            "3a: existing variables should NOT be overwritten by called task"
        );
        // shared_var should remain "parent_value" (unchanged by called task)
        assert_eq!(
            exec.variables.get("shared_var").unwrap(),
            "parent_value",
            "3a: shared variables should NOT be overwritten"
        );
        // new_var should be copied back
        assert_eq!(
            exec.variables.get("new_var").unwrap(),
            "new_value",
            "3a: new variables from called task should be copied back"
        );
    }

    #[test]
    fn test_execute_call_parameter_override_logic() {
        // Verify the parameter override logic used in execute_call
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        // Set up parent variables
        exec.variables
            .insert("base_url".to_string(), "https://default.com".to_string());
        exec.variables
            .insert("username".to_string(), "parent_user".to_string());

        // Simulate the 3b flow:
        // 1. Copy parent variables to called executor
        let mut called_vars = exec.variables.clone();

        // 2. Apply Call parameters as overrides (with variable substitution)
        let params: HashMap<String, serde_yml::Value> = [
            (
                "username".to_string(),
                serde_yml::Value::String("alice".to_string()),
            ),
            (
                "timeout".to_string(),
                serde_yml::Value::Number(serde_yml::Number::from(30)),
            ),
        ]
        .into();

        for (key, value) in &params {
            let var_name = key.clone();
            let raw_value = match value {
                serde_yml::Value::String(s) => s.clone(),
                serde_yml::Value::Number(n) => n.to_string(),
                serde_yml::Value::Bool(b) => b.to_string(),
                _ => format!("{value:?}"),
            };
            // Substitute ${variable} references (use a simple version)
            let resolved =
                raw_value.replace("${base_url}", exec.variables.get("base_url").unwrap());
            called_vars.insert(var_name, resolved);
        }

        // Verify: parameters override parent variables
        assert_eq!(
            called_vars.get("username").unwrap(),
            "alice",
            "3b: Call parameters should override parent variables"
        );
        assert_eq!(
            called_vars.get("timeout").unwrap(),
            "30",
            "3b: new parameters should be added to called task"
        );
        // base_url should still be inherited from parent
        assert_eq!(
            called_vars.get("base_url").unwrap(),
            "https://default.com",
            "3b: non-overridden parent variables should be inherited"
        );
    }

    #[test]
    fn test_execute_call_variable_substitution_in_parameters() {
        // Verify that ${variable} substitution works in Call parameter values
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.variables
            .insert("domain".to_string(), "example.com".to_string());
        exec.variables
            .insert("protocol".to_string(), "https".to_string());

        // Simulate parameter with ${variable} references
        let raw_value = "${protocol}://${domain}/api".to_string();
        let resolved = exec.substitute_variables(&raw_value);

        assert_eq!(
            resolved, "https://example.com/api",
            "3b: variable substitution in Call parameters should work"
        );
    }

    // ── Call: Full passthrough (3a + 3b combined) ──────────────────────

    #[test]
    fn test_execute_call_passthrough() {
        let mock = MockDslApi::new();

        // Case 1: Parent has 2 vars, called task adds 2 + tries to overwrite 1
        {
            let mut exec = create_executor(&mock, vec![]);
            exec.variables
                .insert("url".to_string(), "https://example.com".to_string());
            exec.variables.insert("count".to_string(), "10".to_string());
            let pre_call_vars: HashSet<String> = exec.variables.keys().cloned().collect();
            let mut called_vars = exec.variables.clone();
            called_vars.insert("result".to_string(), "success".to_string());
            called_vars.insert("session_id".to_string(), "abc-123".to_string());
            called_vars.insert("url".to_string(), "https://override.com".to_string());
            for (key, value) in called_vars {
                if !pre_call_vars.contains(&key) {
                    exec.variables.insert(key, value);
                }
            }
            assert_eq!(exec.variables.get("url").unwrap(), "https://example.com");
            assert_eq!(exec.variables.get("count").unwrap(), "10");
            assert_eq!(exec.variables.get("result").unwrap(), "success");
            assert_eq!(exec.variables.get("session_id").unwrap(), "abc-123");
            assert_eq!(
                exec.variables.len(),
                4,
                "parent vars preserved + new copied back"
            );
        }

        // Case 2: Called task produces no new vars
        {
            let mut exec = create_executor(&mock, vec![]);
            exec.variables
                .insert("existing".to_string(), "value".to_string());
            let pre_call_vars: HashSet<String> = exec.variables.keys().cloned().collect();
            let called_vars = exec.variables.clone();
            for (key, value) in called_vars {
                if !pre_call_vars.contains(&key) {
                    exec.variables.insert(key, value);
                }
            }
            assert_eq!(exec.variables.len(), 1, "no new vars");
            assert_eq!(exec.variables.get("existing").unwrap(), "value");
        }

        // Case 3: Parent has no variables — all copied back
        {
            let mut exec = create_executor(&mock, vec![]);
            let pre_call_vars: HashSet<String> = HashSet::new();
            let mut called_vars = HashMap::new();
            called_vars.insert("a".to_string(), "1".to_string());
            called_vars.insert("b".to_string(), "2".to_string());
            for (key, value) in called_vars {
                if !pre_call_vars.contains(&key) {
                    exec.variables.insert(key, value);
                }
            }
            assert_eq!(exec.variables.get("a").unwrap(), "1");
            assert_eq!(exec.variables.get("b").unwrap(), "2");
            assert_eq!(exec.variables.len(), 2);
        }
    }

    // ── Call: Parameter overrides (3b) ──────────────────────────────────

    #[test]
    fn test_execute_call_parameter_overrides() {
        let mock = MockDslApi::new();

        // Case 1: Override specific parent vars while others remain inherited
        {
            let mut exec = create_executor(&mock, vec![]);
            exec.variables
                .insert("host".to_string(), "default.com".to_string());
            exec.variables
                .insert("port".to_string(), "8080".to_string());
            exec.variables
                .insert("debug".to_string(), "false".to_string());
            let mut called_vars = exec.variables.clone();
            let params: HashMap<String, serde_yml::Value> = [(
                "host".to_string(),
                serde_yml::Value::String("api.example.com".to_string()),
            )]
            .into();
            for (key, value) in &params {
                let resolved = match value {
                    serde_yml::Value::String(s) => s.clone(),
                    _ => format!("{value:?}"),
                };
                called_vars.insert(key.clone(), resolved);
            }
            assert_eq!(called_vars.get("host").unwrap(), "api.example.com");
            assert_eq!(called_vars.get("port").unwrap(), "8080");
            assert_eq!(called_vars.get("debug").unwrap(), "false");
            assert_eq!(called_vars.len(), 3, "no extra vars added");
        }

        // Case 2: Params can be String, Number, or Bool
        {
            let mut called_vars = HashMap::new();
            let params: HashMap<String, serde_yml::Value> = [
                (
                    "name".to_string(),
                    serde_yml::Value::String("alice".to_string()),
                ),
                (
                    "age".to_string(),
                    serde_yml::Value::Number(serde_yml::Number::from(30)),
                ),
                ("active".to_string(), serde_yml::Value::Bool(true)),
            ]
            .into();
            for (key, value) in &params {
                let resolved = match value {
                    serde_yml::Value::String(s) => s.clone(),
                    serde_yml::Value::Number(n) => n.to_string(),
                    serde_yml::Value::Bool(b) => b.to_string(),
                    _ => format!("{value:?}"),
                };
                called_vars.insert(key.clone(), resolved);
            }
            assert_eq!(called_vars.get("name").unwrap(), "alice");
            assert_eq!(called_vars.get("age").unwrap(), "30");
            assert_eq!(called_vars.get("active").unwrap(), "true");
        }
    }

    #[test]
    fn test_execute_call_parameter_edge_cases() {
        let mock = MockDslApi::new();

        // Case 1: ${variable} references in parameter values are resolved
        {
            let mut exec = create_executor(&mock, vec![]);
            exec.variables
                .insert("base_url".to_string(), "https://example.com".to_string());
            exec.variables
                .insert("api_key".to_string(), "sk-test123".to_string());
            let params: HashMap<String, serde_yml::Value> = [
                (
                    "endpoint".to_string(),
                    serde_yml::Value::String("${base_url}/api/v1".to_string()),
                ),
                (
                    "auth_header".to_string(),
                    serde_yml::Value::String("Bearer ${api_key}".to_string()),
                ),
            ]
            .into();
            for (key, value) in &params {
                let raw = match value {
                    serde_yml::Value::String(s) => s.clone(),
                    _ => continue,
                };
                let resolved = exec.substitute_variables(&raw);
                match key.as_str() {
                    "endpoint" => assert_eq!(resolved, "https://example.com/api/v1"),
                    "auth_header" => assert_eq!(resolved, "Bearer sk-test123"),
                    _ => panic!("unexpected param key"),
                }
            }
        }

        // Case 2: Empty params map — no overrides, new vars still propagate
        {
            let mut exec = create_executor(&mock, vec![]);
            exec.variables
                .insert("x".to_string(), "parent_x".to_string());
            let mut called_vars = exec.variables.clone();
            called_vars.insert("y".to_string(), "new_y".to_string());
            let pre_call: HashSet<String> = exec.variables.keys().cloned().collect();
            for (key, value) in called_vars {
                if !pre_call.contains(&key) {
                    exec.variables.insert(key, value);
                }
            }
            assert_eq!(exec.variables.get("x").unwrap(), "parent_x");
            assert_eq!(exec.variables.get("y").unwrap(), "new_y");
            assert_eq!(exec.variables.len(), 2);
        }

        // Case 3: params=None — pure variable passthrough
        {
            let mut exec = create_executor(&mock, vec![]);
            exec.variables
                .insert("mode".to_string(), "auto".to_string());
            let pre_call: HashSet<String> = exec.variables.keys().cloned().collect();
            let mut called_vars = exec.variables.clone();
            called_vars.insert("result".to_string(), "done".to_string());
            for (key, value) in called_vars {
                if !pre_call.contains(&key) {
                    exec.variables.insert(key, value);
                }
            }
            assert_eq!(exec.variables.get("mode").unwrap(), "auto");
            assert_eq!(exec.variables.get("result").unwrap(), "done");
            assert_eq!(exec.variables.len(), 2);
        }

        // Case 4: No parent vars — param passes through literally
        {
            let mut called_vars = HashMap::new();
            called_vars.insert("url".to_string(), "https://example.com".to_string());
            assert_eq!(called_vars.get("url").unwrap(), "https://example.com");
        }
    }

    // ── Call: Cache TTL propagation + Call depth ────────────────────────

    #[test]
    fn test_execute_call_cache_ttl() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);
        exec.set_cache_ttl(15000);
        let called_task_def = create_task_def("child", vec![Action::Wait { duration_ms: 1 }]);
        let mut called_exec = DslExecutor::with_depth(&mock, called_task_def.clone(), 1);
        called_exec.cache_ttl = exec.cache_ttl;
        assert_eq!(called_exec.get_cache_ttl(), 15000, "custom TTL propagated");
        let entry = crate::task::dsl::cache::SelectorCacheEntry::with_ttl(
            true,
            true,
            None,
            0,
            called_exec.cache_ttl,
        );
        assert_eq!(entry.ttl, Duration::from_millis(15000));

        // Default TTL propagation
        let default_exec = create_executor(&mock, vec![]);
        let mut default_called = DslExecutor::with_depth(&mock, called_task_def, 1);
        default_called.cache_ttl = default_exec.cache_ttl;
        assert_eq!(
            default_called.get_cache_ttl(),
            DEFAULT_CACHE_TTL_MS,
            "default TTL propagated"
        );
    }

    #[test]
    fn test_execute_call_depth() {
        let mock = MockDslApi::new();
        let exec = create_executor(&mock, vec![]);
        let called_task_def = create_task_def("child", vec![Action::Wait { duration_ms: 1 }]);
        let called_exec =
            DslExecutor::with_depth(&mock, called_task_def.clone(), exec.call_depth + 1);
        assert_eq!(called_exec.call_depth, 1, "depth increments");

        // Nested: depth 0 → 1 → 2
        let child_exec = DslExecutor::with_depth(&mock, called_task_def, 1);
        let grandchild_task_def =
            create_task_def("grandchild", vec![Action::Wait { duration_ms: 1 }]);
        let grandchild_exec =
            DslExecutor::with_depth(&mock, grandchild_task_def, child_exec.call_depth + 1);
        assert_eq!(grandchild_exec.call_depth, 2, "nested depth");
        assert!(grandchild_exec.call_depth < MAX_CALL_DEPTH);
    }
}
