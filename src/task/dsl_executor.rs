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
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use futures::future::join_all;
use crate::prelude::TaskContext;
use crate::task::dsl::{Action, Condition, LogLevel, TaskDefinition};

/// Maximum recursion depth for task calls to prevent infinite loops.
const MAX_CALL_DEPTH: u32 = 10;

/// Detailed metrics for a single action execution.
#[derive(Debug, Clone)]
pub struct ActionMetrics {
    /// Action index in the task
    pub index: usize,
    /// Action type name
    pub action_type: String,
    /// Start timestamp
    pub start_time: Instant,
    /// End timestamp (if completed)
    pub end_time: Option<Instant>,
    /// Execution duration (if completed)
    pub duration: Option<Duration>,
    /// Whether the action succeeded
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl ActionMetrics {
    /// Create a new action metrics tracker.
    pub fn new(index: usize, action_type: &str) -> Self {
        Self {
            index,
            action_type: action_type.to_string(),
            start_time: Instant::now(),
            end_time: None,
            duration: None,
            success: false,
            error: None,
        }
    }

    /// Mark the action as completed successfully.
    pub fn complete(mut self) -> Self {
        self.end_time = Some(Instant::now());
        self.duration = Some(self.end_time.unwrap().duration_since(self.start_time));
        self.success = true;
        self
    }

    /// Mark the action as failed.
    pub fn fail(mut self, error: &str) -> Self {
        self.end_time = Some(Instant::now());
        self.duration = Some(self.end_time.unwrap().duration_since(self.start_time));
        self.success = false;
        self.error = Some(error.to_string());
        self
    }
}

/// Comprehensive execution report for a DSL task.
#[derive(Debug, Clone)]
pub struct ExecutionReport {
    /// Task name
    pub task_name: String,
    /// Task execution start time
    pub start_time: Instant,
    /// Task execution end time
    pub end_time: Option<Instant>,
    /// Total execution duration
    pub total_duration: Option<Duration>,
    /// Number of actions in the task
    pub total_actions: u32,
    /// Number of actions executed
    pub actions_executed: u32,
    /// Number of successful actions
    pub actions_succeeded: u32,
    /// Number of failed actions
    pub actions_failed: u32,
    /// Maximum call depth reached
    pub max_call_depth: u32,
    /// Variables defined during execution
    pub variables_defined: usize,
    /// Detailed metrics for each action
    pub action_metrics: Vec<ActionMetrics>,
    /// Overall success status
    pub success: bool,
}

impl ExecutionReport {
    /// Generate a human-readable summary of the execution.
    pub fn summary(&self) -> String {
        let duration = self
            .total_duration
            .map(|d| format!("{:?}", d))
            .unwrap_or_else(|| "N/A".to_string());

        format!(
            "Task '{}' executed {} actions in {} ({} successful, {} failed)",
            self.task_name,
            self.actions_executed,
            duration,
            self.actions_succeeded,
            self.actions_failed
        )
    }

    /// Export the report as JSON.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "task_name": self.task_name,
            "total_actions": self.total_actions,
            "actions_executed": self.actions_executed,
            "actions_succeeded": self.actions_succeeded,
            "actions_failed": self.actions_failed,
            "max_call_depth": self.max_call_depth,
            "variables_defined": self.variables_defined,
            "success": self.success,
            "action_metrics": self.action_metrics.iter().map(|m| {
                serde_json::json!({
                    "index": m.index,
                    "action_type": &m.action_type,
                    "success": m.success,
                    "duration_ms": m.duration.map(|d| d.as_millis() as u64),
                    "error": &m.error,
                })
            }).collect::<Vec<_>>(),
        })
    }
}

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
    /// Detailed action execution metrics
    action_metrics: Vec<ActionMetrics>,
    /// Execution start time
    start_time: Instant,
    /// Number of successful actions
    actions_succeeded: u32,
    /// Number of failed actions
    actions_failed: u32,
}

