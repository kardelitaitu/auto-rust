//! Task orchestration and execution coordination module.
//!
//! The orchestrator manages:
//! - Parallel execution of task groups across sessions
//! - Global concurrency control via semaphores
//! - Retry logic with exponential backoff
//! - Error handling and load balancing
//! - Resource allocation and distribution

use crate::api::RetryPolicy;
use crate::config::Config;
use crate::metrics::{MetricsCollector};
use crate::session::Session;
use crate::cli::TaskDefinition;
use crate::logger::{LogContext, set_log_context};
use crate::result::{TaskErrorKind, TaskResult};
use anyhow::{Result, bail};
use log::{info, warn};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{timeout, Duration};

/// Formats milliseconds into a human-readable duration string.
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

/// Central coordinator for task execution across multiple browser sessions.
/// Manages global concurrency limits, session allocation, and task distribution.
/// Ensures efficient resource utilization and fault tolerance.
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
    /// Initializes global concurrency controls and prepares for task execution.
    ///
    /// # Arguments
    /// * `config` - Configuration settings for orchestration behavior
    ///
    /// # Returns
    /// A new Orchestrator instance ready for task execution
    pub fn new(config: Config) -> Self {
        Self {
            global_active_tasks: Arc::new(AtomicUsize::new(0)),
            global_semaphore: Arc::new(Semaphore::new(config.orchestrator.max_global_concurrency)),
            config,
        }
    }

    /// Executes a group of tasks across available browser sessions.
    /// Tasks within a group run in parallel across different sessions,
    /// respecting global concurrency limits.
    ///
    /// # Arguments
    /// * `group` - Slice of task definitions to execute
    /// * `sessions` - Available browser sessions for task execution
    ///
    /// # Returns
    /// Vector of task results, one for each task in the group
    pub async fn execute_group(
        &mut self,
        group: &[TaskDefinition],
        sessions: &[Session],
        metrics: Arc<MetricsCollector>,
    ) -> Result<()> {
        if sessions.is_empty() {
            bail!("No active sessions available");
        }

        if group.is_empty() {
            warn!("Empty task group, skipping");
            return Ok(());
        }

        let group_start = std::time::Instant::now();
        info!(
            "Executing group with {} task(s) across {} session(s)",
            group.len(),
            sessions.len()
        );

        // Apply group timeout
        let group_timeout = Duration::from_millis(self.config.orchestrator.group_timeout_ms);

        let task_futures: Vec<_> = group
        .iter()
        .map(|task_def| {
            let global_active = self.global_active_tasks.clone();
            let global_sem = self.global_semaphore.clone();
            let config = self.config.clone();
            let metrics = metrics.clone();

            async move {
                    // Global concurrency throttling
                    let _permit = global_sem.acquire().await?;
                    global_active.fetch_add(1, Ordering::SeqCst);

                    // Stagger task starts to prevent network spikes
                    tokio::time::sleep(Duration::from_millis(
                        config.orchestrator.task_stagger_delay_ms
                    ))
                    .await;

                    // Find an available session and execute the task
                    let result = execute_task_on_session(
                        task_def,
                        sessions,
                        &config,
                        metrics.clone(),
                    )
                    .await;

                    global_active.fetch_sub(1, Ordering::SeqCst);
                    result
                }
            })
            .collect();

        // Execute all tasks in parallel within the group
        let results = timeout(group_timeout, async {
            futures::future::join_all(task_futures).await
        })
        .await;

        match results {
            Ok(results) => {
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
            Err(_) => {
                bail!(
                    "Group timeout exceeded ({}ms)",
                    self.config.orchestrator.group_timeout_ms
                );
            }
        }
    }
}

