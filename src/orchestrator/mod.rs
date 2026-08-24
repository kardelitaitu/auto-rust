//! Task orchestration and execution coordination module.
//!
//! The orchestrator manages:
//! - Parallel execution of task groups across sessions
//! - Global concurrency control via semaphores
//! - Single-attempt execution with fail-fast behavior
//! - Error handling and load balancing
//! - Resource allocation and distribution
//!
//! # Module structure
//!
//! - `health.rs` — `format_duration()`, `broadcast_execution_count()`, `should_mark_session_unhealthy()`
//! - `guards.rs` — `GlobalExecutionSlot`, `SessionExecutionGuard`, `acquire_global_execution_slot()`
//! - `retry.rs` — `TaskAttemptFailure`, `execute_task_with_retry()`
//! - `execution.rs` — `execute_group_with_cancel()`, `execute_task_on_session()`
//! - `test_utils.rs` — shared test helpers

mod execution;
mod guards;
mod health;
mod retry;

#[cfg(test)]
mod test_utils;

use crate::cli::CliTaskDefinition;
use crate::config::Config;
use crate::error::Result;
use crate::metrics::MetricsCollector;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

/// Central coordinator for task execution across multiple browser sessions.
///
/// The `Orchestrator` manages:
/// - Global concurrency limits across all sessions
/// - Session allocation and task distribution
/// - Retry logic with exponential backoff
/// - Error handling and load balancing
/// - Resource allocation and distribution
///
/// # Examples
///
/// ```no_run
/// # use auto::orchestrator::Orchestrator;
/// # use auto::config::Config;
/// # async fn example(config: Config) {
/// let mut orchestrator = Orchestrator::new(config);
/// // Execute task groups across sessions
/// # }
/// ```
pub struct Orchestrator {
    /// Configuration settings for orchestration behavior (shared via Arc to avoid cloning per task)
    config: Arc<Config>,
    /// Global counter of currently active tasks across all sessions
    global_active_tasks: Arc<AtomicUsize>,
    /// Semaphore limiting total concurrent tasks across all sessions
    global_semaphore: Arc<Semaphore>,
}

impl Orchestrator {
    /// Creates a new orchestrator with the given configuration.
    ///
    /// Initializes global concurrency controls and prepares for task execution.
    /// The orchestrator respects the configured `max_global_concurrency` limit
    /// to prevent resource exhaustion.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration settings for orchestration behavior
    ///
    /// # Returns
    ///
    /// A new `Orchestrator` instance ready for task execution.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use auto::orchestrator::Orchestrator;
    /// # use auto::config::Config;
    /// # let config: Config = todo!();
    /// let orchestrator = Orchestrator::new(config);
    /// ```
    #[must_use]
    pub fn new(config: Config) -> Self {
        let max_concurrency = config.orchestrator.max_global_concurrency;
        Self {
            global_active_tasks: Arc::new(AtomicUsize::new(0)),
            global_semaphore: Arc::new(Semaphore::new(max_concurrency)),
            config: Arc::new(config),
        }
    }

    /// Executes a group of tasks across available browser sessions.
    ///
    /// Tasks within a group run in parallel across different sessions,
    /// respecting global concurrency limits. Each task is broadcast to all
    /// healthy sessions, with partial failure allowed if at least one session succeeds.
    ///
    /// # Execution Model
    ///
    /// - Tasks run in parallel across sessions
    /// - Global semaphore limits total concurrent task-session executions
    /// - Retry logic with exponential backoff
    /// - Health scoring and session selection
    ///
    /// # Arguments
    ///
    /// * `group` - Slice of task definitions to execute
    /// * `sessions` - Available browser sessions for task execution
    /// * `metrics` - Metrics collector for tracking execution statistics
    ///
    /// # Returns
    ///
    /// `Ok(())` if the group completes successfully (allowing partial failures)
    /// `Err(OrchestratorError)` if all sessions fail for all tasks
    pub async fn execute_group(
        &mut self,
        group: &[CliTaskDefinition],
        sessions: &[crate::session::Session],
        metrics: Arc<MetricsCollector>,
    ) -> Result<()> {
        execution::execute_group_with_cancel(
            Arc::clone(&self.config),
            &self.global_active_tasks,
            &self.global_semaphore,
            group,
            sessions,
            metrics,
            CancellationToken::new(),
        )
        .await
    }