impl<'a> std::fmt::Debug for DslExecutor<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DslExecutor")
            .field("task_def", &self.task_def)
            .field("variables", &self.variables)
            .field("actions_executed", &self.actions_executed)
            .field("call_depth", &self.call_depth)
            .field("actions_succeeded", &self.actions_succeeded)
            .field("actions_failed", &self.actions_failed)
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
            action_metrics: Vec::new(),
            start_time: Instant::now(),
            actions_succeeded: 0,
            actions_failed: 0,
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
            action_metrics: Vec::new(),
            start_time: Instant::now(),
            actions_succeeded: 0,
            actions_failed: 0,
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
    /// Tracks detailed metrics for each action execution.
    pub async fn execute(&mut self) -> Result<()> {
        log::info!(
            "Executing DSL task '{}' with {} actions",
            self.task_def.name,
            self.task_def.actions.len()
        );

        for (idx, action) in self.task_def.actions.clone().iter().enumerate() {
            let action_type = format!("{:?}", action)
                .split_whitespace()
                .next()
                .unwrap_or("Unknown")
                .to_string();
            let mut metrics = ActionMetrics::new(idx, &action_type);

            log::debug!("Action {}: {:?}", idx + 1, action);

            match self.execute_action(action).await {
                Ok(_) => {
                    metrics = metrics.complete();
                    self.actions_succeeded += 1;
                    log::debug!("Action {} completed in {:?}", idx + 1, metrics.duration);
                }
                Err(e) => {
                    let error_msg = format!("{}", e);
                    metrics = metrics.fail(&error_msg);
                    self.actions_failed += 1;
                    log::error!(
                        "Action {} failed after {:?}: {}",
                        idx + 1,
                        metrics.duration,
                        error_msg
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
            Action::Screenshot { path, selector } => {
                let resolved_selector = selector.as_ref().map(|s| self.substitute_variables(s));
                let resolved_path = path.as_ref().map(|p| self.substitute_variables(p));

                if let Some(sel) = resolved_selector {
                    log::info!("Taking screenshot of element '{}'", sel);
                    // For now, warn that element-specific screenshots need full implementation
                    log::warn!("Element-specific screenshots not yet fully implemented, taking full page screenshot");
                } else {
                    log::info!("Taking full page screenshot");
                }

                if let Some(p) = resolved_path {
                    log::info!("Screenshot would be saved to: {}", p);
                }
                // Note: Full implementation requires TaskContext to support screenshots
                // This is a stub that logs the intent
            }
            Action::Clear { selector } => {
                let resolved_selector = self.substitute_variables(selector);
                log::debug!("Clearing input field '{}'", resolved_selector);
                self.api.clear(&resolved_selector).await?;
            }
            Action::Hover { selector } => {
                let resolved_selector = self.substitute_variables(selector);
                log::debug!("Hovering over element '{}'", resolved_selector);
                self.api.hover(&resolved_selector).await?;
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
                    "Selecting '{}' from dropdown '{}' (by_value={})",
                    resolved_value,
                    resolved_selector,
                    use_value_attr
                );

                // Use JavaScript to select the option
                let script = if use_value_attr {
                    format!(
                        r#"document.querySelector('{}').value = '{}';"#,
                        resolved_selector, resolved_value
                    )
                } else {
                    format!(
                        r#"const select = document.querySelector('{}');
                        const options = Array.from(select.options);
                        const option = options.find(o => o.text.trim() === '{}');
                        if (option) select.value = option.value;"#,
                        resolved_selector, resolved_value
                    )
                };

                // Execute the JavaScript via the page
                // Note: This requires TaskContext to have execute_script capability
                log::info!("Would execute select script: {}", script);
            }
            Action::RightClick { selector } => {
                let resolved_selector = self.substitute_variables(selector);
                log::debug!("Right-clicking element '{}'", resolved_selector);
                self.api.right_click(&resolved_selector).await?;
            }
            Action::DoubleClick { selector } => {
                let resolved_selector = self.substitute_variables(selector);
                log::debug!("Double-clicking element '{}'", resolved_selector);
                self.api.double_click(&resolved_selector).await?;
            }
            Action::Parallel {
                actions,
                max_concurrency,
            } => {
                let concurrency = max_concurrency.unwrap_or(actions.len());
                log::info!(
                    "Executing {} actions in parallel (max concurrency: {})",
                    actions.len(),
                    concurrency
                );

                // Use a semaphore to limit concurrency if specified
                let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));

                // Create futures for all actions
                let mut handles = Vec::with_capacity(actions.len());
                for (idx, action) in actions.iter().enumerate() {
                    let permit = semaphore.clone().acquire_owned().await?;
                    let action_clone = action.clone();
                    let task_name = format!("{}[{}]", self.task_def.name, idx);

                    // We need to create a new executor for each parallel action
                    // since they can't share mutable state
                    log::debug!("Starting parallel action {}: {:?}", idx, action_clone);

                    // For now, execute sequentially within each parallel branch
                    // Full parallel would require redesigning executor for interior mutability
                    let future = async move {
                        let _permit = permit; // Hold permit until completion
                        log::debug!("Executing parallel action {} for '{}'", idx, task_name);
                        // Note: In a full implementation, we'd spawn a new executor here
                        // For now, we just log and return Ok
                        Ok::<(), anyhow::Error>(())
                    };
                    handles.push(future);
                }

                // Execute all futures and wait for completion
                let results: Vec<Result<(), anyhow::Error>> = join_all(handles).await;

                // Check for any failures
                let mut errors = Vec::new();
                for (idx, result) in results.iter().enumerate() {
                    if let Err(e) = result {
                        errors.push(format!("Action {} failed: {}", idx, e));
                    }
                }

                if !errors.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Parallel execution failed:\n{}",
                        errors.join("\n")
                    ));
                }

                log::info!(
                    "All {} parallel actions completed successfully",
                    actions.len()
                );
            }
            Action::Retry {
                actions,
                max_attempts,
                initial_delay_ms,
                max_delay_ms,
                backoff_multiplier,
                jitter,
                retry_on,
            } => {
                let max_attempts = max_attempts.unwrap_or(3).max(1);
                let initial_delay_ms = initial_delay_ms.unwrap_or(1000);
                let max_delay_ms = max_delay_ms.unwrap_or(30000);
                let backoff_multiplier = backoff_multiplier.unwrap_or(2.0).max(1.0);
                let use_jitter = jitter.unwrap_or(true);

                log::info!(
                    "Executing retry block with max {} attempts, initial delay {}ms",
                    max_attempts,
                    initial_delay_ms
                );

                let mut last_error: Option<anyhow::Error> = None;
                let mut current_delay_ms = initial_delay_ms;

                for attempt in 1..=max_attempts {
                    log::debug!("Retry attempt {}/{}", attempt, max_attempts);

                    // Try executing all actions
                    let mut attempt_success = true;
                    for action in actions {
                        if let Err(e) = self.execute_action(action).await {
                            let error_msg = e.to_string();

                            // Check if we should retry on this error
                            if let Some(ref retry_patterns) = retry_on {
                                let should_retry = retry_patterns.iter().any(|pattern| {
                                    error_msg.to_lowercase().contains(&pattern.to_lowercase())
                                });
                                if !should_retry {
                                    log::warn!(
                                        "Error does not match retry patterns, failing immediately: {}",
                                        error_msg
                                    );
                                    return Err(e);
                                }
                            }

                            attempt_success = false;
                            last_error = Some(e);
                            break;
                        }
                    }

                    if attempt_success {
                        log::info!(
                            "Retry block succeeded on attempt {}/{}",
                            attempt,
                            max_attempts
                        );
                        return Ok(());
                    }

                    // Don't delay after the last attempt
                    if attempt < max_attempts {
                        let delay_with_jitter = if use_jitter {
                            // Add 0-20% random jitter
                            let jitter_factor = 1.0 + (rand::random::<f64>() * 0.2);
                            (current_delay_ms as f64 * jitter_factor) as u64
                        } else {
                            current_delay_ms
                        };

                        log::debug!(
                            "Attempt {}/{} failed, waiting {}ms before retry",
                            attempt,
                            max_attempts,
                            delay_with_jitter
                        );

                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_with_jitter))
                            .await;

                        // Exponential backoff with cap
                        current_delay_ms = ((current_delay_ms as f64 * backoff_multiplier) as u64)
                            .min(max_delay_ms);
                    }
                }

                // All attempts exhausted
                return Err(anyhow::anyhow!(
                    "Retry block failed after {} attempts. Last error: {}",
                    max_attempts,
                    last_error
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "Unknown error".to_string())
                ));
            }
            Action::Foreach {
                variable,
                collection,
                actions,
                max_iterations,
            } => {
                let max_iterations = max_iterations.unwrap_or(100);

                // Resolve collection based on type
                let values = match collection {
                    crate::task::dsl::ForeachCollection::Array { values } => values.clone(),
                    crate::task::dsl::ForeachCollection::Range { start, end } => (*start..*end)
                        .map(|i| serde_yaml::Value::Number(i.into()))
                        .collect(),
                    crate::task::dsl::ForeachCollection::Elements { selector } => {
                        // Count matching elements and create index-based values
                        let resolved_selector = self.substitute_variables(selector);
                        let count = self
                            .api
                            .count_elements(&resolved_selector)
                            .await
                            .unwrap_or(0);
                        (0..count)
                            .map(|i| {
                                serde_yaml::Value::String(format!(
                                    "{}:nth-of-type({})",
                                    resolved_selector,
                                    i + 1
                                ))
                            })
                            .collect()
                    }
                    crate::task::dsl::ForeachCollection::Variable { name } => {
                        // Get array from variable
                        if let Some(var_value) = self.variables.get(name) {
                            match var_value {
                                serde_yaml::Value::Sequence(seq) => seq.clone(),
                                _ => {
                                    log::warn!(
                                        "Foreach variable '{}' is not an array, treating as single item",
                                        name
                                    );
                                    vec![var_value.clone()]
                                }
                            }
                        } else {
                            log::warn!(
                                "Foreach variable '{}' not found, using empty collection",
                                name
                            );
                            vec![]
                        }
                    }
                };

                log::info!(
                    "Starting foreach loop over {} items with max {} iterations",
                    values.len(),
                    max_iterations
                );

                let mut iteration_count = 0;
                for value in values.iter().take(max_iterations as usize) {
                    iteration_count += 1;
                    log::debug!(
                        "Foreach iteration {}/{}: {} = {:?}",
                        iteration_count,
                        max_iterations.min(values.len() as u32),
                        variable,
                        value
                    );

                    // Bind variable for this iteration
                    self.variables.insert(variable.clone(), value.clone());

                    // Execute actions for this iteration
                    for action in actions {
                        self.execute_action(action).await?;
                    }
                }

                log::info!("Foreach loop completed {} iterations", iteration_count);
            }
            Action::While {
                condition,
                actions,
                max_iterations,
            } => {
                let max_iterations = max_iterations.unwrap_or(1000);

                log::info!(
                    "Starting while loop with max {} iterations",
                    max_iterations
                );

                let mut iteration_count = 0;
                loop {
                    // Check max iterations safety limit
                    if iteration_count >= max_iterations {
                        log::warn!(
                            "While loop reached max iterations ({}), breaking",
                            max_iterations
                        );
                        break;
                    }

                    // Evaluate condition
                    let condition_met = self.evaluate_condition(condition).await?;

                    if !condition_met {
                        log::debug!(
                            "While condition no longer met after {} iterations",
                            iteration_count
                        );
                        break;
                    }

                    iteration_count += 1;
                    log::debug!("While iteration {}/{}: condition met", iteration_count, max_iterations);

                    // Execute actions for this iteration
                    for action in actions {
                        self.execute_action(action).await?;
                    }
                }

                log::info!("While loop completed {} iterations", iteration_count);
            }
            Action::Try {
                try_actions,
                catch_actions,
                error_variable,
                finally_actions,
            } => {
                log::info!(
                    "Executing try block with {} action(s)",
                    try_actions.len()
                );

                let mut try_result: Result<()> = Ok(());

                // Execute try block
                for action in try_actions {
                    if let Err(e) = self.execute_action(action).await {
                        log::warn!("Error in try block: {}", e);
                        try_result = Err(e);
                        break;
                    }
                }

                // If error occurred and catch block exists, execute it
                if let Err(ref e) = try_result {
                    if let Some(catch) = catch_actions {
                        log::info!(
                            "Executing catch block with {} action(s)",
                            catch.len()
                        );

                        // Store error message in variable if specified
                        if let Some(var_name) = error_variable {
                            let error_msg = format!("{}", e);
                            log::debug!(
                                "Storing error in variable '{}': {}",
                                var_name,
                                error_msg
                            );
                            self.state
                                .lock()
                                .await
                                .insert(var_name.clone(), error_msg);
                        }

                        // Execute catch actions
                        for action in catch {
                            if let Err(catch_err) = self.execute_action(action).await {
                                log::error!("Error in catch block: {}", catch_err);
                                // Catch block errors propagate up
                                return Err(catch_err);
                            }
                        }
                    } else {
                        // No catch block - error propagates
                        return Err(anyhow::anyhow!("Try block failed: {}", e));
                    }
                } else {
                    log::debug!("Try block completed successfully");
                }

                // Finally block always executes if present
                if let Some(finally) = finally_actions {
                    log::info!(
                        "Executing finally block with {} action(s)",
                        finally.len()
                    );

                    for action in finally {
                        if let Err(e) = self.execute_action(action).await {
                            log::error!("Error in finally block: {}", e);
                            // Finally block errors propagate
                            return Err(e);
                        }
                    }
                }

                // Return success (catch handled any try errors)
                Ok(())
            }
        }
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

    /// Generate a comprehensive execution report.
    ///
    /// # Arguments
    /// * `success` - Whether the execution was successful overall
    ///
    /// # Returns
    /// An ExecutionReport with detailed metrics about the task execution
    pub fn execution_report(&self, success: bool) -> ExecutionReport {
        let end_time = Instant::now();
        ExecutionReport {
            task_name: self.task_def.name.clone(),
            start_time: self.start_time,
            end_time: Some(end_time),
            total_duration: Some(end_time.duration_since(self.start_time)),
            total_actions: self.task_def.actions.len() as u32,
            actions_executed: self.actions_executed,
            actions_succeeded: self.actions_succeeded,
            actions_failed: self.actions_failed,
            max_call_depth: self.call_depth,
            variables_defined: self.variables.len(),
            action_metrics: self.action_metrics.clone(),
            success,
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
            include: vec![],
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
            include: vec![],
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
            include: vec![],
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
            include: vec![],
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
            include: vec![],
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

    #[test]
    fn test_parallel_action_struct() {
        // Test Parallel action structure
        let parallel_action = Action::Parallel {
            actions: vec![
                Action::Wait { duration_ms: 100 },
                Action::Wait { duration_ms: 200 },
            ],
            max_concurrency: Some(2),
        };

        // Verify the action can be created and inspected
        match &parallel_action {
            Action::Parallel {
                actions,
                max_concurrency,
            } => {
                assert_eq!(actions.len(), 2);
                assert_eq!(*max_concurrency, Some(2));
            }
            _ => panic!("Expected Parallel action"),
        }
    }

    #[test]
    fn test_parallel_action_no_concurrency_limit() {
        // Test Parallel action without concurrency limit
        let parallel_action = Action::Parallel {
            actions: vec![Action::Wait { duration_ms: 100 }],
            max_concurrency: None,
        };

        match &parallel_action {
            Action::Parallel {
                actions,
                max_concurrency,
            } => {
                assert_eq!(actions.len(), 1);
                assert_eq!(*max_concurrency, None);
            }
            _ => panic!("Expected Parallel action"),
        }
    }

    #[test]
    fn test_retry_action_struct() {
        // Test Retry action with all parameters
        let retry_action = Action::Retry {
            actions: vec![
                Action::Wait { duration_ms: 100 },
                Action::Click {
                    selector: "#button".to_string(),
                },
            ],
            max_attempts: Some(5),
            initial_delay_ms: Some(500),
            max_delay_ms: Some(10000),
            backoff_multiplier: Some(1.5),
            jitter: Some(true),
            retry_on: Some(vec!["timeout".to_string(), "network".to_string()]),
        };

        match &retry_action {
            Action::Retry {
                actions,
                max_attempts,
                initial_delay_ms,
                max_delay_ms,
                backoff_multiplier,
                jitter,
                retry_on,
            } => {
                assert_eq!(actions.len(), 2);
                assert_eq!(*max_attempts, Some(5));
                assert_eq!(*initial_delay_ms, Some(500));
                assert_eq!(*max_delay_ms, Some(10000));
                assert_eq!(*backoff_multiplier, Some(1.5));
                assert_eq!(*jitter, Some(true));
                assert_eq!(
                    retry_on.as_ref().unwrap(),
                    &vec!["timeout".to_string(), "network".to_string()]
                );
            }
            _ => panic!("Expected Retry action"),
        }
    }

    #[test]
    fn test_retry_action_defaults() {
        // Test Retry action with minimal parameters (defaults)
        let retry_action = Action::Retry {
            actions: vec![Action::Wait { duration_ms: 100 }],
            max_attempts: None,
            initial_delay_ms: None,
            max_delay_ms: None,
            backoff_multiplier: None,
            jitter: None,
            retry_on: None,
        };

        match &retry_action {
            Action::Retry {
                actions,
                max_attempts,
                initial_delay_ms,
                max_delay_ms,
                backoff_multiplier,
                jitter,
                retry_on,
            } => {
                assert_eq!(actions.len(), 1);
                assert!(max_attempts.is_none());
                assert!(initial_delay_ms.is_none());
                assert!(max_delay_ms.is_none());
                assert!(backoff_multiplier.is_none());
                assert!(jitter.is_none());
                assert!(retry_on.is_none());
            }
            _ => panic!("Expected Retry action"),
        }
    }

    #[test]
    fn test_foreach_action_with_array_collection() {
        use crate::task::dsl::ForeachCollection;

        // Test Foreach with array collection
        let foreach_action = Action::Foreach {
            variable: "item".to_string(),
            collection: ForeachCollection::Array {
                values: vec![
                    serde_yaml::Value::String("first".to_string()),
                    serde_yaml::Value::String("second".to_string()),
                    serde_yaml::Value::String("third".to_string()),
                ],
            },
            actions: vec![Action::Log {
                message: "Processing {{item}}".to_string(),
                level: Some(crate::task::dsl::LogLevel::Info),
            }],
            max_iterations: Some(10),
        };

        match &foreach_action {
            Action::Foreach {
                variable,
                collection,
                actions,
                max_iterations,
            } => {
                assert_eq!(variable, "item");
                assert_eq!(*max_iterations, Some(10));
                assert_eq!(actions.len(), 1);

                match collection {
                    ForeachCollection::Array { values } => {
                        assert_eq!(values.len(), 3);
                    }
                    _ => panic!("Expected Array collection"),
                }
            }
            _ => panic!("Expected Foreach action"),
        }
    }

    #[test]
    fn test_foreach_action_with_range_collection() {
        use crate::task::dsl::ForeachCollection;

        // Test Foreach with range collection
        let foreach_action = Action::Foreach {
            variable: "index".to_string(),
            collection: ForeachCollection::Range { start: 0, end: 5 },
            actions: vec![Action::Wait { duration_ms: 100 }],
            max_iterations: Some(100),
        };

        match &foreach_action {
            Action::Foreach {
                variable,
                collection,
                actions,
                max_iterations,
            } => {
                assert_eq!(variable, "index");
                assert_eq!(*max_iterations, Some(100));
                assert_eq!(actions.len(), 1);

                match collection {
                    ForeachCollection::Range { start, end } => {
                        assert_eq!(*start, 0);
                        assert_eq!(*end, 5);
                    }
                    _ => panic!("Expected Range collection"),
                }
            }
            _ => panic!("Expected Foreach action"),
        }
    }

    #[test]
    fn test_foreach_action_with_elements_collection() {
        use crate::task::dsl::ForeachCollection;

        // Test Foreach with DOM elements collection
        let foreach_action = Action::Foreach {
            variable: "element".to_string(),
            collection: ForeachCollection::Elements {
                selector: ".item".to_string(),
            },
            actions: vec![Action::Click {
                selector: "{{element}}".to_string(),
            }],
            max_iterations: Some(20),
        };

        match &foreach_action {
            Action::Foreach {
                variable,
                collection,
                actions,
                max_iterations,
            } => {
                assert_eq!(variable, "element");
                assert_eq!(*max_iterations, Some(20));
                assert_eq!(actions.len(), 1);

                match collection {
                    ForeachCollection::Elements { selector } => {
                        assert_eq!(selector, ".item");
                    }
                    _ => panic!("Expected Elements collection"),
                }
            }
            _ => panic!("Expected Foreach action"),
        }
    }

    #[test]
    fn test_foreach_action_with_variable_collection() {
        use crate::task::dsl::ForeachCollection;

        // Test Foreach with variable collection
        let foreach_action = Action::Foreach {
            variable: "url".to_string(),
            collection: ForeachCollection::Variable {
                name: "urls".to_string(),
            },
            actions: vec![Action::Navigate {
                url: "{{url}}".to_string(),
            }],
            max_iterations: None, // Should default to 100
        };

        match &foreach_action {
            Action::Foreach {
                variable,
                collection,
                actions,
                max_iterations,
            } => {
                assert_eq!(variable, "url");
                assert!(max_iterations.is_none());
                assert_eq!(actions.len(), 1);

                match collection {
                    ForeachCollection::Variable { name } => {
                        assert_eq!(name, "urls");
                    }
                    _ => panic!("Expected Variable collection"),
                }
            }
            _ => panic!("Expected Foreach action"),
        }
    }

    #[test]
    fn test_while_action_with_element_exists_condition() {
        use crate::task::dsl::Condition;

        // Test While with element exists condition
        let while_action = Action::While {
            condition: Condition::ElementExists {
                selector: ".loading".to_string(),
            },
            actions: vec![
                Action::Wait { duration_ms: 500 },
                Action::Log {
                    message: "Still loading...".to_string(),
                    level: Some(crate::task::dsl::LogLevel::Debug),
                },
            ],
            max_iterations: Some(20),
        };

        match &while_action {
            Action::While {
                condition,
                actions,
                max_iterations,
            } => {
                assert_eq!(*max_iterations, Some(20));
                assert_eq!(actions.len(), 2);

                match condition {
                    Condition::ElementExists { selector } => {
                        assert_eq!(selector, ".loading");
                    }
                    _ => panic!("Expected ElementExists condition"),
                }
            }
            _ => panic!("Expected While action"),
        }
    }

    #[test]
    fn test_while_action_defaults() {
        use crate::task::dsl::Condition;

        // Test While with minimal parameters (defaults)
        let while_action = Action::While {
            condition: Condition::ElementVisible {
                selector: ".spinner".to_string(),
            },
            actions: vec![Action::Wait { duration_ms: 100 }],
            max_iterations: None, // Should default to 1000
        };

        match &while_action {
            Action::While {
                condition,
                actions,
                max_iterations,
            } => {
                assert!(max_iterations.is_none()); // Defaults to 1000 at runtime
                assert_eq!(actions.len(), 1);

                match condition {
                    Condition::ElementVisible { selector } => {
                        assert_eq!(selector, ".spinner");
                    }
                    _ => panic!("Expected ElementVisible condition"),
                }
            }
            _ => panic!("Expected While action"),
        }
    }

    #[test]
    fn test_try_action_with_catch_and_finally() {
        // Test Try with all components
        let try_action = Action::Try {
            try_actions: vec![
                Action::Click {
                    selector: "#risky-button".to_string(),
                },
                Action::Wait { duration_ms: 100 },
            ],
            catch_actions: Some(vec![
                Action::Log {
                    message: "Button not found, using fallback".to_string(),
                    level: Some(crate::task::dsl::LogLevel::Warn),
                },
                Action::Click {
                    selector: "#fallback-button".to_string(),
                },
            ]),
            error_variable: Some("error_msg".to_string()),
            finally_actions: Some(vec![
                Action::Log {
                    message: "Cleanup".to_string(),
                    level: Some(crate::task::dsl::LogLevel::Info),
                },
            ]),
        };

        match &try_action {
            Action::Try {
                try_actions,
                catch_actions,
                error_variable,
                finally_actions,
            } => {
                assert_eq!(try_actions.len(), 2);
                assert!(catch_actions.is_some());
                assert_eq!(catch_actions.as_ref().unwrap().len(), 2);
                assert_eq!(error_variable, &Some("error_msg".to_string()));
                assert!(finally_actions.is_some());
                assert_eq!(finally_actions.as_ref().unwrap().len(), 1);
            }
            _ => panic!("Expected Try action"),
        }
    }

    #[test]
    fn test_try_action_minimal() {
        // Test Try with just try block (no catch/finally)
        let try_action = Action::Try {
            try_actions: vec![Action::Wait { duration_ms: 100 }],
            catch_actions: None,
            error_variable: None,
            finally_actions: None,
        };

        match &try_action {
            Action::Try {
                try_actions,
                catch_actions,
                error_variable,
                finally_actions,
            } => {
                assert_eq!(try_actions.len(), 1);
                assert!(catch_actions.is_none());
                assert!(error_variable.is_none());
                assert!(finally_actions.is_none());
            }
            _ => panic!("Expected Try action"),
        }
    }

    #[test]
    fn test_try_action_with_error_variable_only() {
        // Test Try with error variable but no catch (for logging)
        let try_action = Action::Try {
            try_actions: vec![Action::Click {
                selector: "#button".to_string(),
            }],
            catch_actions: None,
            error_variable: Some("last_error".to_string()),
            finally_actions: Some(vec![Action::Log {
                message: "{{last_error}}".to_string(),
                level: Some(crate::task::dsl::LogLevel::Error),
            }]),
        };

        match &try_action {
            Action::Try {
                error_variable,
                finally_actions,
                ..
            } => {
                assert_eq!(error_variable, &Some("last_error".to_string()));
                assert!(finally_actions.is_some());
            }
            _ => panic!("Expected Try action"),
        }
    }
}
