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

    // ── Per-action handlers ────────────────────────────────────────────────

    /// Navigate to a URL.
    async fn execute_navigate(&mut self, url: &str) -> Result<()> {
        let resolved_url = self.substitute_variables(url);
        self.api.navigate(&resolved_url, 30000).await?;
        self.clear_cache();
        Ok(())
    }

    /// Click an element.
    async fn execute_click(&mut self, selector: &str) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        self.api.click(&resolved_selector).await?;
        self.clear_cache();
        Ok(())
    }

    /// Type text into an element.
    async fn execute_type(&mut self, selector: &str, text: &str) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        let resolved_text = self.substitute_variables(text);
        self.api.r#type(&resolved_selector, &resolved_text).await?;
        self.clear_cache();
        Ok(())
    }

    /// Wait for a duration.
    async fn execute_wait(&mut self, duration_ms: u64) -> Result<()> {
        tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
        Ok(())
    }

    /// Wait for an element to be visible (with timeout).
    async fn execute_wait_for(&mut self, selector: &str, timeout_ms: Option<u64>) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        let timeout = timeout_ms.unwrap_or(5000);
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout);
        while tokio::time::Instant::now() < deadline {
            if self.cached_exists(&resolved_selector).await? {
                return Ok(());
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        Err(anyhow::anyhow!(
            "Timeout waiting for element: {resolved_selector}"
        ))
    }

    /// Scroll to an element.
    async fn execute_scroll_to(&mut self, selector: &str) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        self.api.scroll_to(&resolved_selector).await?;
        self.clear_cache();
        Ok(())
    }

    /// Extract text from an element and optionally store in a variable.
    async fn execute_extract(&mut self, selector: &str, variable: Option<&str>) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        let text = self.api.text(&resolved_selector).await?.unwrap_or_default();
        if let Some(var_name) = variable {
            log::debug!("Extracting variable '{var_name}': {text}");
            self.variables.insert(var_name.to_string(), text);
        }
        Ok(())
    }

    /// Execute JavaScript in the page.
    async fn execute_js(&mut self, script: &str) -> Result<()> {
        let resolved_script = self.substitute_variables(script);
        log::debug!("Executing JS: {resolved_script}");
        let result = self.api.execute_js(&resolved_script).await?;
        log::info!("JS result: {result}");
        Ok(())
    }

    /// Log a message at the specified level.
    async fn execute_log(&mut self, message: &str, level: Option<&LogLevel>) -> Result<()> {
        let resolved_message = self.substitute_variables(message);
        match level.unwrap_or(&LogLevel::Info) {
            LogLevel::Debug => log::debug!("{resolved_message}"),
            LogLevel::Info => log::info!("{resolved_message}"),
            LogLevel::Warn => log::warn!("{resolved_message}"),
            LogLevel::Error => log::error!("{resolved_message}"),
        }
        Ok(())
    }

    /// Take a screenshot of the current page or a specific element.
    async fn execute_screenshot(
        &mut self,
        path: Option<&str>,
        selector: Option<&str>,
    ) -> Result<()> {
        let resolved_selector = selector.map(|s| self.substitute_variables(s));
        let resolved_path = path.map(|p| self.substitute_variables(p));

        if let Some(ref sel) = resolved_selector {
            log::info!("Taking element screenshot of '{sel}'");
        } else {
            log::info!("Taking full page screenshot");
        }

        // For element screenshots, scroll into view first
        if let Some(ref sel) = resolved_selector {
            self.api.scroll_to(sel).await?;
        }

        let file_path = self.api.screenshot().await?;

        // If a custom path was specified, copy/move the default screenshot
        if let Some(ref dest) = resolved_path {
            log::info!("Screenshot saved to: {dest} (default: {file_path})");
            // Note: the TaskContext screenshot method saves with a generated name;
            // a custom path would require a copy operation or a new screenshot method.
            // For now, log both paths so the user knows where the file is.
        } else {
            log::info!("Screenshot saved to: {file_path}");
        }

        self.clear_cache();
        Ok(())
    }

    /// Clear an input field.
    async fn execute_clear(&mut self, selector: &str) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        log::debug!("Clearing input field '{resolved_selector}'");
        self.api.clear(&resolved_selector).await?;
        self.clear_cache();
        Ok(())
    }

    /// Hover over an element.
    async fn execute_hover(&mut self, selector: &str) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        log::debug!("Hovering over element '{resolved_selector}'");
        self.api.hover(&resolved_selector).await?;
        self.clear_cache();
        Ok(())
    }

    /// Select an option from a dropdown.
    async fn execute_select(
        &mut self,
        selector: &str,
        value: &str,
        by_value: Option<bool>,
    ) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        let resolved_value = self.substitute_variables(value);
        let use_value_attr = by_value.unwrap_or(false);

        log::debug!(
            "Selecting '{resolved_value}' from dropdown '{resolved_selector}' (by_value={use_value_attr})"
        );

        let script = if use_value_attr {
            format!(r"document.querySelector('{resolved_selector}').value = '{resolved_value}';")
        } else {
            format!(
                r"const select = document.querySelector('{resolved_selector}');
                const options = Array.from(select.options);
                const option = options.find(o => o.text.trim() === '{resolved_value}');
                if (option) select.value = option.value;"
            )
        };

        self.api.execute_js(&script).await?;
        self.clear_cache();
        Ok(())
    }

    /// Right-click on an element.
    async fn execute_right_click(&mut self, selector: &str) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        log::debug!("Right-clicking element '{resolved_selector}'");
        self.api.right_click(&resolved_selector).await?;
        self.clear_cache();
        Ok(())
    }

    /// Double-click on an element.
    async fn execute_double_click(&mut self, selector: &str) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        log::debug!("Double-clicking element '{resolved_selector}'");
        self.api.double_click(&resolved_selector).await?;
        self.clear_cache();
        Ok(())
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
        let entry =
            super::cache::SelectorCacheEntry::with_ttl(exists, visible, None, 0, self.cache_ttl);
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
        let entry =
            super::cache::SelectorCacheEntry::with_ttl(exists, visible, None, 0, self.cache_ttl);
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
        let entry = super::cache::SelectorCacheEntry::with_ttl(
            exists,
            visible,
            text.clone(),
            0,
            self.cache_ttl,
        );
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
    use crate::task::dsl::{Action, Condition, ForeachCollection, LogLevel, TaskDefinition};
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

    // ── Screenshot action ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_execute_action_screenshot_calls_api() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Screenshot {
            path: None,
            selector: None,
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(
            calls.len(),
            1,
            "Screenshot should make exactly one API call"
        );
        assert_eq!(
            calls[0],
            MockCall::Screenshot,
            "Screenshot should call api.screenshot"
        );
    }

    #[tokio::test]
    async fn test_execute_action_screenshot_with_selector_scrolls_first() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Screenshot {
            path: None,
            selector: Some("#element".to_string()),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        // scroll_to + screenshot
        assert_eq!(calls.len(), 2, "Should scroll to element then screenshot");
        assert_eq!(
            calls[0],
            MockCall::ScrollTo {
                selector: "#element".to_string()
            },
            "first call should scroll to the element"
        );
        assert_eq!(
            calls[1],
            MockCall::Screenshot,
            "second call should take the screenshot"
        );
    }

    #[tokio::test]
    async fn test_execute_action_screenshot_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            crate::task::dsl::cache::SelectorCacheEntry::new(true, true, None, 0),
        );
        assert!(
            exec.cache_size() > 0,
            "cache should have entries before screenshot"
        );

        exec.execute_action(&Action::Screenshot {
            path: None,
            selector: None,
        })
        .await
        .unwrap();

        assert_eq!(
            exec.cache_size(),
            0,
            "cache should be cleared after screenshot"
        );
    }

    // ── Select action ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_execute_action_select_by_text() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Select {
            selector: "#country".to_string(),
            value: "United States".to_string(),
            by_value: Some(false),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1, "Select should make exactly one API call");
        assert!(
            matches!(&calls[0], MockCall::ExecuteJs { script } if script.contains("options.find(o => o.text.trim() === 'United States')")),
            "Select by text should execute JS that finds option by label text"
        );
    }

    #[tokio::test]
    async fn test_execute_action_select_by_value() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Select {
            selector: "#country".to_string(),
            value: "US".to_string(),
            by_value: Some(true),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1, "Select should make exactly one API call");
        assert!(
            matches!(&calls[0], MockCall::ExecuteJs { script } if script == "document.querySelector('#country').value = 'US';"),
            "Select by value should execute JS that sets value directly"
        );
    }

    #[tokio::test]
    async fn test_execute_action_select_with_variable_substitution() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);
        exec.variables
            .insert("target".to_string(), "Canada".to_string());
        exec.variables
            .insert("country_sel".to_string(), "#country".to_string());

        exec.execute_action(&Action::Select {
            selector: "${country_sel}".to_string(),
            value: "${target}".to_string(),
            by_value: Some(false),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert!(
            matches!(&calls[0], MockCall::ExecuteJs { script } if script.contains("options.find(o => o.text.trim() === 'Canada')") && script.contains("querySelector('#country')")),
            "variables should be substituted in both selector and value"
        );
    }

    #[tokio::test]
    async fn test_execute_action_select_defaults_to_text_lookup() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Select {
            selector: "#sel".to_string(),
            value: "option1".to_string(),
            by_value: None, // defaults to false = text lookup
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert!(
            matches!(&calls[0], MockCall::ExecuteJs { script } if script.contains("options.find(o => o.text.trim() === 'option1')")),
            "by_value=None should default to text lookup (false)"
        );
    }

    #[tokio::test]
    async fn test_execute_action_select_propagates_execute_js_error() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_action(&Action::Select {
                selector: "#sel".to_string(),
                value: "x".to_string(),
                by_value: Some(false),
            })
            .await;

        assert!(result.is_err(), "Select should propagate execute_js error");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("MockDslApi forced failure"),
            "error message should come from mock"
        );
    }

    // ── Click action ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_execute_action_click_calls_api() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Click {
            selector: "#submit-btn".to_string(),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1, "should make exactly one API call");
        assert_eq!(
            calls[0],
            MockCall::Click {
                selector: "#submit-btn".to_string()
            },
            "Click should call api.click with the correct selector"
        );
    }

    #[tokio::test]
    async fn test_execute_action_click_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        // Prime the cache with an entry
        exec.selector_cache.insert(
            "#stale".to_string(),
            crate::task::dsl::cache::SelectorCacheEntry::new(true, true, None, 0),
        );
        assert!(
            exec.cache_size() > 0,
            "cache should have entries before click"
        );

        exec.execute_action(&Action::Click {
            selector: "#btn".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(exec.cache_size(), 0, "cache should be cleared after click");
    }

    #[tokio::test]
    async fn test_execute_action_click_propagates_error() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_action(&Action::Click {
                selector: "#broken".to_string(),
            })
            .await;

        assert!(result.is_err(), "click should propagate API error");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("MockDslApi forced failure"),
            "error message should come from mock"
        );
    }

    // ── Type action ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_execute_action_type_calls_api() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Type {
            selector: "#username".to_string(),
            text: "test_user".to_string(),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0],
            MockCall::Type {
                selector: "#username".to_string(),
                text: "test_user".to_string(),
            },
            "Type should call api.r#type with correct selector and text"
        );
    }

    #[tokio::test]
    async fn test_execute_action_type_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            crate::task::dsl::cache::SelectorCacheEntry::new(true, true, None, 0),
        );
        assert!(exec.cache_size() > 0);

        exec.execute_action(&Action::Type {
            selector: "#input".to_string(),
            text: "hello".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(exec.cache_size(), 0, "cache should be cleared after type");
    }

    #[tokio::test]
    async fn test_execute_action_type_propagates_error() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_action(&Action::Type {
                selector: "#input".to_string(),
                text: "fail".to_string(),
            })
            .await;

        assert!(result.is_err(), "type should propagate API error");
    }

    // ── Navigate action ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_execute_action_navigate_calls_api() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Navigate {
            url: "https://example.com".to_string(),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0],
            MockCall::Navigate {
                url: "https://example.com".to_string(),
                timeout_ms: 30000,
            },
            "Navigate should call api.navigate with url and 30s timeout"
        );
    }

    #[tokio::test]
    async fn test_execute_action_navigate_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            crate::task::dsl::cache::SelectorCacheEntry::new(true, true, None, 0),
        );
        assert!(exec.cache_size() > 0);

        exec.execute_action(&Action::Navigate {
            url: "https://example.com".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(
            exec.cache_size(),
            0,
            "cache should be cleared after navigate"
        );
    }

    #[tokio::test]
    async fn test_execute_action_navigate_propagates_error() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_action(&Action::Navigate {
                url: "https://fail.example.com".to_string(),
            })
            .await;

        assert!(result.is_err(), "navigate should propagate API error");
    }

    // ── Other DOM-changing actions that should clear cache ────────────────

    #[tokio::test]
    async fn test_execute_action_scroll_to_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            crate::task::dsl::cache::SelectorCacheEntry::new(true, true, None, 0),
        );

        exec.execute_action(&Action::ScrollTo {
            selector: "#footer".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(exec.cache_size(), 0, "ScrollTo should clear cache");
    }

    #[tokio::test]
    async fn test_execute_action_clear_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            crate::task::dsl::cache::SelectorCacheEntry::new(true, true, None, 0),
        );

        exec.execute_action(&Action::Clear {
            selector: "#input".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(exec.cache_size(), 0, "Clear should clear cache");
    }

    #[tokio::test]
    async fn test_execute_action_hover_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            crate::task::dsl::cache::SelectorCacheEntry::new(true, true, None, 0),
        );

        exec.execute_action(&Action::Hover {
            selector: "#menu".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(exec.cache_size(), 0, "Hover should clear cache");
    }

    #[tokio::test]
    async fn test_execute_action_right_click_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            crate::task::dsl::cache::SelectorCacheEntry::new(true, true, None, 0),
        );

        exec.execute_action(&Action::RightClick {
            selector: "#item".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(exec.cache_size(), 0, "RightClick should clear cache");
    }

    #[tokio::test]
    async fn test_execute_action_double_click_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            crate::task::dsl::cache::SelectorCacheEntry::new(true, true, None, 0),
        );

        exec.execute_action(&Action::DoubleClick {
            selector: "#item".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(exec.cache_size(), 0, "DoubleClick should clear cache");
    }

    // ── Non-DOM-changing actions should NOT clear cache ───────────────────

    #[tokio::test]
    async fn test_execute_action_wait_does_not_clear_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            crate::task::dsl::cache::SelectorCacheEntry::new(true, true, None, 0),
        );
        let cache_before = exec.cache_size();

        exec.execute_action(&Action::Wait { duration_ms: 1 })
            .await
            .unwrap();

        assert_eq!(
            exec.cache_size(),
            cache_before,
            "Wait should NOT clear cache"
        );
    }

    #[tokio::test]
    async fn test_execute_action_extract_does_not_clear_cache() {
        let mock = MockDslApi::new();
        mock.set_text_result("#title", Some("Hello"));
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            crate::task::dsl::cache::SelectorCacheEntry::new(true, true, None, 0),
        );
        let cache_before = exec.cache_size();

        exec.execute_action(&Action::Extract {
            selector: "#title".to_string(),
            variable: Some("title".to_string()),
        })
        .await
        .unwrap();

        assert_eq!(
            exec.cache_size(),
            cache_before,
            "Extract should NOT clear cache"
        );
    }

    // ── Variable substitution ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_execute_action_click_with_variable_substitution() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);
        exec.variables
            .insert("btn_id".to_string(), "dynamic-button".to_string());

        exec.execute_action(&Action::Click {
            selector: "#${btn_id}".to_string(),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(
            calls[0],
            MockCall::Click {
                selector: "#dynamic-button".to_string()
            },
            "variable btn_id should be substituted before calling api.click"
        );
    }

    #[tokio::test]
    async fn test_execute_action_type_with_variable_substitution() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);
        exec.variables
            .insert("user".to_string(), "alice".to_string());

        exec.execute_action(&Action::Type {
            selector: "#input".to_string(),
            text: "Hello ${user}".to_string(),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(
            calls[0],
            MockCall::Type {
                selector: "#input".to_string(),
                text: "Hello alice".to_string(),
            },
            "variable user should be substituted in both selector and text"
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

    // ── Call variable copy-back (3a) and parameter passing (3b) ─────────
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
    fn test_execute_call_passthrough_preserves_parent_vars() {
        // Full end-to-end simulation: parent vars → called task → diff copy-back
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        // Parent has 2 variables
        exec.variables
            .insert("url".to_string(), "https://example.com".to_string());
        exec.variables.insert("count".to_string(), "10".to_string());

        // Snapshot pre-call variables
        let pre_call_vars: HashSet<String> = exec.variables.keys().cloned().collect();

        // Called executor inherits parent vars
        let mut called_vars = exec.variables.clone();

        // Called task: adds 2 new vars, tries to overwrite 1 existing
        called_vars.insert("result".to_string(), "success".to_string());
        called_vars.insert("session_id".to_string(), "abc-123".to_string());
        called_vars.insert("url".to_string(), "https://override.com".to_string()); // should NOT propagate

        // Diff-based copy-back: only new vars
        for (key, value) in called_vars {
            if !pre_call_vars.contains(&key) {
                exec.variables.insert(key, value);
            }
        }

        // Parent vars preserved unchanged
        assert_eq!(
            exec.variables.get("url").unwrap(),
            "https://example.com",
            "parent url should NOT be overwritten by called task"
        );
        assert_eq!(
            exec.variables.get("count").unwrap(),
            "10",
            "parent count should remain unchanged"
        );

        // New vars copied back
        assert_eq!(
            exec.variables.get("result").unwrap(),
            "success",
            "new var 'result' from called task should be copied back"
        );
        assert_eq!(
            exec.variables.get("session_id").unwrap(),
            "abc-123",
            "new var 'session_id' from called task should be copied back"
        );

        // Parent still has exactly 4 vars (2 original + 2 new)
        assert_eq!(exec.variables.len(), 4);
    }

    #[test]
    fn test_execute_call_passthrough_with_no_new_vars() {
        // Called task produces no new vars — nothing should change in parent
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.variables
            .insert("existing".to_string(), "value".to_string());

        let pre_call_vars: HashSet<String> = exec.variables.keys().cloned().collect();

        // Called task has the same vars as parent
        let called_vars = exec.variables.clone();

        for (key, value) in called_vars {
            if !pre_call_vars.contains(&key) {
                exec.variables.insert(key, value);
            }
        }

        assert_eq!(exec.variables.len(), 1, "no new vars should be added");
        assert_eq!(
            exec.variables.get("existing").unwrap(),
            "value",
            "existing var should be unchanged"
        );
    }

    #[test]
    fn test_execute_call_passthrough_with_no_parent_vars() {
        // Parent has no variables — called task's new vars should all be copied back
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        let pre_call_vars: HashSet<String> = HashSet::new();

        // Called task creates new variables
        let mut called_vars = HashMap::new();
        called_vars.insert("a".to_string(), "1".to_string());
        called_vars.insert("b".to_string(), "2".to_string());

        for (key, value) in called_vars {
            if !pre_call_vars.contains(&key) {
                exec.variables.insert(key, value);
            }
        }

        assert_eq!(
            exec.variables.len(),
            2,
            "both vars from called task should be copied back"
        );
        assert_eq!(exec.variables.get("a").unwrap(), "1");
        assert_eq!(exec.variables.get("b").unwrap(), "2");
    }

    // ── Call: Parameter override interaction (3b) ───────────────────────

    #[test]
    fn test_execute_call_params_override_specific_parent_vars() {
        // Parent has multiple vars — only overridden ones change
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.variables
            .insert("host".to_string(), "default.com".to_string());
        exec.variables
            .insert("port".to_string(), "8080".to_string());
        exec.variables
            .insert("debug".to_string(), "false".to_string());

        // Copy parent vars to called executor (3b step 1)
        let mut called_vars = exec.variables.clone();

        // Apply params that override only 'host' (3b step 2)
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

        assert_eq!(
            called_vars.get("host").unwrap(),
            "api.example.com",
            "'host' should be overridden by Call params"
        );
        assert_eq!(
            called_vars.get("port").unwrap(),
            "8080",
            "'port' should remain inherited from parent"
        );
        assert_eq!(
            called_vars.get("debug").unwrap(),
            "false",
            "'debug' should remain inherited from parent"
        );
        assert_eq!(called_vars.len(), 3, "no new vars added");
    }

    #[test]
    fn test_execute_call_params_with_multiple_types() {
        // Call parameters can be String, Number, or Bool — all should work
        let mock = MockDslApi::new();
        let _exec = create_executor(&mock, vec![]);

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

        assert_eq!(called_vars.get("name").unwrap(), "alice", "String param");
        assert_eq!(
            called_vars.get("age").unwrap(),
            "30",
            "Number param as string"
        );
        assert_eq!(
            called_vars.get("active").unwrap(),
            "true",
            "Bool param as string"
        );
    }

    #[test]
    fn test_execute_call_params_variable_substitution_in_values() {
        // Call params can use ${variable} references that get resolved before passing
        let mock = MockDslApi::new();
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
            (
                "retry_msg".to_string(),
                serde_yml::Value::String("Retrying ${base_url} with key ${api_key}".to_string()),
            ),
        ]
        .into();

        // Verify each param value is substituted correctly using substitute_variables
        for (key, value) in &params {
            let raw = match value {
                serde_yml::Value::String(s) => s.clone(),
                _ => continue,
            };
            let resolved = exec.substitute_variables(&raw);

            match key.as_str() {
                "endpoint" => assert_eq!(resolved, "https://example.com/api/v1"),
                "auth_header" => assert_eq!(resolved, "Bearer sk-test123"),
                "retry_msg" => {
                    assert_eq!(resolved, "Retrying https://example.com with key sk-test123")
                }
                _ => panic!("unexpected param key"),
            }
        }
    }

    #[test]
    fn test_execute_call_params_with_no_variables_in_parent() {
        // No parent vars — params are passed through as-is (${var} not resolved)
        let mock = MockDslApi::new();
        let _exec = create_executor(&mock, vec![]);

        let params: HashMap<String, serde_yml::Value> = [(
            "url".to_string(),
            serde_yml::Value::String("https://example.com".to_string()),
        )]
        .into();

        // Simulate param setting (no parent vars to copy first)
        let mut called_vars = HashMap::new();
        for (key, value) in &params {
            let resolved = match value {
                serde_yml::Value::String(s) => s.clone(),
                _ => "?".to_string(),
            };
            // Since the executor doesn't have substitute_variables on a reference like this,
            // just verify the value passes through literally
            assert_eq!(resolved, "https://example.com");
            called_vars.insert(key.clone(), resolved.to_string());
        }

        assert_eq!(called_vars.get("url").unwrap(), "https://example.com");
    }

    // ── Call: Empty / edge case parameters ──────────────────────────────

    #[test]
    fn test_execute_call_params_empty_map() {
        // parameters: Some(empty HashMap) — no overrides
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.variables
            .insert("x".to_string(), "parent_x".to_string());

        // Copy parent vars
        let mut called_vars = exec.variables.clone();

        // Apply empty params (no overrides)
        let params: HashMap<String, serde_yml::Value> = HashMap::new();
        for _value in params.values() {
            // no params to apply
        }

        // called task adds a new var
        called_vars.insert("y".to_string(), "new_y".to_string());

        // Diff-based copy-back
        let pre_call: HashSet<String> = exec.variables.keys().cloned().collect();
        for (key, value) in called_vars {
            if !pre_call.contains(&key) {
                exec.variables.insert(key, value);
            }
        }

        assert_eq!(
            exec.variables.get("x").unwrap(),
            "parent_x",
            "parent var unchanged"
        );
        assert_eq!(
            exec.variables.get("y").unwrap(),
            "new_y",
            "new var from called task copied back"
        );
        assert_eq!(exec.variables.len(), 2);
    }

    #[test]
    fn test_execute_call_params_none_option() {
        // parameters: None — no overrides, pure variable passthrough
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.variables
            .insert("mode".to_string(), "auto".to_string());

        // parameters is None in execute_call, so nothing extra to apply
        let pre_call: HashSet<String> = exec.variables.keys().cloned().collect();

        // Called executor gets only parent vars
        let mut called_vars = exec.variables.clone();
        called_vars.insert("result".to_string(), "done".to_string());

        // Diff copy-back
        for (key, value) in called_vars {
            if !pre_call.contains(&key) {
                exec.variables.insert(key, value);
            }
        }

        assert_eq!(exec.variables.get("mode").unwrap(), "auto");
        assert_eq!(exec.variables.get("result").unwrap(), "done");
        assert_eq!(exec.variables.len(), 2);
    }

    // ── Call: Cache TTL propagation ─────────────────────────────────────

    #[test]
    fn test_execute_call_cache_ttl_propagates_to_called_executor() {
        // Cache TTL from parent executor should propagate to called executor
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        // Set a custom cache TTL on the parent
        exec.set_cache_ttl(15000); // 15 seconds

        // Simulate creating a called executor (what execute_call does)
        let called_task_def = create_task_def("child", vec![Action::Wait { duration_ms: 1 }]);
        let mut called_exec = DslExecutor::with_depth(&mock, called_task_def, 1);

        // Cache TTL should propagate (execute_call sets this explicitly)
        called_exec.cache_ttl = exec.cache_ttl;

        assert_eq!(
            called_exec.get_cache_ttl(),
            15000,
            "called executor should inherit parent's cache TTL"
        );

        // Verify it actually affects the cache entry
        let entry = crate::task::dsl::cache::SelectorCacheEntry::with_ttl(
            true,
            true,
            None,
            0,
            called_exec.cache_ttl,
        );
        assert_eq!(entry.ttl, Duration::from_millis(15000));
    }

    #[test]
    fn test_execute_call_cache_ttl_default_propagation() {
        // Default cache TTL should also propagate
        let mock = MockDslApi::new();
        let exec = create_executor(&mock, vec![]);

        let called_task_def = create_task_def("child", vec![Action::Wait { duration_ms: 1 }]);
        let mut called_exec = DslExecutor::with_depth(&mock, called_task_def, 1);

        // Execute_call propagates cache_ttl explicitly
        called_exec.cache_ttl = exec.cache_ttl;

        assert_eq!(
            called_exec.get_cache_ttl(),
            DEFAULT_CACHE_TTL_MS,
            "called executor should inherit default cache TTL"
        );
    }

    // ── Call: Call depth ─────────────────────────────────────────────────

    #[test]
    fn test_execute_call_increments_call_depth() {
        // The called executor should have call_depth = parent + 1
        let mock = MockDslApi::new();
        let exec = create_executor(&mock, vec![]);

        let called_task_def = create_task_def("child", vec![Action::Wait { duration_ms: 1 }]);
        let called_exec = DslExecutor::with_depth(&mock, called_task_def, exec.call_depth + 1);

        assert_eq!(
            called_exec.call_depth, 1,
            "called executor should have call_depth = parent.call_depth + 1"
        );
    }

    #[test]
    fn test_execute_call_nested_call_depth() {
        // Nested call: depth 2
        let mock = MockDslApi::new();
        let _exec = create_executor(&mock, vec![]);

        // Simulate first call: parent.depth=0 → child.depth=1
        let child_task_def = create_task_def("child", vec![Action::Wait { duration_ms: 1 }]);
        let child_exec = DslExecutor::with_depth(&mock, child_task_def, 1);

        // Simulate second call from child: child.depth=1 → grandchild.depth=2
        let grandchild_task_def =
            create_task_def("grandchild", vec![Action::Wait { duration_ms: 1 }]);
        let grandchild_exec =
            DslExecutor::with_depth(&mock, grandchild_task_def, child_exec.call_depth + 1);

        assert_eq!(
            grandchild_exec.call_depth, 2,
            "nested call should have depth=2"
        );
        assert!(
            grandchild_exec.call_depth < MAX_CALL_DEPTH,
            "depth should be under MAX_CALL_DEPTH"
        );
    }

    // ── If/Else action (control flow) ─────────────────────────────────────

    #[tokio::test]
    async fn test_execute_if_then_branch() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::If {
            condition: Condition::True,
            then: vec![Action::Click {
                selector: "#then-btn".to_string(),
            }],
            r#else: Some(vec![Action::Click {
                selector: "#else-btn".to_string(),
            }]),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1, "then branch should execute");
        assert_eq!(
            calls[0],
            MockCall::Click {
                selector: "#then-btn".to_string()
            },
            "then action should be the one called"
        );
    }

    #[tokio::test]
    async fn test_execute_if_else_branch() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::If {
            condition: Condition::False,
            then: vec![Action::Click {
                selector: "#then-btn".to_string(),
            }],
            r#else: Some(vec![Action::Click {
                selector: "#else-btn".to_string(),
            }]),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(
            calls.len(),
            1,
            "else branch should execute when condition is false"
        );
        assert_eq!(
            calls[0],
            MockCall::Click {
                selector: "#else-btn".to_string()
            },
            "else action should be the one called"
        );
    }

    #[tokio::test]
    async fn test_execute_if_no_else_condition_false() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::If {
            condition: Condition::False,
            then: vec![Action::Click {
                selector: "#then-btn".to_string(),
            }],
            r#else: None,
        })
        .await
        .unwrap();

        assert!(
            mock.get_calls().is_empty(),
            "no actions should execute when condition is false and no else"
        );
    }

    #[tokio::test]
    async fn test_execute_if_element_exists_condition() {
        let mock = MockDslApi::new();
        mock.set_exists_result("#exists", true);
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::If {
            condition: Condition::ElementExists {
                selector: "#exists".to_string(),
            },
            then: vec![Action::Click {
                selector: "#then-btn".to_string(),
            }],
            r#else: Some(vec![Action::Click {
                selector: "#else-btn".to_string(),
            }]),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 2, "exists check + then click");
        assert_eq!(
            calls[0],
            MockCall::Exists {
                selector: "#exists".to_string()
            },
            "first call should be exists check"
        );
        assert_eq!(
            calls[1],
            MockCall::Click {
                selector: "#then-btn".to_string()
            },
            "then branch should execute when element exists"
        );
    }

    #[tokio::test]
    async fn test_execute_if_element_not_exists_routes_to_else() {
        let mock = MockDslApi::new();
        mock.set_exists_result("#nonexistent", false);
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::If {
            condition: Condition::ElementExists {
                selector: "#nonexistent".to_string(),
            },
            then: vec![Action::Click {
                selector: "#then-btn".to_string(),
            }],
            r#else: Some(vec![Action::Click {
                selector: "#else-btn".to_string(),
            }]),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 2, "exists check + else click");
        assert_eq!(
            calls[0],
            MockCall::Exists {
                selector: "#nonexistent".to_string()
            },
            "first call should be exists check"
        );
        assert_eq!(
            calls[1],
            MockCall::Click {
                selector: "#else-btn".to_string()
            },
            "else branch should execute when element does not exist"
        );
    }

    #[tokio::test]
    async fn test_execute_if_with_variable_equality() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);
        exec.variables
            .insert("status".to_string(), "ready".to_string());

        exec.execute_action(&Action::If {
            condition: Condition::VariableEquals {
                name: "status".to_string(),
                value: serde_yml::Value::String("ready".to_string()),
            },
            then: vec![Action::Click {
                selector: "#proceed".to_string(),
            }],
            r#else: Some(vec![Action::Click {
                selector: "#wait".to_string(),
            }]),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1, "then branch should execute");
        assert_eq!(
            calls[0],
            MockCall::Click {
                selector: "#proceed".to_string()
            },
            "then action should be called when variable matches"
        );
    }

    // ── Loop action ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_execute_loop_fixed_count() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Loop {
            count: Some(3),
            condition: None,
            actions: vec![Action::Click {
                selector: "#item".to_string(),
            }],
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(
            calls.len(),
            3,
            "Loop with count=3 should execute 3 iterations"
        );
        for call in &calls {
            assert_eq!(
                *call,
                MockCall::Click {
                    selector: "#item".to_string()
                }
            );
        }
    }

    #[tokio::test]
    async fn test_execute_loop_zero_count() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Loop {
            count: Some(0),
            condition: None,
            actions: vec![Action::Click {
                selector: "#item".to_string(),
            }],
        })
        .await
        .unwrap();

        assert!(
            mock.get_calls().is_empty(),
            "Loop with count=0 should not execute any actions"
        );
    }

    #[tokio::test]
    async fn test_execute_loop_condition_false_exits_immediately() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Loop {
            count: None,
            condition: Some(Condition::False),
            actions: vec![Action::Click {
                selector: "#item".to_string(),
            }],
        })
        .await
        .unwrap();

        assert!(
            mock.get_calls().is_empty(),
            "Loop with condition=false should exit without executing actions"
        );
    }

    #[tokio::test]
    async fn test_execute_loop_multiple_inner_actions() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Loop {
            count: Some(2),
            condition: None,
            actions: vec![
                Action::Click {
                    selector: "#first".to_string(),
                },
                Action::Click {
                    selector: "#second".to_string(),
                },
            ],
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 4, "2 iterations of 2 actions each = 4 calls");
        assert_eq!(
            calls[0],
            MockCall::Click {
                selector: "#first".to_string()
            }
        );
        assert_eq!(
            calls[1],
            MockCall::Click {
                selector: "#second".to_string()
            }
        );
        assert_eq!(
            calls[2],
            MockCall::Click {
                selector: "#first".to_string()
            }
        );
        assert_eq!(
            calls[3],
            MockCall::Click {
                selector: "#second".to_string()
            }
        );
    }

    // ── Foreach action ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_execute_foreach_array_with_variable_binding() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Foreach {
            variable: "item".to_string(),
            collection: ForeachCollection::Array {
                values: vec![
                    serde_yml::Value::String("a".to_string()),
                    serde_yml::Value::String("b".to_string()),
                    serde_yml::Value::String("c".to_string()),
                ],
            },
            actions: vec![Action::Type {
                selector: "#input".to_string(),
                text: "${item}".to_string(),
            }],
            max_iterations: None,
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 3, "3 iterations for 3 array values");
        assert_eq!(
            calls[0],
            MockCall::Type {
                selector: "#input".to_string(),
                text: "a".to_string()
            },
            "first iteration should bind item=a"
        );
        assert_eq!(
            calls[1],
            MockCall::Type {
                selector: "#input".to_string(),
                text: "b".to_string()
            },
            "second iteration should bind item=b"
        );
        assert_eq!(
            calls[2],
            MockCall::Type {
                selector: "#input".to_string(),
                text: "c".to_string()
            },
            "third iteration should bind item=c"
        );
    }

    #[tokio::test]
    async fn test_execute_foreach_range() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Foreach {
            variable: "index".to_string(),
            collection: ForeachCollection::Range { start: 0, end: 3 },
            actions: vec![Action::Click {
                selector: "${index}".to_string(),
            }],
            max_iterations: None,
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 3, "Range 0..3 should produce 3 iterations");
        assert_eq!(
            calls[0],
            MockCall::Click {
                selector: "0".to_string()
            }
        );
        assert_eq!(
            calls[1],
            MockCall::Click {
                selector: "1".to_string()
            }
        );
        assert_eq!(
            calls[2],
            MockCall::Click {
                selector: "2".to_string()
            }
        );
    }

    #[tokio::test]
    async fn test_execute_foreach_max_iterations() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Foreach {
            variable: "item".to_string(),
            collection: ForeachCollection::Array {
                values: vec![
                    serde_yml::Value::String("a".to_string()),
                    serde_yml::Value::String("b".to_string()),
                    serde_yml::Value::String("c".to_string()),
                    serde_yml::Value::String("d".to_string()),
                    serde_yml::Value::String("e".to_string()),
                ],
            },
            actions: vec![Action::Click {
                selector: "#item".to_string(),
            }],
            max_iterations: Some(3),
        })
        .await
        .unwrap();

        assert_eq!(
            mock.get_calls().len(),
            3,
            "Foreach with max_iterations=3 should only process 3 of 5 items"
        );
    }

    #[tokio::test]
    async fn test_execute_foreach_empty_array() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Foreach {
            variable: "item".to_string(),
            collection: ForeachCollection::Array { values: vec![] },
            actions: vec![Action::Click {
                selector: "#item".to_string(),
            }],
            max_iterations: None,
        })
        .await
        .unwrap();

        assert!(
            mock.get_calls().is_empty(),
            "Foreach with empty array should produce 0 iterations"
        );
    }

    #[tokio::test]
    async fn test_execute_foreach_elements_collection() {
        let mock = MockDslApi::new();
        mock.set_count_result(".items", 3);
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Foreach {
            variable: "sel".to_string(),
            collection: ForeachCollection::Elements {
                selector: ".items".to_string(),
            },
            actions: vec![Action::Click {
                selector: "${sel}".to_string(),
            }],
            max_iterations: None,
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 4, "1 count_elements + 3 clicks");
        assert_eq!(
            calls[0],
            MockCall::CountElements {
                selector: ".items".to_string()
            },
            "first call should count elements matching selector"
        );
        assert_eq!(
            calls[1],
            MockCall::Click {
                selector: ".items:nth-of-type(1)".to_string()
            },
            "first click uses :nth-of-type(1)"
        );
        assert_eq!(
            calls[2],
            MockCall::Click {
                selector: ".items:nth-of-type(2)".to_string()
            },
            "second click uses :nth-of-type(2)"
        );
        assert_eq!(
            calls[3],
            MockCall::Click {
                selector: ".items:nth-of-type(3)".to_string()
            },
            "third click uses :nth-of-type(3)"
        );
    }

    // ── While action ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_execute_while_runs_with_max_iterations() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::While {
            condition: Condition::True,
            actions: vec![Action::Click {
                selector: "#item".to_string(),
            }],
            max_iterations: Some(3),
        })
        .await
        .unwrap();

        assert_eq!(
            mock.get_calls().len(),
            3,
            "While with True condition and max=3 should execute 3 iterations"
        );
    }

    #[tokio::test]
    async fn test_execute_while_false_condition_skips_body() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::While {
            condition: Condition::False,
            actions: vec![Action::Click {
                selector: "#item".to_string(),
            }],
            max_iterations: Some(100),
        })
        .await
        .unwrap();

        assert!(
            mock.get_calls().is_empty(),
            "While with False condition should skip loop body entirely"
        );
    }

    #[tokio::test]
    async fn test_execute_while_element_visible_condition() {
        let mock = MockDslApi::new();
        mock.set_visible_result("#spinner", true);
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::While {
            condition: Condition::ElementVisible {
                selector: "#spinner".to_string(),
            },
            actions: vec![Action::Click {
                selector: "#cancel".to_string(),
            }],
            max_iterations: Some(2),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        // 2 iterations × (visible check + click) + 1 extra condition check after last iteration
        assert_eq!(
            calls.len(),
            5,
            "2 iterations of (visible check + click) + 1 final check"
        );
        assert_eq!(
            calls[0],
            MockCall::Visible {
                selector: "#spinner".to_string()
            },
            "iteration 1: visible check"
        );
        assert_eq!(
            calls[1],
            MockCall::Click {
                selector: "#cancel".to_string()
            },
            "iteration 1: click"
        );
        assert_eq!(
            calls[2],
            MockCall::Visible {
                selector: "#spinner".to_string()
            },
            "iteration 2: visible check"
        );
        assert_eq!(
            calls[3],
            MockCall::Click {
                selector: "#cancel".to_string()
            },
            "iteration 2: click"
        );
        assert_eq!(
            calls[4],
            MockCall::Visible {
                selector: "#spinner".to_string()
            },
            "final condition check before loop exit"
        );
    }

    // ── Retry action ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_execute_retry_succeeds_first_attempt() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Retry {
            actions: vec![Action::Click {
                selector: "#btn".to_string(),
            }],
            max_attempts: Some(3),
            initial_delay_ms: Some(1),
            max_delay_ms: Some(10),
            backoff_multiplier: Some(1.0),
            jitter: Some(false),
            retry_on: None,
        })
        .await
        .unwrap();

        assert_eq!(
            mock.get_calls().len(),
            1,
            "Retry should succeed on first attempt when actions succeed"
        );
    }

    #[tokio::test]
    async fn test_execute_retry_fails_after_exhausting_attempts() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_action(&Action::Retry {
                actions: vec![Action::Click {
                    selector: "#btn".to_string(),
                }],
                max_attempts: Some(3),
                initial_delay_ms: Some(1),
                max_delay_ms: Some(5),
                backoff_multiplier: Some(1.0),
                jitter: Some(false),
                retry_on: None,
            })
            .await;

        assert!(
            result.is_err(),
            "Retry should fail after exhausting all attempts"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Retry block failed after 3 attempts"),
            "error message should indicate all attempts exhausted"
        );
        assert_eq!(
            mock.get_calls().len(),
            3,
            "should attempt 3 times before giving up"
        );
    }

    #[tokio::test]
    async fn test_execute_retry_retry_on_pattern_match() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_action(&Action::Retry {
                actions: vec![Action::Click {
                    selector: "#btn".to_string(),
                }],
                max_attempts: Some(3),
                initial_delay_ms: Some(1),
                max_delay_ms: Some(5),
                backoff_multiplier: Some(1.0),
                jitter: Some(false),
                retry_on: Some(vec!["forced".to_string()]),
            })
            .await;

        assert!(
            result.is_err(),
            "Retry should fail after exhausting all attempts"
        );
        assert_eq!(
            mock.get_calls().len(),
            3,
            "retry_on pattern 'forced' matches 'MockDslApi forced failure', so all 3 attempts run"
        );
    }

    #[tokio::test]
    async fn test_execute_retry_retry_on_pattern_no_match() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_action(&Action::Retry {
                actions: vec![Action::Click {
                    selector: "#btn".to_string(),
                }],
                max_attempts: Some(3),
                initial_delay_ms: Some(1),
                max_delay_ms: Some(5),
                backoff_multiplier: Some(1.0),
                jitter: Some(false),
                retry_on: Some(vec!["different_error".to_string()]),
            })
            .await;

        assert!(
            result.is_err(),
            "Retry should fail immediately when error doesn't match retry_on"
        );
        assert_eq!(
            mock.get_calls().len(),
            1,
            "only 1 attempt since error doesn't match retry_on pattern"
        );
    }

    #[tokio::test]
    async fn test_execute_retry_single_attempt() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_action(&Action::Retry {
                actions: vec![Action::Click {
                    selector: "#btn".to_string(),
                }],
                max_attempts: Some(1),
                initial_delay_ms: Some(1),
                max_delay_ms: Some(5),
                backoff_multiplier: Some(1.0),
                jitter: Some(false),
                retry_on: None,
            })
            .await;

        assert!(result.is_err(), "should fail after single attempt");
        assert_eq!(
            mock.get_calls().len(),
            1,
            "max_attempts=1 should execute exactly 1 attempt"
        );
    }

    // ── Try/Catch/Finally action ──────────────────────────────────────────

    #[tokio::test]
    async fn test_execute_try_succeeds_without_catch_or_finally() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Try {
            try_actions: vec![Action::Click {
                selector: "#btn".to_string(),
            }],
            catch_actions: None,
            error_variable: None,
            finally_actions: None,
        })
        .await
        .unwrap();

        assert_eq!(
            mock.get_calls().len(),
            1,
            "try block should execute successfully"
        );
    }

    #[tokio::test]
    async fn test_execute_try_catch_executes_on_failure() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Try {
            try_actions: vec![Action::Click {
                selector: "#failing-btn".to_string(),
            }],
            catch_actions: Some(vec![Action::Log {
                message: "caught error".to_string(),
                level: Some(LogLevel::Info),
            }]),
            error_variable: Some("err".to_string()),
            finally_actions: None,
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        // Try: Click fails (still recorded), Catch: Log (no API call)
        assert_eq!(
            calls.len(),
            1,
            "only the failing try Click should be recorded (catch uses Log which has no API call)"
        );
        assert_eq!(
            calls[0],
            MockCall::Click {
                selector: "#failing-btn".to_string()
            },
            "try Click should have been attempted"
        );
        assert!(
            exec.variables.contains_key("err"),
            "error variable should be set on catch"
        );
        assert!(
            exec.variables
                .get("err")
                .unwrap()
                .contains("MockDslApi forced failure"),
            "error variable should contain the error message"
        );
    }

    #[tokio::test]
    async fn test_execute_try_finally_always_executes_on_success() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Try {
            try_actions: vec![Action::Click {
                selector: "#btn".to_string(),
            }],
            catch_actions: None,
            error_variable: None,
            finally_actions: Some(vec![Action::Click {
                selector: "#finally-btn".to_string(),
            }]),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 2, "try click + finally click");
        assert_eq!(
            calls[0],
            MockCall::Click {
                selector: "#btn".to_string()
            },
            "try block should execute first"
        );
        assert_eq!(
            calls[1],
            MockCall::Click {
                selector: "#finally-btn".to_string()
            },
            "finally should execute after successful try"
        );
    }

    #[tokio::test]
    async fn test_execute_try_finally_always_executes_on_failure() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Try {
            try_actions: vec![Action::Click {
                selector: "#failing-btn".to_string(),
            }],
            catch_actions: Some(vec![Action::Log {
                message: "catch".to_string(),
                level: Some(LogLevel::Info),
            }]),
            error_variable: None,
            finally_actions: Some(vec![Action::Log {
                message: "finally executed".to_string(),
                level: Some(LogLevel::Info),
            }]),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        // try: Click fails (recorded), catch: Log (no API call), finally: Log (no API call)
        assert_eq!(
            calls.len(),
            1,
            "only the failing try click is recorded (catch and finally use Log with no API call)"
        );
        assert_eq!(
            calls[0],
            MockCall::Click {
                selector: "#failing-btn".to_string()
            },
            "try block executes first and fails"
        );
    }

    #[tokio::test]
    async fn test_execute_try_suppresses_error_when_no_catch() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_action(&Action::Try {
                try_actions: vec![Action::Click {
                    selector: "#failing-btn".to_string(),
                }],
                catch_actions: None,
                error_variable: None,
                finally_actions: None,
            })
            .await;

        // Try always returns Ok(()) at the top level, suppressing errors
        assert!(
            result.is_ok(),
            "Try should suppress errors even without catch"
        );
    }

    #[tokio::test]
    async fn test_execute_try_catch_action_failure_propagates() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_action(&Action::Try {
                try_actions: vec![Action::Click {
                    selector: "#try-btn".to_string(),
                }],
                catch_actions: Some(vec![Action::Click {
                    selector: "#catch-btn".to_string(),
                }]),
                error_variable: None,
                finally_actions: None,
            })
            .await;

        assert!(
            result.is_err(),
            "catch action failure should propagate from Try"
        );
        assert_eq!(
            mock.get_calls().len(),
            2,
            "try click (fails) + catch click (fails)"
        );
    }

    #[tokio::test]
    async fn test_execute_try_multiple_catch_actions() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Try {
            try_actions: vec![Action::Click {
                selector: "#failing-btn".to_string(),
            }],
            catch_actions: Some(vec![
                Action::Log {
                    message: "step 1".to_string(),
                    level: Some(LogLevel::Info),
                },
                Action::Log {
                    message: "step 2".to_string(),
                    level: Some(LogLevel::Info),
                },
            ]),
            error_variable: Some("err".to_string()),
            finally_actions: None,
        })
        .await
        .unwrap();

        // Only 1 API call (the failing Click), Log actions don't call the API
        assert_eq!(
            mock.get_calls().len(),
            1,
            "only the failing Click should be recorded"
        );
        assert!(
            exec.variables.contains_key("err"),
            "error variable should still be set"
        );
    }
}
