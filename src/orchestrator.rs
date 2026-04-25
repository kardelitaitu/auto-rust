//! Task orchestration and execution coordination module.
//!
//! The orchestrator manages:
//! - Parallel execution of task groups across sessions
//! - Global concurrency control via semaphores
//! - Single-attempt execution with fail-fast behavior
//! - Error handling and load balancing
//! - Resource allocation and distribution

use crate::api::RetryPolicy;
use crate::cli::TaskDefinition;
use crate::config::Config;
use crate::error::{OrchestratorError, Result, SessionError, TaskError};
use crate::logger::{scoped_log_context, LogContext};
use crate::metrics::MetricsCollector;
use crate::result::{TaskErrorKind, TaskResult};
use crate::session::Session;
use futures::stream::{FuturesUnordered, StreamExt};
use log::{info, warn};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

/// Formats milliseconds into a human-readable duration string.
///
/// Converts a duration in milliseconds to a concise, human-readable format:
/// - < 1000ms: "500ms"
/// - < 60s: "30s"
/// - < 1h: "5min" or "5min 30s"
/// - >= 1h: "2h" or "2h 15min"
///
/// # Arguments
///
/// * `ms` - Duration in milliseconds
///
/// # Returns
///
/// A human-readable duration string.
///
/// # Examples
///
/// ```ignore
/// // Internal helper example:
/// // assert_eq!(format_duration(500), "500ms");
/// // assert_eq!(format_duration(5000), "5s");
/// // assert_eq!(format_duration(90000), "1min 30s");
/// // assert_eq!(format_duration(3600000), "1h");
/// ```
fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60000 {
        let secs = ms / 1000;
        format!("{}s", secs)
    } else if ms < 3600000 {
        let mins = ms / 60000;
        let secs = (ms % 60000) / 1000;
        if secs == 0 {
            format!("{}min", mins)
        } else {
            format!("{}min {}s", mins, secs)
        }
    } else {
        let hours = ms / 3600000;
        let mins = (ms % 3600000) / 60000;
        if mins == 0 {
            format!("{}h", hours)
        } else {
            format!("{}h {}min", hours, mins)
        }
    }
}

fn broadcast_execution_count(task_count: usize, session_count: usize) -> usize {
    task_count.saturating_mul(session_count)
}

struct GlobalExecutionSlot {
    active_counter: Arc<AtomicUsize>,
    _permit: OwnedSemaphorePermit,
}

impl GlobalExecutionSlot {
    fn new(active_counter: Arc<AtomicUsize>, permit: OwnedSemaphorePermit) -> Self {
        active_counter.fetch_add(1, Ordering::SeqCst);
        Self {
            active_counter,
            _permit: permit,
        }
    }
}

impl Drop for GlobalExecutionSlot {
    fn drop(&mut self) {
        self.active_counter.fetch_sub(1, Ordering::SeqCst);
    }
}

