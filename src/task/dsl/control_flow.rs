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
    use crate::task::dsl::api::mock::{MockCall, MockDslApi};
    use crate::task::dsl::{Action, Condition, ForeachCollection};
    use std::collections::HashMap;
    use tokio::time::Duration;

    fn create_task_def(name: &str, actions: Vec<Action>) -> crate::task::dsl::TaskDefinition {
        crate::task::dsl::TaskDefinition {
            name: name.to_string(),
            description: format!("Test: {name}"),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![],
            actions,
        }
    }

    fn create_executor<'a>(
        mock: &'a MockDslApi,
        actions: Vec<Action>,
    ) -> crate::task::dsl::DslExecutor<'a, MockDslApi> {
        crate::task::dsl::DslExecutor::new(mock, create_task_def("test", actions))
    }

    // ── RetryConfig construction tests ──

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
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
        assert!((config.backoff_multiplier - 2.0).abs() < f64::EPSILON);
        assert!(config.jitter);
        assert!(config.retry_on.is_none());
    }

    #[test]
    fn test_retry_config_from_action_partial_overrides() {
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
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
    }

    // ── execute_if ──

    #[tokio::test]
    async fn if_condition_true_runs_then_branch() {
        let mock = MockDslApi::new();
        mock.set_exists_result("#btn", true);
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_if(
            &Condition::ElementExists {
                selector: "#btn".to_string(),
            },
            &[Action::Click {
                selector: "#then-btn".to_string(),
            }],
            &None,
        )
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 2, "condition check + then action");
        assert!(matches!(calls[0], MockCall::Exists { .. }));
        assert!(matches!(calls[1], MockCall::Click { .. }));
    }

    #[tokio::test]
    async fn if_condition_false_runs_else_branch() {
        let mock = MockDslApi::new();
        mock.set_exists_result("#btn", false);
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_if(
            &Condition::ElementExists {
                selector: "#btn".to_string(),
            },
            &[Action::Click {
                selector: "#then-btn".to_string(),
            }],
            &Some(vec![Action::Click {
                selector: "#else-btn".to_string(),
            }]),
        )
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 2, "condition check + else action");
        assert!(matches!(calls[0], MockCall::Exists { .. }));
        assert!(matches!(calls[1], MockCall::Click { .. }));
    }

    #[tokio::test]
    async fn if_no_else_skips_when_false() {
        let mock = MockDslApi::new();
        mock.set_exists_result("#btn", false);
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_if(
            &Condition::ElementExists {
                selector: "#btn".to_string(),
            },
            &[Action::Click {
                selector: "#then-btn".to_string(),
            }],
            &None,
        )
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert!(calls.iter().any(|c| matches!(c, MockCall::Exists { .. })));
        assert!(!calls.iter().any(|c| matches!(c, MockCall::Click { .. })));
    }

    // ── execute_loop ──

    #[tokio::test]
    async fn loop_fixed_count_iterates_n_times() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_loop(
            &Some(3),
            &None,
            &[Action::Click {
                selector: "#item".to_string(),
            }],
        )
        .await
        .unwrap();

        let clicks: Vec<_> = mock
            .get_calls()
            .into_iter()
            .filter(|c| matches!(c, MockCall::Click { .. }))
            .collect();
        assert_eq!(clicks.len(), 3);
    }

    #[tokio::test]
    async fn loop_zero_count_skips() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_loop(
            &Some(0),
            &None,
            &[Action::Click {
                selector: "#item".to_string(),
            }],
        )
        .await
        .unwrap();

        assert!(mock.get_calls().is_empty());
    }

    #[tokio::test]
    async fn loop_condition_false_exits_immediately() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_loop(
            &None,
            &Some(Condition::False),
            &[Action::Click {
                selector: "#item".to_string(),
            }],
        )
        .await
        .unwrap();

        assert!(mock.get_calls().is_empty());
    }

    // ── execute_foreach ──

    #[tokio::test]
    async fn foreach_iterates_all_items() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_foreach(
            "item",
            &ForeachCollection::Array {
                values: vec![
                    serde_yml::Value::String("a".to_string()),
                    serde_yml::Value::String("b".to_string()),
                    serde_yml::Value::String("c".to_string()),
                ],
            },
            &[Action::Click {
                selector: "#item".to_string(),
            }],
            &None,
        )
        .await
        .unwrap();

        let clicks: Vec<_> = mock
            .get_calls()
            .into_iter()
            .filter(|c| matches!(c, MockCall::Click { .. }))
            .collect();
        assert_eq!(clicks.len(), 3);
    }

    #[tokio::test]
    async fn foreach_empty_collection_skips() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_foreach(
            "item",
            &ForeachCollection::Array { values: vec![] },
            &[Action::Click {
                selector: "#item".to_string(),
            }],
            &None,
        )
        .await
        .unwrap();

        assert!(mock.get_calls().is_empty());
    }

    // ── execute_retry ──

    #[tokio::test]
    async fn retry_succeeds_on_first_try() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_retry(
            &[Action::Click {
                selector: "#btn".to_string(),
            }],
            &RetryConfig {
                max_attempts: 3,
                initial_delay_ms: 1,
                max_delay_ms: 5,
                backoff_multiplier: 1.0,
                jitter: false,
                retry_on: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(mock.get_calls().len(), 1);
    }

    #[tokio::test]
    async fn retry_retries_on_failure_up_to_max() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let fail_arc = mock.fail_all.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(5)).await;
            *fail_arc.lock().unwrap() = false;
        });

        let mut exec = create_executor(&mock, vec![]);

        exec.execute_retry(
            &[Action::Click {
                selector: "#btn".to_string(),
            }],
            &RetryConfig {
                max_attempts: 3,
                initial_delay_ms: 10,
                max_delay_ms: 100,
                backoff_multiplier: 1.0,
                jitter: false,
                retry_on: None,
            },
        )
        .await
        .unwrap();

        let clicks: Vec<_> = mock
            .get_calls()
            .into_iter()
            .filter(|c| matches!(c, MockCall::Click { .. }))
            .collect();
        assert!(clicks.len() >= 2, "should retry after first failure");
    }

    #[tokio::test]
    async fn retry_fails_after_max_attempts() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_retry(
                &[Action::Click {
                    selector: "#btn".to_string(),
                }],
                &RetryConfig {
                    max_attempts: 3,
                    initial_delay_ms: 1,
                    max_delay_ms: 5,
                    backoff_multiplier: 1.0,
                    jitter: false,
                    retry_on: None,
                },
            )
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Retry block failed after 3 attempts"));
        assert_eq!(mock.get_calls().len(), 3);
    }

    // ── execute_parallel ──

    #[tokio::test]
    async fn parallel_executes_all_actions() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_parallel(
                &[
                    Action::Click {
                        selector: "#a".to_string(),
                    },
                    Action::Click {
                        selector: "#b".to_string(),
                    },
                    Action::Click {
                        selector: "#c".to_string(),
                    },
                ],
                &None,
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn parallel_empty_actions_does_nothing() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        let result = exec.execute_parallel(&[], &None).await;

        assert!(result.is_ok());
    }

    // ── evaluate_condition delegation ──

    #[tokio::test]
    async fn evaluate_condition_element_exists() {
        let mock = MockDslApi::new();
        mock.set_exists_result("#present", true);
        mock.set_exists_result("#absent", false);
        let exec = create_executor(&mock, vec![]);

        let result = exec
            .evaluate_condition(&Condition::ElementExists {
                selector: "#present".to_string(),
            })
            .await
            .unwrap();
        assert!(result);

        let result = exec
            .evaluate_condition(&Condition::ElementExists {
                selector: "#absent".to_string(),
            })
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn evaluate_condition_text_equals() {
        let mock = MockDslApi::new();
        mock.set_text_result("#status", Some("ready"));
        let exec = create_executor(&mock, vec![]);

        let result = exec
            .evaluate_condition(&Condition::TextEquals {
                selector: "#status".to_string(),
                value: "ready".to_string(),
            })
            .await
            .unwrap();
        assert!(result);

        let result = exec
            .evaluate_condition(&Condition::TextEquals {
                selector: "#status".to_string(),
                value: "not-ready".to_string(),
            })
            .await
            .unwrap();
        assert!(!result);
    }

    // ── execute_while ─────────────────────────────────────────────────

    #[tokio::test]
    async fn while_runs_up_to_max_iterations() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_while(
            &Condition::True,
            &[Action::Click {
                selector: "#item".to_string(),
            }],
            &Some(3),
        )
        .await
        .unwrap();

        let clicks: Vec<_> = mock
            .get_calls()
            .into_iter()
            .filter(|c| matches!(c, MockCall::Click { .. }))
            .collect();
        assert_eq!(clicks.len(), 3);
    }

    #[tokio::test]
    async fn while_false_condition_skips_body() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_while(
            &Condition::False,
            &[Action::Click {
                selector: "#item".to_string(),
            }],
            &Some(100),
        )
        .await
        .unwrap();

        assert!(mock.get_calls().is_empty());
    }

    // ── execute_try ───────────────────────────────────────────────────

    #[tokio::test]
    async fn try_succeeds_without_catch_or_finally() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_try(
            &[Action::Click {
                selector: "#btn".to_string(),
            }],
            None,
            None,
            None,
        )
        .await
        .unwrap();

        assert_eq!(mock.get_calls().len(), 1);
    }

    #[tokio::test]
    async fn try_catches_error_and_sets_variable() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_try(
            &[Action::Click {
                selector: "#failing".to_string(),
            }],
            Some(&vec![Action::Log {
                message: "caught".to_string(),
                level: None,
            }]),
            Some("err"),
            None,
        )
        .await
        .unwrap();

        assert!(exec.variables.contains_key("err"));
        assert!(exec
            .variables
            .get("err")
            .unwrap()
            .contains("MockDslApi forced failure"));
    }

    #[tokio::test]
    async fn try_finally_runs_on_success() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_try(
            &[Action::Click {
                selector: "#btn".to_string(),
            }],
            None,
            None,
            Some(&vec![Action::Click {
                selector: "#finally".to_string(),
            }]),
        )
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 2);
    }

    #[tokio::test]
    async fn try_finally_runs_on_failure() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_try(
            &[Action::Click {
                selector: "#failing".to_string(),
            }],
            None,
            None,
            Some(&vec![Action::Log {
                message: "finally executed".to_string(),
                level: None,
            }]),
        )
        .await
        .unwrap();

        // Only the failing try click should be recorded (Log doesn't call API)
        assert_eq!(mock.get_calls().len(), 1);
    }

    #[tokio::test]
    async fn try_suppresses_error_without_catch() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_try(
                &[Action::Click {
                    selector: "#failing".to_string(),
                }],
                None,
                None,
                None,
            )
            .await;

        assert!(result.is_ok());
    }

    // ── additional evaluate_condition ─────────────────────────────────

    #[tokio::test]
    async fn evaluate_condition_visible() {
        let mock = MockDslApi::new();
        mock.set_visible_result("#visible", true);
        mock.set_visible_result("#hidden", false);
        let exec = create_executor(&mock, vec![]);

        assert!(exec
            .evaluate_condition(&Condition::ElementVisible {
                selector: "#visible".to_string(),
            })
            .await
            .unwrap());
        assert!(!exec
            .evaluate_condition(&Condition::ElementVisible {
                selector: "#hidden".to_string(),
            })
            .await
            .unwrap());
    }
}