/// Execute a single task on ALL sessions in parallel
/// Wait for ALL sessions to complete (each runs independently)
async fn execute_task_on_session(
    task_def: &TaskDefinition,
    sessions: &[Session],
    config: &Config,
    metrics: Arc<MetricsCollector>,
) -> Result<()> {
    if sessions.is_empty() {
        bail!("No sessions available");
    }

    info!(
        "[{}] Starting task on {} sessions",
        task_def.name,
        sessions.len()
    );

    // Create parallel tasks for each session
    let session_futures: Vec<_> = sessions
        .iter()
        .map(|session| {
            let task_def = task_def.clone();
            let config = config.clone();
            let metrics = metrics.clone();
            async move {
                let result = execute_task_with_retry(&task_def, session, &config, metrics).await;
                (session.id.clone(), result)
            }
        })
        .collect();

    // Run ALL sessions in parallel and wait for ALL to complete
    let results = futures::future::join_all(session_futures).await;

    let mut success_count = 0;
    let mut failed_sessions = Vec::new();

    for (session_id, task_result) in results {
        metrics.task_completed_from_result(
            task_def.name.clone(),
            session_id.clone(),
            &task_result,
        );

        if task_result.is_success() {
            info!("[{}][{}] Completed", session_id, task_def.name);
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
        info!(
            "[{}] All {} sessions completed successfully",
            task_def.name,
            sessions.len()
        );
        Ok(())
    } else {
        warn!(
            "[{}] {}/{} sessions failed: {}",
            task_def.name,
            failed_sessions.len(),
            sessions.len(),
            failed_sessions.join(", ")
        );
        // Return Ok if at least one session succeeded
        if success_count > 0 {
            Ok(())
        } else {
            bail!(
                "Task {} failed on all {} sessions",
                task_def.name,
                sessions.len()
            )
        }
    }
}

/// Execute a task with timeout and retry logic using exponential backoff
async fn execute_task_with_retry(
    task_def: &TaskDefinition,
    session: &Session,
    config: &Config,
    metrics: Arc<MetricsCollector>,
) -> TaskResult {
    let start = std::time::Instant::now();
    let max_retries = config.orchestrator.max_retries;
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

    // Acquire worker permit
    let permit = match session.acquire_worker(config.orchestrator.worker_wait_timeout_ms).await {
        Some(permit) => permit,
        None => {
            return TaskResult::failure(
                start.elapsed().as_millis() as u64,
                "Failed to acquire worker".to_string(),
                TaskErrorKind::Session,
            );
        }
    };

    if !session.is_healthy() {
        session.mark_unhealthy();
        drop(permit);
        return TaskResult::failure(
            start.elapsed().as_millis() as u64,
            format!("Session {} is unhealthy, skipping task", session.id),
            TaskErrorKind::Session,
        );
    }

    let payload_json = serde_json::Value::Object(
        task_def
            .payload
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
            .collect(),
    );

    if let Err(e) = crate::validation::validate_task(&task_def.name, payload_json.clone()) {
        drop(permit);
        return TaskResult::failure(
            start.elapsed().as_millis() as u64,
            format!("Task {} validation failed: {}", task_def.name, e),
            TaskErrorKind::Validation,
        );
    }

    let page = if task_def.name == "pageview" {
        let target_url = payload_json
            .get("url")
            .and_then(|v| v.as_str())
            .or_else(|| payload_json.get("value").and_then(|v| v.as_str()));
        match target_url {
            Some(target_url) => match session.acquire_page_at(target_url).await {
                Ok(page) => page,
                Err(e) => {
                    drop(permit);
                    return TaskResult::failure(
                        start.elapsed().as_millis() as u64,
                        e.to_string(),
                        TaskErrorKind::Browser,
                    );
                }
            },
            None => {
                drop(permit);
                return TaskResult::failure(
                    start.elapsed().as_millis() as u64,
                    "pageview requires a url or value payload".to_string(),
                    TaskErrorKind::Validation,
                );
            }
        }
    } else {
        match session.acquire_page().await {
            Ok(page) => page,
            Err(e) => {
                drop(permit);
                return TaskResult::failure(
                    start.elapsed().as_millis() as u64,
                    e.to_string(),
                    TaskErrorKind::Browser,
                );
            }
        }
    };

    let profile_name = session.behavior_profile.name.clone();
    let ctx = LogContext {
        session_id: Some(session.id.clone()),
        profile_name: Some(profile_name),
        task_name: Some(task_def.name.clone()),
    };
    set_log_context(ctx);

    let timeout_display = format_duration(config.orchestrator.task_timeout_ms);
    info!(
        "Executing task (timeout: {}, retries: {})...",
        timeout_display, max_retries
    );

    let mut last_failure: Option<(String, TaskErrorKind)> = None;
    let mut attempt = 0;

    for current_attempt in 1..=retry_policy.max_retries + 1 {
        attempt = current_attempt;

        let task_ctx = crate::runtime::task_context::TaskContext::new(
            session.id.clone(),
            page.clone(),
            session.behavior_profile.clone(),
            session.behavior_runtime,
        );

        let task_result = tokio::time::timeout(
            task_timeout,
            crate::task::perform_task(&task_ctx, &task_def.name, payload_json.clone(), max_retries),
        )
        .await;

        drop(task_ctx);

        match task_result {
            Ok(Ok(task_result)) if task_result.is_success() => {
                session.release_page(page).await;
                drop(permit);
                session.mark_healthy();
                return task_result.with_attempt(current_attempt, max_retries);
            }
            Ok(Ok(task_result)) => {
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
                let error = e.to_string();
                last_failure = Some((error.clone(), TaskErrorKind::classify(&error)));
            }
            Err(_) => {
                last_failure = Some((
                    format!("Task timed out after {}ms", config.orchestrator.task_timeout_ms),
                    TaskErrorKind::Timeout,
                ));
            }
        }

        if current_attempt >= retry_policy.max_retries + 1 {
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
        tokio::time::sleep(delay).await;
    }

    session.release_page(page).await;
    drop(permit);
    session.increment_failure();

    let (msg, kind) = last_failure.unwrap_or_else(|| {
        (
            "Unknown task failure".to_string(),
            TaskErrorKind::Unknown,
        )
    });
    if kind == TaskErrorKind::Timeout {
        session.mark_unhealthy();
    }

    TaskResult::failure(
        start.elapsed().as_millis() as u64,
        format!("Task {} failed after retries: {}", task_def.name, msg),
        kind,
    )
    .with_retry(attempt.max(1), max_retries, msg)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