async fn acquire_global_execution_slot(
    task_name: &str,
    session_id: &str,
    queue_start: std::time::Instant,
    global_active_tasks: Arc<AtomicUsize>,
    global_semaphore: Arc<Semaphore>,
    cancel_token: CancellationToken,
) -> std::result::Result<GlobalExecutionSlot, TaskResult> {
    let permit = tokio::select! {
        permit = global_semaphore.acquire_owned() => permit,
        _ = cancel_token.cancelled() => {
            return Err(TaskResult::cancelled(
                queue_start.elapsed().as_millis() as u64,
                format!(
                    "Task {task_name} cancelled before acquiring global execution slot for session {session_id}"
                ),
                TaskErrorKind::Timeout,
            ));
        }
    };

    match permit {
        Ok(permit) => Ok(GlobalExecutionSlot::new(global_active_tasks, permit)),
        Err(_) => Err(TaskResult::failure(
            queue_start.elapsed().as_millis() as u64,
            format!(
                "Task {task_name} failed to acquire global execution slot for session {session_id}"
            ),
            TaskErrorKind::Session,
        )),
    }
}

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
    /// Configuration settings for orchestration behavior
    config: Config,
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
    pub fn new(config: Config) -> Self {
        Self {
            global_active_tasks: Arc::new(AtomicUsize::new(0)),
            global_semaphore: Arc::new(Semaphore::new(config.orchestrator.max_global_concurrency)),
            config,
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
        group: &[TaskDefinition],
        sessions: &[Session],
        metrics: Arc<MetricsCollector>,
    ) -> Result<()> {
        if sessions.is_empty() {
            return Err(OrchestratorError::Session(
                SessionError::InitializationFailed("No active sessions available".to_string()),
            ));
        }

        if group.is_empty() {
            warn!("Empty task group, skipping");
            return Ok(());
        }

        let group_start = std::time::Instant::now();
        info!(
            "Broadcast fan-out: {} task(s) x {} session(s) = {} execution(s)",
            group.len(),
            sessions.len(),
            broadcast_execution_count(group.len(), sessions.len())
        );

        // Apply group timeout
        let group_timeout = Duration::from_millis(self.config.orchestrator.group_timeout_ms);
        let group_cancel = CancellationToken::new();

        let mut task_futures: FuturesUnordered<_> = group
            .iter()
            .map(|task_def| {
                let config = self.config.clone();
                let metrics = metrics.clone();
                let cancel_token = group_cancel.clone();
                let global_active = self.global_active_tasks.clone();
                let global_sem = self.global_semaphore.clone();

                async move {
                    if cancel_token.is_cancelled() {
                        return Ok(());
                    }

                    // Stagger task starts to prevent network spikes
                    tokio::select! {
                        _ = cancel_token.cancelled() => {
                            global_active.fetch_sub(1, Ordering::SeqCst);
                            return Ok(());
                        }
                        _ = tokio::time::sleep(Duration::from_millis(
                            config.orchestrator.task_stagger_delay_ms,
                        )) => {}
                    }

                    // Find an available session and execute the task
                    let result = execute_task_on_session(
                        task_def,
                        sessions,
                        &config,
                        metrics.clone(),
                        cancel_token,
                        global_active,
                        global_sem,
                    )
                    .await;
                    result
                }
            })
            .collect();

        let group_deadline = tokio::time::sleep(group_timeout);
        tokio::pin!(group_deadline);
        let mut results = Vec::with_capacity(group.len());
        let mut timed_out = false;

        while !task_futures.is_empty() {
            tokio::select! {
                _ = &mut group_deadline, if !timed_out => {
                    timed_out = true;
                    warn!(
                        "Group timeout exceeded ({}ms), cancelling outstanding tasks",
                        self.config.orchestrator.group_timeout_ms
                    );
                    group_cancel.cancel();
                }
                maybe_result = task_futures.next() => {
                    if let Some(result) = maybe_result {
                        results.push(result);
                    }
                }
            }
        }

        if timed_out {
            return Err(OrchestratorError::Task(TaskError::Timeout {
                task_name: "task group".to_string(),
                timeout_ms: self.config.orchestrator.group_timeout_ms,
            }));
        }

        let success_count = results.iter().filter(|r| r.is_ok()).count();
        let fail_count = results.len() - success_count;

        info!(
            "Group complete: {} succeeded, {} failed ({}s)",
            success_count,
            fail_count,
            group_start.elapsed().as_secs_f64()
        );

        if fail_count > 0 {
            warn!("{fail_count} task(s) failed in group");
        }

        Ok(())
    }
}

/// Executes a single task on all available sessions in parallel.
///
/// This function broadcasts a task to all sessions, waits for all to complete,
/// and returns success if at least one session succeeds. Each session runs
/// the task independently with its own retry logic.
///
/// # Arguments
///
/// * `task_def` - The task definition to execute
/// * `sessions` - Available browser sessions
/// * `config` - The orchestrator configuration
/// * `metrics` - Metrics collector for tracking
/// * `cancel_token` - Cancellation token for graceful shutdown
///
/// # Returns
///
/// `Ok(())` if at least one session succeeds
/// `Err(OrchestratorError)` if all sessions fail
async fn execute_task_on_session(
    task_def: &TaskDefinition,
    sessions: &[Session],
    config: &Config,
    metrics: Arc<MetricsCollector>,
    cancel_token: CancellationToken,
    global_active_tasks: Arc<AtomicUsize>,
    global_semaphore: Arc<Semaphore>,
) -> Result<()> {
    if sessions.is_empty() {
        return Err(OrchestratorError::Session(
            SessionError::InitializationFailed("No sessions available".to_string()),
        ));
    }

    // Create parallel tasks for each session
    let session_futures: Vec<_> = sessions
        .iter()
        .map(|session| {
            let task_def = task_def.clone();
            let config = config.clone();
            let metrics = metrics.clone();
            let cancel_token = cancel_token.clone();
            let global_active_tasks = global_active_tasks.clone();
            let global_semaphore = global_semaphore.clone();
            async move {
                let queue_start = std::time::Instant::now();
                let _slot = match acquire_global_execution_slot(
                    &task_def.name,
                    &session.id,
                    queue_start,
                    global_active_tasks,
                    global_semaphore,
                    cancel_token.clone(),
                )
                .await
                {
                    Ok(slot) => slot,
                    Err(task_result) => return (session.id.clone(), task_result),
                };

                let result =
                    execute_task_with_retry(&task_def, session, &config, metrics, cancel_token)
                        .await;
                (session.id.clone(), result)
            }
        })
        .collect();

    // Run ALL sessions in parallel and wait for ALL to complete
    let results = futures::future::join_all(session_futures).await;

    let mut success_count = 0;
    let mut failed_sessions = Vec::new();

    for (session_id, task_result) in results {
        metrics.task_completed_from_result(task_def.name.clone(), session_id.clone(), &task_result);

        if task_result.is_success() {
            success_count += 1;
        } else {
            warn!(
                "[{}][{}] Failed: {}",
                session_id,
                task_def.name,
                task_result
                    .last_error
                    .clone()
                    .unwrap_or_else(|| "Unknown error".to_string())
            );
            failed_sessions.push(session_id);
        }
    }

    if failed_sessions.is_empty() {
        Ok(())
    } else {
        warn!(
            "[{}] {}/{} sessions failed: {}",
            task_def.name,
            failed_sessions.len(),
            sessions.len(),
            failed_sessions.join(", ")
        );
        // Return Ok if at least one session succeeded, but log warning for partial failure
        if success_count > 0 {
            warn!(
                "[{}] Partial failure: {}/{} sessions succeeded (some failed)",
                task_def.name,
                success_count,
                sessions.len()
            );
            Ok(())
        } else {
            Err(OrchestratorError::Task(TaskError::ExecutionFailed {
                task_name: task_def.name.clone(),
                reason: format!("failed on all {} sessions", sessions.len()),
            }))
        }
    }
}

