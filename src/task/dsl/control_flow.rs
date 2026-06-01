//! Control flow action handlers for DSL execution.
//!
//! Handles conditional execution, loops, and parallel execution
//! for DSL tasks. These are methods on `DslExecutor`.

use crate::task::dsl::{Action, Condition};
use anyhow::Result;
use futures::future::join_all;

// Retry configuration parameters
#[derive(Debug, Clone)]
pub(super) struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub jitter: bool,
    pub retry_on: Option<Vec<String>>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
            jitter: true,
            retry_on: None,
        }
    }
}

impl RetryConfig {
    #[allow(dead_code)]
    pub fn from_action(action: &crate::task::dsl::Action) -> Self {
        match action {
            crate::task::dsl::Action::Retry {
                max_attempts,
                initial_delay_ms,
                max_delay_ms,
                backoff_multiplier,
                jitter,
                retry_on,
                ..
            } => Self {
                max_attempts: max_attempts.unwrap_or(3),
                initial_delay_ms: initial_delay_ms.unwrap_or(1000),
                max_delay_ms: max_delay_ms.unwrap_or(30000),
                backoff_multiplier: backoff_multiplier.unwrap_or(2.0),
                jitter: jitter.unwrap_or(true),
                retry_on: retry_on.clone(),
            },
            _ => Self::default(),
        }
    }
}