    /// Executes a group of tasks with an external cancellation token.
    pub async fn execute_group_with_cancel(
        &mut self,
        group: &[CliTaskDefinition],
        sessions: &[crate::session::Session],
        metrics: Arc<MetricsCollector>,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        execution::execute_group_with_cancel(
            Arc::clone(&self.config),
            &self.global_active_tasks,
            &self.global_semaphore,
            group,
            sessions,
            metrics,
            cancel_token,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::CliTaskDefinition;
    use crate::config::{
        Config, OrchestratorConfig, TaskDiscoveryConfig, TracingConfig, TwitterActivityConfig,
    };
    use crate::error::{OrchestratorError, SessionError, TaskError};
    use crate::session::DurationMs;
    use crate::session::Session;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use tokio_util::sync::CancellationToken;

    use super::test_utils::{connect_test_session, create_test_config};

    #[test]
    fn test_orchestrator_new_initialization() {
        let config = create_test_config();
        let orchestrator = Orchestrator::new(config);

        assert_eq!(orchestrator.config.orchestrator.max_global_concurrency, 10);
        assert_eq!(orchestrator.global_active_tasks.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_orchestrator_new_with_different_configs() {
        let config = Config {
            orchestrator: OrchestratorConfig {
                max_global_concurrency: 15,
                group_timeout_ms: DurationMs::new_const(10000),
                task_timeout_ms: DurationMs::new_const(60000),
                task_stagger_delay_ms: 200,
                worker_wait_timeout_ms: DurationMs::new_const(10000),
                retry_delay_ms: DurationMs::new_const(2000),
                max_retries: 3,
            },
            browser: Default::default(),
            tracing: TracingConfig::default(),
            twitter_activity: TwitterActivityConfig::default(),
            task_discovery: TaskDiscoveryConfig::default(),
        };

        let orchestrator = Orchestrator::new(config.clone());
        assert_eq!(orchestrator.config.orchestrator.max_global_concurrency, 15);
        assert_eq!(
            orchestrator.config.orchestrator.group_timeout_ms.get(),
            10000
        );
        assert_eq!(
            orchestrator.config.orchestrator.task_timeout_ms.get(),
            60000
        );

        let config2 = Config {
            orchestrator: OrchestratorConfig::default(),
            browser: Default::default(),
            tracing: TracingConfig::default(),
            twitter_activity: TwitterActivityConfig::default(),
            task_discovery: TaskDiscoveryConfig::default(),
        };
        let orchestrator2 = Orchestrator::new(config2);
        assert_eq!(orchestrator2.config.orchestrator.max_global_concurrency, 5);
    }

    #[tokio::test]
    async fn test_execute_group_with_empty_sessions_returns_error() {
        let config = create_test_config();
        let mut orchestrator = Orchestrator::new(config);
        let sessions: Vec<Session> = vec![];
        let metrics = Arc::new(crate::metrics::MetricsCollector::new(100));
        let task_def = CliTaskDefinition {
            name: "test_task".to_string(),
            payload: Default::default(),
        };

        let result = orchestrator
            .execute_group(&[task_def], &sessions, metrics)
            .await;

        assert!(result.is_err());
        match result {
            Err(OrchestratorError::Session(SessionError::InitializationFailed(msg))) => {
                assert!(msg.contains("No active sessions"));
            }
            _ => panic!("Expected Session::InitializationFailed error"),
        }
    }

    #[tokio::test]
    async fn test_execute_group_with_empty_task_group_returns_ok() {
        let tasks: Vec<CliTaskDefinition> = vec![];
        assert!(tasks.is_empty());
    }

    #[tokio::test]
    async fn test_group_timeout_with_insufficient_time() {
        let mut config = create_test_config();
        config.orchestrator.group_timeout_ms = DurationMs::new_const(1);
        config.orchestrator.task_stagger_delay_ms = 0;

        let mut orchestrator = Orchestrator::new(config);
        let metrics = Arc::new(crate::metrics::MetricsCollector::new(100));

        let task_def = CliTaskDefinition {
            name: "slow_task".to_string(),
            payload: Default::default(),
        };

        let sessions: Vec<Session> = vec![];

        let result = orchestrator
            .execute_group(&[task_def], &sessions, metrics)
            .await;

        assert!(result.is_err());
        match result {
            Err(OrchestratorError::Session(SessionError::InitializationFailed(msg))) => {
                assert!(msg.contains("No active sessions"));
            }
            _ => panic!("Expected Session::InitializationFailed error for empty sessions"),
        }
    }

    #[tokio::test]
    async fn test_execute_group_with_cancel_times_out_on_stagger_delay() -> anyhow::Result<()> {
        let Some(session) = connect_test_session().await? else {
            return Ok(());
        };

        let mut config = create_test_config();
        config.orchestrator.group_timeout_ms = DurationMs::new_const(25);
        config.orchestrator.task_stagger_delay_ms = 250;

        let mut orchestrator = Orchestrator::new(config.clone());
        let metrics = Arc::new(crate::metrics::MetricsCollector::new(10));
        let tasks = vec![CliTaskDefinition {
            name: "pageview".to_string(),
            payload: Default::default(),
        }];
        let sessions = vec![session];

        let result = orchestrator
            .execute_group_with_cancel(&tasks, &sessions, metrics, CancellationToken::new())
            .await;

        match result {
            Err(OrchestratorError::Task(TaskError::Timeout {
                task_name,
                timeout_ms,
            })) => {
                assert_eq!(task_name, "task group");
                assert_eq!(timeout_ms, 25);
            }
            other => panic!("expected group timeout, got {:?}", other),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_execute_group_with_cancel_returns_cancelled_on_shutdown() -> anyhow::Result<()> {
        let Some(session) = connect_test_session().await? else {
            return Ok(());
        };

        let mut config = create_test_config();
        config.orchestrator.group_timeout_ms = DurationMs::new_const(5000);
        config.orchestrator.task_stagger_delay_ms = 250;

        let mut orchestrator = Orchestrator::new(config.clone());
        let metrics = Arc::new(crate::metrics::MetricsCollector::new(10));
        let tasks = vec![CliTaskDefinition {
            name: "pageview".to_string(),
            payload: Default::default(),
        }];
        let sessions = vec![session];
        let cancel_token = CancellationToken::new();
        cancel_token.cancel();

        let result = orchestrator
            .execute_group_with_cancel(&tasks, &sessions, metrics, cancel_token)
            .await;

        match result {
            Err(OrchestratorError::Task(TaskError::Cancelled(message))) => {
                assert!(message.contains("shutdown request"));
            }
            other => panic!("expected shutdown cancellation, got {:?}", other),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_execute_task_with_retry_cancels_before_worker_acquisition() -> anyhow::Result<()>
    {
        let Some(session) = connect_test_session().await? else {
            return Ok(());
        };

        let config = create_test_config();
        let metrics = Arc::new(crate::metrics::MetricsCollector::new(10));
        let task_def = CliTaskDefinition {
            name: "pageview".to_string(),
            payload: Default::default(),
        };

        let _held_worker = session
            .acquire_worker(10)
            .await
            .expect("worker should be available");
        let cancel_token = CancellationToken::new();
        cancel_token.cancel();

        let result = super::retry::execute_task_with_retry(
            &task_def,
            &session,
            &config,
            metrics,
            cancel_token,
        )
        .await;

        assert!(matches!(
            result.status,
            crate::result::TaskStatus::Cancelled
        ));
        assert!(result
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("before worker acquisition"));
        // Worker still held by _held_worker — session is not idle, but has capacity
        assert!(!session.is_idle());
        assert!(session.has_available_workers());

        Ok(())
    }
}