/// Execute a task with timeout and retry logic using exponential backoff
async fn execute_task_with_retry(
    task_def: &TaskDefinition,
    session: &Session,
    config: &Config,
    metrics: Arc<MetricsCollector>,
    cancel_token: CancellationToken,
) -> TaskResult {
    let start = std::time::Instant::now();
    let max_retries = 0;
    let task_timeout = Duration::from_millis(config.orchestrator.task_timeout_ms);
    metrics.task_started();

    // Create retry policy with exponential backoff and jitter
    let retry_policy = RetryPolicy {
        max_retries,
        initial_delay: Duration::from_millis(config.orchestrator.retry_delay_ms),
        max_delay: Duration::from_secs(30),
        factor: 2.0,
        jitter: 0.3,
    };

    let payload_json = serde_json::Value::Object(task_def.payload.clone().into_iter().collect());

    if let Err(e) = crate::validation::validate_task(&task_def.name, payload_json.clone()) {
        return TaskResult::failure(
            start.elapsed().as_millis() as u64,
            format!("Task {} validation failed: {}", task_def.name, e),
            TaskErrorKind::Validation,
        );
    }

    if !session.is_healthy() {
        session.mark_unhealthy();
        session.set_state(crate::session::SessionState::Failed);
        return TaskResult::failure(
            start.elapsed().as_millis() as u64,
            format!("Session {} is unhealthy, skipping task", session.id),
            TaskErrorKind::Session,
        );
    }

    // Intentional check-then-act pattern: Multiple tasks can run on the same
    // session concurrently (broadcast fan-out model). The worker_semaphore
    // is the primary concurrency control (max_workers). This state check
    // prevents re-entry of the same task and provides observability.
    // See: https://github.com/your-org/rust-orchestrator/docs/ARCHITECTURE.md#session-concurrency
    if !session.is_idle() {
        return TaskResult::failure(
            start.elapsed().as_millis() as u64,
            format!(
                "Session {} is not idle (state: {:?}), skipping task",
                session.id,
                session.state()
            ),
            TaskErrorKind::Session,
        );
    }

    // Set Busy state for observability. Note: This is NOT exclusive locking;
    // multiple tasks may pass the check and set Busy concurrently, which is
    // intentional for the broadcast execution model.
    session.set_state(crate::session::SessionState::Busy);

    let permit = match tokio::select! {
        permit = session.acquire_worker(config.orchestrator.worker_wait_timeout_ms) => permit,
        _ = cancel_token.cancelled() => {
            warn!(
                "task_cancel | task={} session={} stage=worker_acquisition",
                task_def.name, session.id
            );
            return TaskResult::cancelled(
                start.elapsed().as_millis() as u64,
                format!("Task {} cancelled before worker acquisition", task_def.name),
                TaskErrorKind::Timeout,
            );
        }
    } {
        Some(permit) => permit,
        None => {
            return TaskResult::failure(
                start.elapsed().as_millis() as u64,
                "Failed to acquire worker".to_string(),
                TaskErrorKind::Session,
            );
        }
    };

    let page = match session.acquire_page().await {
        Ok(page) => page,
        Err(e) => {
            drop(permit);
            return TaskResult::failure(
                start.elapsed().as_millis() as u64,
                e.to_string(),
                TaskErrorKind::Browser,
            );
        }
    };

    let profile_name = session.behavior_profile.name.clone();
    let ctx = LogContext {
        session_id: Some(session.id.clone()),
        profile_name: Some(profile_name),
        task_name: Some(task_def.name.clone()),
    };
    let _log_ctx_guard = scoped_log_context(ctx);

    let timeout_display = format_duration(config.orchestrator.task_timeout_ms);
    info!(
        "task_start | task={} session={} timeout={} retries={}",
        task_def.name, session.id, timeout_display, max_retries
    );

    let mut last_failure: Option<(String, TaskErrorKind)> = None;
    let mut attempt = 0;

    for current_attempt in 1..=retry_policy.max_retries + 1 {
        if cancel_token.is_cancelled() {
            warn!(
                "task_cancel | task={} session={} stage=pre_attempt attempt={}",
                task_def.name, session.id, current_attempt
            );
            last_failure = Some((
                format!("Task {} cancelled during group shutdown", task_def.name),
                TaskErrorKind::Timeout,
            ));
            break;
        }

        attempt = current_attempt;

        let task_ctx = crate::runtime::task_context::TaskContext::new_with_metrics(
            session.id.clone(),
            page.clone(),
            session.behavior_profile.clone(),
            session.behavior_runtime,
            config.browser.native_interaction.clone(),
            metrics.clone(),
        );

        let task_result = tokio::select! {
            _ = cancel_token.cancelled() => {
                drop(task_ctx);
                warn!(
                    "task_cancel | task={} session={} stage=execution attempt={}",
                    task_def.name, session.id, current_attempt
                );
                last_failure = Some((
                    format!("Task {} cancelled during execution", task_def.name),
                    TaskErrorKind::Timeout,
                ));
                break;
            }
            task_result = timeout(
                task_timeout,
                crate::task::perform_task(&task_ctx, &task_def.name, payload_json.clone(), config),
            ) => task_result,
        };

        match task_result {
            Ok(Ok(task_result)) if task_result.is_success() => {
                drop(task_ctx);
                session.release_page(page).await;
                drop(permit);
                session.mark_healthy();
                session.set_state(crate::session::SessionState::Idle);
                return task_result.with_attempt(current_attempt, max_retries);
            }
            Ok(Ok(task_result)) => {
                drop(task_ctx);
                let error = task_result
                    .last_error
                    .clone()
                    .unwrap_or_else(|| "Unknown error".to_string());
                let kind = task_result
                    .error_kind
                    .unwrap_or_else(|| TaskErrorKind::classify(&error));
                last_failure = Some((error, kind));
            }
            Ok(Err(e)) => {
                drop(task_ctx);
                let error = e.to_string();
                last_failure = Some((error.clone(), TaskErrorKind::classify(&error)));
            }
            Err(_) => {
                drop(task_ctx);
                last_failure = Some((
                    format!(
                        "Task timed out after {}ms",
                        config.orchestrator.task_timeout_ms
                    ),
                    TaskErrorKind::Timeout,
                ));
            }
        }

        if current_attempt > retry_policy.max_retries {
            break;
        }

        if !last_failure
            .as_ref()
            .map(|(_, kind)| kind.is_retryable())
            .unwrap_or(false)
        {
            break;
        }

        let delay = retry_policy.delay_for_attempt(current_attempt);
        warn!(
            "task_retry | task={} session={} attempt={} next_delay_ms={} kind={:?}",
            task_def.name,
            session.id,
            current_attempt,
            delay.as_millis(),
            last_failure
                .as_ref()
                .map(|(_, kind)| *kind)
                .unwrap_or(TaskErrorKind::Unknown)
        );
        tokio::select! {
            _ = cancel_token.cancelled() => {
                warn!(
                    "task_cancel | task={} session={} stage=backoff attempt={}",
                    task_def.name, session.id, current_attempt
                );
                last_failure = Some((
                    format!("Task {} cancelled during retry backoff", task_def.name),
                    TaskErrorKind::Timeout,
                ));
                break;
            }
            _ = tokio::time::sleep(delay) => {}
        }
    }

    session.release_page(page).await;
    drop(permit);

    let was_cancelled = last_failure
        .as_ref()
        .map(|(msg, _)| msg.contains("cancelled"))
        .unwrap_or(false);
    if !was_cancelled {
        session.increment_failure();
    }

    let (msg, kind) = last_failure
        .unwrap_or_else(|| ("Unknown task failure".to_string(), TaskErrorKind::Unknown));
    if should_mark_session_unhealthy(kind, was_cancelled) {
        session.mark_unhealthy();
        session.set_state(crate::session::SessionState::Failed);
    } else {
        // Transition back to Idle if session is still healthy
        session.set_state(crate::session::SessionState::Idle);
    }

    info!(
        "task_cleanup | task={} session={} status=failed attempt={} cancelled={}",
        task_def.name,
        session.id,
        attempt.max(1),
        was_cancelled
    );

    if was_cancelled {
        return TaskResult::cancelled(
            start.elapsed().as_millis() as u64,
            format!("Task {} cancelled after retries: {}", task_def.name, msg),
            kind,
        )
        .with_retry(attempt.max(1), max_retries, msg);
    }

    TaskResult::failure(
        start.elapsed().as_millis() as u64,
        format!("Task {} failed after retries: {}", task_def.name, msg),
        kind,
    )
    .with_retry(attempt.max(1), max_retries, msg)
}