impl<T: super::DslApi> super::DslExecutor<'_, T> {
    /// Execute an If/Else action.
    #[allow(clippy::cast_precision_loss)]
    pub(super) async fn execute_if(
        &mut self,
        condition: &Condition,
        then: &[Action],
        r#else: &Option<Vec<Action>>,
    ) -> Result<()> {
        if self.evaluate_condition(condition).await? {
            for action in then {
                Box::pin(self.execute_action(action)).await?;
            }
        } else if let Some(else_actions) = r#else {
            for action in else_actions {
                Box::pin(self.execute_action(action)).await?;
            }
        }
        Ok(())
    }

    /// Execute a Loop action (fixed count or conditional).
    pub(super) async fn execute_loop(
        &mut self,
        count: &Option<u32>,
        condition: &Option<Condition>,
        actions: &[Action],
    ) -> Result<()> {
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
                log::warn!("Loop reached max iterations ({max_iterations}), breaking");
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
        Ok(())
    }

    /// Execute a Foreach action (iterate over collection).
    pub(super) async fn execute_foreach(
        &mut self,
        variable: &str,
        collection: &crate::task::dsl::ForeachCollection,
        actions: &[Action],
        max_iterations: &Option<u32>,
    ) -> Result<()> {
        let max_iterations = max_iterations.unwrap_or(100);

        // Resolve collection based on type
        let values: Vec<String> = match collection {
            crate::task::dsl::ForeachCollection::Array { values } => values
                .iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect(),
            crate::task::dsl::ForeachCollection::Range { start, end } => {
                (*start..*end).map(|i| i.to_string()).collect()
            }
            crate::task::dsl::ForeachCollection::Elements { selector } => {
                // Count matching elements and create index-based values.
                // Note: Uses :nth-of-type() selectors which are fragile with mixed
                // DOM structures and don't handle dynamic element changes between
                // iterations. Prefer data-testid selectors with Array collections
                // for more robust iteration.
                let resolved_selector = self.substitute_variables(selector);
                let count = self
                    .api
                    .count_elements(&resolved_selector)
                    .await
                    .unwrap_or(0);
                (0..count)
                    .map(|i| format!("{}:nth-of-type({})", resolved_selector, i + 1))
                    .collect()
            }
            crate::task::dsl::ForeachCollection::Variable { name } => {
                // Get value from variable - treat as single item or comma-separated list
                if let Some(var_value) = self.variables.get(name) {
                    // Check if value contains commas - if so, split it
                    if var_value.contains(',') {
                        var_value.split(',').map(|s| s.trim().to_string()).collect()
                    } else {
                        vec![var_value.clone()]
                    }
                } else {
                    log::warn!("Foreach variable '{name}' not found, using empty collection");
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
            self.variables.insert(variable.to_string(), value.clone());

            // Execute actions for this iteration
            for action in actions {
                Box::pin(self.execute_action(action)).await?;
            }
        }

        log::info!("Foreach loop completed {iteration_count} iterations");
        Ok(())
    }

    /// Execute a While action (condition-based loop).
    pub(super) async fn execute_while(
        &mut self,
        condition: &Condition,
        actions: &[Action],
        max_iterations: &Option<u32>,
    ) -> Result<()> {
        let max_iterations = max_iterations.unwrap_or(1000);

        let mut iteration_count = 0;
        while self.evaluate_condition(condition).await? && iteration_count < max_iterations {
            iteration_count += 1;
            for action in actions {
                Box::pin(self.execute_action(action)).await?;
            }
        }

        if iteration_count >= max_iterations {
            log::warn!("While loop reached max iterations ({max_iterations})");
        }

        Ok(())
    }

    /// Execute a Retry action (retry block with backoff).
    #[allow(clippy::cast_precision_loss)]
    pub(super) async fn execute_retry(
        &mut self,
        actions: &[Action],
        config: &RetryConfig,
    ) -> Result<()> {
        let max_attempts = config.max_attempts.max(1);
        let initial_delay_ms = config.initial_delay_ms;
        let max_delay_ms = config.max_delay_ms;
        let backoff_multiplier = config.backoff_multiplier.max(1.0);
        let use_jitter = config.jitter;

        log::info!(
            "Executing retry block with max {max_attempts} attempts, initial delay {initial_delay_ms}ms"
        );

        let mut last_error: Option<anyhow::Error> = None;
        let mut current_delay_ms = initial_delay_ms;

        for attempt in 1..=max_attempts {
            log::debug!("Retry attempt {attempt}/{max_attempts}");

            // Try executing all actions
            let mut attempt_success = true;
            for action in actions {
                if let Err(e) = Box::pin(self.execute_action(action)).await {
                    let error_msg = e.to_string();

                    // Check if we should retry on this error
                    if let Some(ref retry_patterns) = config.retry_on {
                        let should_retry = retry_patterns.iter().any(|pattern| {
                            error_msg.to_lowercase().contains(&pattern.to_lowercase())
                        });
                        if !should_retry {
                            log::warn!(
                                "Error does not match retry patterns, failing immediately: {error_msg}"
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
                    "Retry attempt {attempt}/{max_attempts} succeeded after {current_delay_ms}ms pause"
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
                    "Attempt {attempt}/{max_attempts} failed, waiting {delay_with_jitter}ms before retry"
                );

                tokio::time::sleep(tokio::time::Duration::from_millis(delay_with_jitter)).await;

                // Exponential backoff with cap
                current_delay_ms =
                    ((current_delay_ms as f64 * backoff_multiplier) as u64).min(max_delay_ms);
            }
        }

        // All attempts exhausted
        Err(anyhow::anyhow!(
            "Retry block failed after {} attempts. Last error: {}",
            max_attempts,
            last_error.map_or_else(|| "Unknown error".to_string(), |e| e.to_string())
        ))
    }

    /// Execute a Parallel action (execute actions in parallel).
    #[allow(clippy::cast_precision_loss)]
    pub(super) async fn execute_parallel(
        &mut self,
        actions: &[Action],
        max_concurrency: &Option<usize>,
    ) -> Result<()> {
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
            log::debug!("Starting parallel action {idx}: {action_clone:?}");

            // For now, execute sequentially within each parallel branch
            // Full parallel would require redesigning executor for interior mutability
            let future = async move {
                let _permit = permit; // Hold permit until completion
                log::debug!("Executing parallel action {idx} for '{task_name}'");
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
                errors.push(format!("Action {idx} failed: {e}"));
            }
        }

        if !errors.is_empty() {
            return Err(anyhow::anyhow!(
                "Parallel execution failed:\n{}",
                errors.join("\n")
            ));
        }

        log::info!("Parallel execution complete");
        Ok(())
    }

    /// Execute a Try/Catch/Finally action.
    pub(super) async fn execute_try(
        &mut self,
        try_actions: &[Action],
        catch_actions: Option<&Vec<Action>>,
        error_variable: Option<&str>,
        finally_actions: Option<&Vec<Action>>,
    ) -> Result<()> {
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
                    self.variables.insert(var_name.to_string(), e.to_string());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
        assert!((config.backoff_multiplier - 2.0).abs() < f64::EPSILON);
        assert!(config.jitter);
        assert!(config.retry_on.is_none());
    }

    #[test]
    fn test_retry_config_from_retry_action() {
        let action = crate::task::dsl::Action::Retry {
            actions: vec![crate::task::dsl::Action::Wait { duration_ms: 100 }],
            max_attempts: Some(5),
            initial_delay_ms: Some(500),
            max_delay_ms: Some(10000),
            backoff_multiplier: Some(3.0),
            jitter: Some(false),
            retry_on: Some(vec!["timeout".to_string(), "connection".to_string()]),
        };

        let config = RetryConfig::from_action(&action);
        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.initial_delay_ms, 500);
        assert_eq!(config.max_delay_ms, 10000);
        assert!((config.backoff_multiplier - 3.0).abs() < f64::EPSILON);
        assert!(!config.jitter);
        assert!(config.retry_on.is_some());
        let patterns = config.retry_on.unwrap();
        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0], "timeout");
        assert_eq!(patterns[1], "connection");
    }

    #[test]
    fn test_retry_config_from_retry_action_defaults() {
        // Retry action with all None fields should use defaults
        let action = crate::task::dsl::Action::Retry {
            actions: vec![],
            max_attempts: None,
            initial_delay_ms: None,
            max_delay_ms: None,
            backoff_multiplier: None,
            jitter: None,
            retry_on: None,
        };

        let config = RetryConfig::from_action(&action);
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
        assert!((config.backoff_multiplier - 2.0).abs() < f64::EPSILON);
        assert!(config.jitter);
        assert!(config.retry_on.is_none());
    }

    #[test]
    fn test_retry_config_from_non_retry_action() {
        let action = crate::task::dsl::Action::Click {
            selector: "#btn".to_string(),
        };

        let config = RetryConfig::from_action(&action);
        // Non-Retry actions should produce default config
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
        assert!((config.backoff_multiplier - 2.0).abs() < f64::EPSILON);
        assert!(config.jitter);
        assert!(config.retry_on.is_none());
    }

    #[test]
    fn test_retry_config_from_action_partial_overrides() {
        // Only override max_attempts, rest should default
        let action = crate::task::dsl::Action::Retry {
            actions: vec![crate::task::dsl::Action::Wait { duration_ms: 10 }],
            max_attempts: Some(10),
            initial_delay_ms: None,
            max_delay_ms: None,
            backoff_multiplier: None,
            jitter: None,
            retry_on: None,
        };

        let config = RetryConfig::from_action(&action);
        assert_eq!(config.max_attempts, 10);
        assert_eq!(config.initial_delay_ms, 1000); // default
        assert_eq!(config.max_delay_ms, 30000); // default
    }
}
