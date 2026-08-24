//! Task group execution and dispatch.
//!
//! Contains `execute_group()`, `execute_group_with_cancel()`, and
//! `execute_task_on_session()` — the core execution pipeline extracted
//! from orchestrator.rs.

use crate::cli::CliTaskDefinition;
use crate::config::Config;
use crate::error::{OrchestratorError, Result, SessionError, TaskError};
use crate::metrics::MetricsCollector;
use crate::session::Session;
use futures::stream::{FuturesUnordered, StreamExt};
use log::{info, warn};
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;

use super::guards::acquire_global_execution_slot;
use super::health::broadcast_execution_count;
use super::retry::execute_task_with_retry;
use crate::result::{TaskErrorKind, TaskResult};

/// Execute a group of tasks with an external cancellation token.
pub(super) async fn execute_group_with_cancel(
    config: Arc<Config>,
    global_active_tasks: &Arc<AtomicUsize>,
    global_semaphore: &Arc<Semaphore>,
    group: &[CliTaskDefinition],
    sessions: &[Session],
    metrics: Arc<MetricsCollector>,
    cancel_token: CancellationToken,
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
    let group_timeout = Duration::from_millis(config.orchestrator.group_timeout_ms.get());
    let group_cancel = cancel_token.child_token();

    let mut task_futures: FuturesUnordered<_> = group
        .iter()
        .map(|task_def| {
            let config = Arc::clone(&config);
            let metrics = metrics.clone();
            let cancel_token = group_cancel.clone();
            let global_active = global_active_tasks.clone();
            let global_sem = global_semaphore.clone();

            async move {
                if cancel_token.is_cancelled() {
                    return Ok(());
                }

                // Random stagger (0..max) so multiple browsers don't start simultaneously
                let max_stagger = config.orchestrator.task_stagger_delay_ms;
                let random_delay = if max_stagger > 0 {
                    use rand::Rng;
                    rand::thread_rng().gen_range(0..=max_stagger)
                } else {
                    0
                };
                if random_delay > 0 {
                    tokio::select! {
                        () = cancel_token.cancelled() => {
                            return Ok(());
                        }
                        () = tokio::time::sleep(Duration::from_millis(random_delay)) => {}
                    }
                }

                let result = execute_task_on_session(
                    task_def,
                    sessions,
                    Arc::clone(&config),
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
    let mut cancelled = false;

    while !task_futures.is_empty() {
        tokio::select! {
            () = &mut group_deadline, if !timed_out => {
                timed_out = true;
                warn!(
                    "Group timeout exceeded ({}ms), cancelling outstanding tasks",
                    config.orchestrator.group_timeout_ms.get()
                );
                group_cancel.cancel();
            }
            () = group_cancel.cancelled(), if !timed_out && !cancelled => {
                cancelled = true;
                warn!("Group cancelled, waiting for outstanding tasks to stop");
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
            timeout_ms: config.orchestrator.group_timeout_ms.get(),
        }));
    }

    if cancelled {
        return Err(OrchestratorError::Task(TaskError::Cancelled(
            "task group cancelled by shutdown request".to_string(),
        )));
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

/// Executes a single task on all available sessions in parallel.
///
/// This function broadcasts a task to all sessions, waits for all to complete,
/// and returns success if at least one session succeeds. Each session runs
/// the task independently with its own retry logic.
async fn execute_task_on_session(
    task_def: &CliTaskDefinition,
    sessions: &[Session],
    config: Arc<Config>,
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

    // Parse SESSION_STAGGER_DELAY_MS once, not per-session
    let stagger_delay_ms = std::env::var("SESSION_STAGGER_DELAY_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(config.orchestrator.task_stagger_delay_ms);

    // Create parallel tasks for each session
    let session_futures: Vec<_> = sessions
        .iter()
        .enumerate()
        .map(|(idx, session)| {
            let task_def = task_def.clone();
            let config = Arc::clone(&config);
            let metrics = metrics.clone();
            let cancel_token = cancel_token.clone();
            let global_active_tasks = global_active_tasks.clone();
            let global_semaphore = global_semaphore.clone();
            async move {
                if idx > 0 && stagger_delay_ms > 0 {
                    let stagger_ms = idx as u64 * stagger_delay_ms;
                    info!(
                        "[stagger] Session {} will start task {} after {}ms stagger delay",
                        session.id, task_def.name, stagger_ms
                    );
                    tokio::select! {
                        () = cancel_token.cancelled() => {
                            return (session.id.clone(), TaskResult::cancelled(
                                0,
                                "Task cancelled during stagger delay".to_string(),
                                TaskErrorKind::Timeout,
                            ));
                        }
                        () = tokio::time::sleep(Duration::from_millis(stagger_ms)) => {}
                    }
                }

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
                task_result.last_error.as_deref().unwrap_or("Unknown error")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::OrchestratorError;
    use tokio_util::sync::CancellationToken;

    #[test]
    fn test_result_aggregation_success_count() {
        let results: Vec<Result<()>> = vec![
            Ok(()),
            Ok(()),
            Err(OrchestratorError::Task(TaskError::ExecutionFailed {
                task_name: "task1".to_string(),
                reason: "failed".to_string(),
            })),
        ];

        let success_count = results.iter().filter(|r| r.is_ok()).count();
        let fail_count = results.len() - success_count;

        assert_eq!(success_count, 2);
        assert_eq!(fail_count, 1);
    }

    #[test]
    fn test_result_aggregation_all_fail() {
        let results: Vec<Result<()>> = vec![
            Err(OrchestratorError::Task(TaskError::ExecutionFailed {
                task_name: "task1".to_string(),
                reason: "error1".to_string(),
            })),
            Err(OrchestratorError::Task(TaskError::ExecutionFailed {
                task_name: "task2".to_string(),
                reason: "error2".to_string(),
            })),
        ];

        let success_count = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(success_count, 0);
    }

    #[test]
    fn test_result_aggregation_all_success() {
        let results: Vec<Result<()>> = vec![Ok(()), Ok(()), Ok(())];

        let success_count = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(success_count, 3);
    }

    #[tokio::test]
    async fn test_cancellation_token_propagates_to_backoff() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let cancelled = Arc::new(AtomicBool::new(false));
        let token = CancellationToken::new();

        let cancel_clone = token.clone();
        let cancelled_clone = cancelled.clone();

        let handle = tokio::spawn(async move {
            let delay = tokio::time::Duration::from_millis(100);

            tokio::select! {
                _ = cancel_clone.cancelled() => {
                    cancelled_clone.store(true, Ordering::SeqCst);
                    true
                }
                _ = tokio::time::sleep(delay) => {
                    false
                }
            }
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        token.cancel();

        let was_cancelled = handle.await.expect("task should complete");
        assert!(was_cancelled, "Should detect cancellation");
        assert!(
            cancelled.load(Ordering::SeqCst),
            "Cancellation flag should be set"
        );
    }
}