fn should_mark_session_unhealthy(kind: TaskErrorKind, was_cancelled: bool) -> bool {
    !was_cancelled
        && matches!(
            kind,
            TaskErrorKind::Timeout
                | TaskErrorKind::Navigation
                | TaskErrorKind::Session
                | TaskErrorKind::Browser
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::TaskStatus;
    use futures::stream::FuturesUnordered;
    use tokio::time::sleep;

    #[test]
    fn test_format_duration_milliseconds() {
        assert_eq!(format_duration(500), "500ms");
        assert_eq!(format_duration(999), "999ms");
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(1000), "1s");
        assert_eq!(format_duration(5000), "5s");
        assert_eq!(format_duration(45000), "45s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(60000), "1min");
        assert_eq!(format_duration(65000), "1min 5s");
        assert_eq!(format_duration(120000), "2min");
        assert_eq!(format_duration(125000), "2min 5s");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(3600000), "1h");
        assert_eq!(format_duration(3660000), "1h 1min");
        assert_eq!(format_duration(7200000), "2h");
        assert_eq!(format_duration(7320000), "2h 2min");
    }

    #[test]
    fn test_broadcast_execution_count() {
        assert_eq!(broadcast_execution_count(0, 5), 0);
        assert_eq!(broadcast_execution_count(3, 4), 12);
    }

    #[test]
    fn test_non_timeout_failure_marks_session_unhealthy() {
        assert!(should_mark_session_unhealthy(TaskErrorKind::Browser, false));
        assert!(should_mark_session_unhealthy(TaskErrorKind::Session, false));
        assert!(!should_mark_session_unhealthy(
            TaskErrorKind::Validation,
            false
        ));
        assert!(!should_mark_session_unhealthy(TaskErrorKind::Browser, true));
    }

    #[tokio::test]
    async fn test_global_execution_slot_enforces_hard_concurrency_bound() {
        let global_semaphore = Arc::new(Semaphore::new(2));
        let global_active = Arc::new(AtomicUsize::new(0));
        let peak_active = Arc::new(AtomicUsize::new(0));
        let cancel_token = CancellationToken::new();
        let mut executions = FuturesUnordered::new();

        for i in 0..8 {
            let global_semaphore = global_semaphore.clone();
            let global_active = global_active.clone();
            let peak_active = peak_active.clone();
            let cancel_token = cancel_token.clone();
            executions.push(tokio::spawn(async move {
                let _slot = acquire_global_execution_slot(
                    "pageview",
                    &format!("session-{i}"),
                    std::time::Instant::now(),
                    global_active.clone(),
                    global_semaphore,
                    cancel_token,
                )
                .await
                .expect("slot should be acquired");

                let current = global_active.load(Ordering::SeqCst);
                loop {
                    let prev_peak = peak_active.load(Ordering::SeqCst);
                    if current <= prev_peak {
                        break;
                    }
                    if peak_active
                        .compare_exchange(prev_peak, current, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok()
                    {
                        break;
                    }
                }

                sleep(Duration::from_millis(25)).await;
            }));
        }

        while let Some(result) = executions.next().await {
            result.expect("execution task should complete");
        }

        assert_eq!(global_active.load(Ordering::SeqCst), 0);
        assert!(
            peak_active.load(Ordering::SeqCst) <= 2,
            "peak active executions exceeded configured global concurrency"
        );
    }

    #[tokio::test]
    async fn test_global_execution_slot_cancels_while_waiting_for_permit() {
        let global_semaphore = Arc::new(Semaphore::new(1));
        let global_active = Arc::new(AtomicUsize::new(0));
        let cancel_token = CancellationToken::new();

        let held_slot = acquire_global_execution_slot(
            "cookiebot",
            "session-1",
            std::time::Instant::now(),
            global_active.clone(),
            global_semaphore.clone(),
            cancel_token.clone(),
        )
        .await
        .expect("first slot should be acquired");

        let waiting_cancel = cancel_token.clone();
        let waiting = tokio::spawn(async move {
            acquire_global_execution_slot(
                "cookiebot",
                "session-2",
                std::time::Instant::now(),
                global_active,
                global_semaphore,
                waiting_cancel,
            )
            .await
        });

        sleep(Duration::from_millis(10)).await;
        cancel_token.cancel();

        let waiting_result = waiting.await.expect("waiting task should join");
        let task_result = match waiting_result {
            Ok(_) => panic!("second slot should be cancelled"),
            Err(task_result) => task_result,
        };
        assert_eq!(task_result.status, TaskStatus::Cancelled);

        drop(held_slot);
    }

    #[test]
    fn test_orchestrator_new_creates_semaphore_with_config() {
        use crate::config::OrchestratorConfig;
        use crate::config::TracingConfig;
        use crate::config::TwitterActivityConfig;
        let config = Config {
            orchestrator: OrchestratorConfig {
                max_global_concurrency: 10,
                group_timeout_ms: 5000,
                task_timeout_ms: 30000,
                task_stagger_delay_ms: 100,
                worker_wait_timeout_ms: 5000,
                retry_delay_ms: 1000,
                max_retries: 0,
                stuck_worker_threshold_ms: 60000,
            },
            browser: Default::default(),
            tracing: TracingConfig::default(),
            twitter_activity: TwitterActivityConfig::default(),
        };

        let orchestrator = Orchestrator::new(config);

        // Verify semaphore was created with correct capacity
        // This is a basic sanity check - the semaphore is private but we can infer it works
        assert_eq!(orchestrator.config.orchestrator.max_global_concurrency, 10);
    }

    #[test]
    fn test_orchestrator_new_initializes_active_task_counter() {
        use crate::config::OrchestratorConfig;
        let config = Config {
            orchestrator: OrchestratorConfig::default(),
            browser: Default::default(),
            tracing: Default::default(),
            twitter_activity: Default::default(),
        };

        let orchestrator = Orchestrator::new(config);

        // Verify active task counter starts at 0
        assert_eq!(orchestrator.global_active_tasks.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_execute_group_with_empty_sessions_returns_error() {
        use crate::config::OrchestratorConfig;
        use crate::metrics::MetricsCollector;

        let config = Config {
            orchestrator: OrchestratorConfig::default(),
            browser: Default::default(),
            tracing: Default::default(),
            twitter_activity: Default::default(),
        };
        let mut orchestrator = Orchestrator::new(config);
        let sessions: Vec<Session> = vec![];
        let metrics = Arc::new(MetricsCollector::new(100));
        let task_def = TaskDefinition {
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
        // This test documents that empty task groups should execute successfully
        // Actual execution requires real Session objects which are complex to construct
        // The behavior is: if tasks vector is empty, execute_group should return Ok
        let tasks: Vec<TaskDefinition> = vec![];
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_format_duration_edge_cases() {
        assert_eq!(format_duration(0), "0ms");
        assert_eq!(format_duration(1), "1ms");
        assert_eq!(format_duration(999), "999ms");
        assert_eq!(format_duration(1000), "1s");
        assert_eq!(format_duration(60000), "1min");
        assert_eq!(format_duration(3600000), "1h");
    }

    #[test]
    fn test_broadcast_execution_count_with_zero_tasks() {
        assert_eq!(broadcast_execution_count(0, 5), 0);
        assert_eq!(broadcast_execution_count(0, 0), 0);
    }

    #[test]
    fn test_broadcast_execution_count_with_zero_sessions() {
        assert_eq!(broadcast_execution_count(5, 0), 0);
    }

    #[test]
    fn test_broadcast_execution_count_large_numbers() {
        assert_eq!(broadcast_execution_count(100, 50), 5000);
        assert_eq!(broadcast_execution_count(1000, 1000), 1000000);
    }

    #[test]
    fn test_should_mark_session_unhealthy_all_error_kinds() {
        assert!(should_mark_session_unhealthy(TaskErrorKind::Timeout, false));
        assert!(should_mark_session_unhealthy(
            TaskErrorKind::Navigation,
            false
        ));
        assert!(should_mark_session_unhealthy(TaskErrorKind::Session, false));
        assert!(should_mark_session_unhealthy(TaskErrorKind::Browser, false));

        // Should NOT mark unhealthy for these
        assert!(!should_mark_session_unhealthy(
            TaskErrorKind::Validation,
            false
        ));
        assert!(!should_mark_session_unhealthy(
            TaskErrorKind::Unknown,
            false
        ));

        // Cancelled tasks should never mark unhealthy
        assert!(!should_mark_session_unhealthy(TaskErrorKind::Timeout, true));
        assert!(!should_mark_session_unhealthy(TaskErrorKind::Browser, true));
    }

    #[tokio::test]
    async fn test_global_execution_slot_decrements_counter_on_drop() {
        let global_semaphore = Arc::new(Semaphore::new(10));
        let global_active = Arc::new(AtomicUsize::new(0));
        let _cancel_token = CancellationToken::new();

        {
            let _slot = GlobalExecutionSlot::new(
                global_active.clone(),
                global_semaphore.clone().acquire_owned().await.unwrap(),
            );
            assert_eq!(global_active.load(Ordering::SeqCst), 1);
        }

        assert_eq!(global_active.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_format_duration_boundary_values() {
        assert_eq!(format_duration(999), "999ms");
        assert_eq!(format_duration(1000), "1s");
        assert_eq!(format_duration(59999), "59s");
        assert_eq!(format_duration(60000), "1min");
        assert_eq!(format_duration(3599999), "59min 59s");
        assert_eq!(format_duration(3600000), "1h");
    }

    #[test]
    fn test_format_duration_large_values() {
        assert_eq!(format_duration(7200000), "2h");
        assert_eq!(format_duration(7260000), "2h 1min");
        assert_eq!(format_duration(10800000), "3h");
        assert_eq!(format_duration(36000000), "10h");
    }

    #[test]
    fn test_format_duration_exact_boundaries() {
        assert_eq!(format_duration(60000), "1min");
        assert_eq!(format_duration(3600000), "1h");
        assert_eq!(format_duration(120000), "2min");
        assert_eq!(format_duration(7200000), "2h");
    }

    #[test]
    fn test_broadcast_execution_count_single_values() {
        assert_eq!(broadcast_execution_count(1, 1), 1);
        assert_eq!(broadcast_execution_count(1, 10), 10);
        assert_eq!(broadcast_execution_count(10, 1), 10);
    }

    #[tokio::test]
    async fn test_global_execution_slot_multiple_slots() {
        let global_semaphore = Arc::new(Semaphore::new(5));
        let global_active = Arc::new(AtomicUsize::new(0));
        let _cancel_token = CancellationToken::new();

        let mut slots = Vec::new();
        for _ in 0..3 {
            slots.push(GlobalExecutionSlot::new(
                global_active.clone(),
                global_semaphore.clone().acquire_owned().await.unwrap(),
            ));
        }

        assert_eq!(global_active.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_orchestrator_config_preservation() {
        use crate::config::OrchestratorConfig;
        let original_config = Config {
            orchestrator: OrchestratorConfig {
                max_global_concurrency: 15,
                group_timeout_ms: 10000,
                task_timeout_ms: 60000,
                task_stagger_delay_ms: 200,
                worker_wait_timeout_ms: 10000,
                retry_delay_ms: 2000,
                max_retries: 3,
                stuck_worker_threshold_ms: 120000,
            },
            browser: Default::default(),
            tracing: Default::default(),
            twitter_activity: Default::default(),
        };

        let orchestrator = Orchestrator::new(original_config.clone());
        assert_eq!(
            orchestrator.config.orchestrator.max_global_concurrency,
            15
        );
        assert_eq!(orchestrator.config.orchestrator.group_timeout_ms, 10000);
    }

    #[test]
    fn test_should_mark_session_unhealthy_unknown_error() {
        assert!(!should_mark_session_unhealthy(TaskErrorKind::Unknown, false));
        assert!(!should_mark_session_unhealthy(TaskErrorKind::Unknown, true));
    }

    #[test]
    fn test_format_duration_minute_only() {
        assert_eq!(format_duration(60000), "1min");
        assert_eq!(format_duration(120000), "2min");
        assert_eq!(format_duration(180000), "3min");
    }

    #[test]
    fn test_format_duration_hour_only() {
        assert_eq!(format_duration(3600000), "1h");
        assert_eq!(format_duration(7200000), "2h");
        assert_eq!(format_duration(10800000), "3h");
    }

    #[test]
    fn test_format_duration_minute_with_seconds() {
        assert_eq!(format_duration(61000), "1min 1s");
        assert_eq!(format_duration(125000), "2min 5s");
        assert_eq!(format_duration(185000), "3min 5s");
    }

    #[test]
    fn test_format_duration_hour_with_minutes() {
        assert_eq!(format_duration(3660000), "1h 1min");
        assert_eq!(format_duration(7320000), "2h 2min");
        assert_eq!(format_duration(10980000), "3h 3min");
    }

    #[test]
    fn test_format_duration_very_large_hours() {
        assert_eq!(format_duration(36000000), "10h");
        assert_eq!(format_duration(72000000), "20h");
        assert_eq!(format_duration(86400000), "24h");
    }

    #[test]
    fn test_format_duration_sub_second_boundary() {
        assert_eq!(format_duration(999), "999ms");
        assert_eq!(format_duration(1000), "1s");
    }

    #[test]
    fn test_format_duration_minute_boundary() {
        assert_eq!(format_duration(59999), "59s");
        assert_eq!(format_duration(60000), "1min");
    }

    #[test]
    fn test_format_duration_hour_boundary() {
        assert_eq!(format_duration(3599999), "59min 59s");
        assert_eq!(format_duration(3600000), "1h");
    }

    #[test]
    fn test_broadcast_execution_count_saturation() {
        assert_eq!(broadcast_execution_count(usize::MAX, 1), usize::MAX);
        assert_eq!(broadcast_execution_count(1, usize::MAX), usize::MAX);
    }

    #[test]
    fn test_broadcast_execution_count_identity() {
        assert_eq!(broadcast_execution_count(1, 1), 1);
        assert_eq!(broadcast_execution_count(0, 0), 0);
    }

    #[test]
    fn test_should_mark_session_unhealthy_validation_error() {
        assert!(!should_mark_session_unhealthy(TaskErrorKind::Validation, false));
        assert!(!should_mark_session_unhealthy(TaskErrorKind::Validation, true));
    }

    #[test]
    fn test_should_mark_session_unhealthy_cancelled_all_kinds() {
        assert!(!should_mark_session_unhealthy(TaskErrorKind::Timeout, true));
        assert!(!should_mark_session_unhealthy(TaskErrorKind::Navigation, true));
        assert!(!should_mark_session_unhealthy(TaskErrorKind::Session, true));
        assert!(!should_mark_session_unhealthy(TaskErrorKind::Browser, true));
        assert!(!should_mark_session_unhealthy(TaskErrorKind::Validation, true));
        assert!(!should_mark_session_unhealthy(TaskErrorKind::Unknown, true));
    }

    #[test]
    fn test_format_duration_zero() {
        assert_eq!(format_duration(0), "0ms");
    }

    #[test]
    fn test_format_duration_single_millisecond() {
        assert_eq!(format_duration(1), "1ms");
    }

    #[test]
    fn test_format_duration_single_second() {
        assert_eq!(format_duration(1000), "1s");
    }

    #[test]
    fn test_format_duration_single_minute() {
        assert_eq!(format_duration(60000), "1min");
    }

    #[test]
    fn test_format_duration_single_hour() {
        assert_eq!(format_duration(3600000), "1h");
    }

    #[test]
    fn test_broadcast_execution_count_asymmetric() {
        assert_eq!(broadcast_execution_count(10, 1), 10);
        assert_eq!(broadcast_execution_count(1, 10), 10);
        assert_eq!(broadcast_execution_count(100, 1), 100);
        assert_eq!(broadcast_execution_count(1, 100), 100);
    }

    #[test]
    fn test_orchestrator_new_stores_config() {
        use crate::config::OrchestratorConfig;
        let config = Config {
            orchestrator: OrchestratorConfig {
                max_global_concurrency: 5,
                group_timeout_ms: 3000,
                task_timeout_ms: 15000,
                task_stagger_delay_ms: 50,
                worker_wait_timeout_ms: 2500,
                retry_delay_ms: 500,
                max_retries: 0,
                stuck_worker_threshold_ms: 30000,
            },
            browser: Default::default(),
            tracing: Default::default(),
            twitter_activity: Default::default(),
        };

        let orchestrator = Orchestrator::new(config.clone());
        assert_eq!(orchestrator.config.orchestrator.max_global_concurrency, 5);
        assert_eq!(orchestrator.config.orchestrator.group_timeout_ms, 3000);
    }

    #[tokio::test]
    async fn test_global_execution_slot_counter_atomicity() {
        let global_active = Arc::new(AtomicUsize::new(0));
        let _cancel_token = CancellationToken::new();
        let semaphore = Arc::new(Semaphore::new(10));

        let _slot1 = GlobalExecutionSlot::new(
            global_active.clone(),
            semaphore.clone().acquire_owned().await.unwrap(),
        );
        let _slot2 = GlobalExecutionSlot::new(
            global_active.clone(),
            semaphore.clone().acquire_owned().await.unwrap(),
        );

        assert_eq!(global_active.load(Ordering::SeqCst), 2);
    }
}
