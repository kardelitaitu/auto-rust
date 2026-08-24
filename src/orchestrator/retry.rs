//! Task retry logic with exponential backoff.
//!
//! Contains `TaskAttemptFailure` error type and `execute_task_with_retry()`
//! — extracted from orchestrator.rs.

use crate::api::RetryPolicy;
use crate::cli::CliTaskDefinition;
use crate::config::Config;
use crate::logger::{scoped_log_context, LogContext};
use crate::metrics::MetricsCollector;
use crate::result::{TaskErrorKind, TaskResult, TaskStatus};
use crate::session::Session;
use crate::utils::duration_ms;
use log::{info, warn};
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

use super::guards::SessionExecutionGuard;
use super::health::should_mark_session_unhealthy;

#[derive(Debug, Clone)]
pub(super) struct TaskAttemptFailure {
    message: String,
    kind: TaskErrorKind,
    cancelled: bool,
}

impl TaskAttemptFailure {
    fn failed(message: String, kind: TaskErrorKind) -> Self {
        Self {
            message,
            kind,
            cancelled: false,
        }
    }

    fn cancelled(message: String, kind: TaskErrorKind) -> Self {
        Self {
            message,
            kind,
            cancelled: true,
        }
    }
}

/// Execute a task with timeout and retry logic using exponential backoff
pub(super) async fn execute_task_with_retry(
    task_def: &CliTaskDefinition,
    session: &Session,
    config: &Config,
    metrics: Arc<MetricsCollector>,
    cancel_token: CancellationToken,
) -> TaskResult {
    let start = std::time::Instant::now();
    let max_retries = config.orchestrator.max_retries;
    let policy = crate::task::policy::get_policy(&task_def.name);

    // Validate policy before use
    if let Err(e) = policy.validate() {
        return TaskResult::failure(
            0,
            format!("Invalid policy for task '{}': {}", task_def.name, e),
            TaskErrorKind::Validation,
        );
    }

    let task_timeout = Duration::from_millis(policy.max_duration_ms.get());
    metrics.task_started();

    // Create retry policy with exponential backoff and jitter
    let retry_policy = RetryPolicy {
        max_retries,
        initial_delay: Duration::from_millis(config.orchestrator.retry_delay_ms.get()),
        max_delay: Duration::from_secs(30),
        factor: 2.0,
        jitter: 0.3,
    };

    let payload_json = serde_json::Value::Object(task_def.payload.clone().into_iter().collect());

    if let Err(e) = crate::validation::validate_task(&task_def.name, payload_json.clone()) {
        return TaskResult::failure(
            duration_ms(start.elapsed()),
            format!("Task {} validation failed: {}", task_def.name, e),
            TaskErrorKind::Validation,
        );
    }

    if !session.is_healthy() {
        session.mark_unhealthy();
        session.set_state(crate::session::SessionState::Failed);
        return TaskResult::failure(
            duration_ms(start.elapsed()),
            format!("Session {} is unhealthy, skipping task", session.id),
            TaskErrorKind::Session,
        );
    }

    if !session.has_available_workers() {
        return TaskResult::failure(
            duration_ms(start.elapsed()),
            format!(
                "Session {} has no available workers ({} of {} busy), skipping task",
                session.id,
                session
                    .active_workers
                    .load(std::sync::atomic::Ordering::SeqCst),
                session.max_workers
            ),
            TaskErrorKind::Session,
        );
    }

    // This guard prevents cancellation/error paths from leaving the session
    // permanently Busy before page acquisition or explicit final cleanup.
    let mut session_guard = SessionExecutionGuard::new(session);

    let permit = match tokio::select! {
        permit = session.acquire_worker(config.orchestrator.worker_wait_timeout_ms.get()) => permit,
        () = cancel_token.cancelled() => {
            warn!(
                "task_cancel | task={} session={} stage=worker_acquisition",
                task_def.name, session.id
            );
            return TaskResult::cancelled(
                duration_ms(start.elapsed()),
                format!("Task {} cancelled before worker acquisition", task_def.name),
                TaskErrorKind::Timeout,
            );
        }
    } {
        Some(permit) => permit,
        None => {
            return TaskResult::failure(
                duration_ms(start.elapsed()),
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
                duration_ms(start.elapsed()),
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

    let timeout_display = super::health::format_duration(config.orchestrator.task_timeout_ms.get());
    info!(
        "task_start | task={} session={} timeout={} retries={}",
        task_def.name, session.id, timeout_display, max_retries
    );

    let mut last_failure: Option<TaskAttemptFailure> = None;
    let mut attempt = 0;

    for current_attempt in 1..=retry_policy.max_retries + 1 {
        if cancel_token.is_cancelled() {
            warn!(
                "task_cancel | task={} session={} stage=pre_attempt attempt={}",
                task_def.name, session.id, current_attempt
            );
            last_failure = Some(TaskAttemptFailure::cancelled(
                format!("Task {} cancelled during group shutdown", task_def.name),
                TaskErrorKind::Timeout,
            ));
            break;
        }

        attempt = current_attempt;

        let policy = crate::task::policy::get_policy(&task_def.name);
        let task_ctx = crate::runtime::task_context::TaskContext::new_with_metrics(
            session.id.clone(),
            page.clone(),
            session.behavior_profile.clone(),
            session.behavior_runtime,
            config.browser.native_interaction.clone(),
            metrics.clone(),
            &config.browser,
            policy,
            Some(cancel_token.clone()),
            session.browser_ws_url.clone(),
        );

        let task_result = tokio::select! {
            () = cancel_token.cancelled() => {
                drop(task_ctx);
                warn!(
                    "task_cancel | task={} session={} stage=execution attempt={}",
                    task_def.name, session.id, current_attempt
                );
                last_failure = Some(TaskAttemptFailure::cancelled(
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
                session_guard.mark_idle();
                return task_result.with_attempt(current_attempt, max_retries);
            }
            Ok(Ok(task_result)) => {
                drop(task_ctx);
                let error = task_result
                    .last_error
                    .as_deref()
                    .unwrap_or("Unknown error")
                    .to_string();
                let kind = task_result
                    .error_kind
                    .unwrap_or_else(|| TaskErrorKind::classify(&error));
                let failure = if matches!(task_result.status, TaskStatus::Cancelled) {
                    TaskAttemptFailure::cancelled(error, kind)
                } else {
                    TaskAttemptFailure::failed(error, kind)
                };
                last_failure = Some(failure);
            }
            Ok(Err(e)) => {
                drop(task_ctx);
                let error = e.to_string();
                last_failure = Some(TaskAttemptFailure::failed(
                    error.clone(),
                    TaskErrorKind::classify(&error),
                ));
            }
            Err(_) => {
                drop(task_ctx);
                last_failure = Some(TaskAttemptFailure::failed(
                    format!(
                        "Task '{}' exceeded policy timeout of {}ms",
                        task_def.name,
                        policy.max_duration_ms.get()
                    ),
                    TaskErrorKind::Timeout,
                ));
                log::warn!(
                    target: "task_policy_audit",
                    "Task killed due to policy timeout | task={} session={} timeout_ms={} event=timeout_enforced",
                    task_def.name,
                    session.id,
                    policy.max_duration_ms.get()
                );
            }
        }

        if current_attempt > retry_policy.max_retries {
            break;
        }

        if !last_failure
            .as_ref()
            .is_some_and(|failure| failure.kind.is_retryable())
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
                .map_or(TaskErrorKind::Unknown, |failure| failure.kind)
        );
        tokio::select! {
            () = cancel_token.cancelled() => {
                warn!(
                    "task_cancel | task={} session={} stage=backoff attempt={}",
                    task_def.name, session.id, current_attempt
                );
                last_failure = Some(TaskAttemptFailure::cancelled(
                    format!("Task {} cancelled during retry backoff", task_def.name),
                    TaskErrorKind::Timeout,
                ));
                break;
            }
            () = tokio::time::sleep(delay) => {}
        }
    }

    session.release_page(page).await;
    drop(permit);

    let was_cancelled = last_failure
        .as_ref()
        .is_some_and(|failure| failure.cancelled);
    if !was_cancelled {
        session.increment_failure();
    }

    let failure = last_failure.unwrap_or_else(|| {
        TaskAttemptFailure::failed("Unknown task failure".to_string(), TaskErrorKind::Unknown)
    });
    let msg = failure.message;
    let kind = failure.kind;
    if should_mark_session_unhealthy(kind, was_cancelled) {
        session.mark_unhealthy();
        session_guard.mark_failed();
    } else {
        session_guard.mark_idle();
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
            duration_ms(start.elapsed()),
            format!("Task {} cancelled after retries: {}", task_def.name, msg),
            kind,
        )
        .with_retry(attempt.max(1), max_retries, msg);
    }

    TaskResult::failure(
        duration_ms(start.elapsed()),
        format!("Task {} failed after retries: {}", task_def.name, msg),
        kind,
    )
    .with_retry(attempt.max(1), max_retries, msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::TaskErrorKind;

    #[test]
    fn test_task_attempt_failure_explicit_cancellation_state() {
        let failed = TaskAttemptFailure::failed("Some error".to_string(), TaskErrorKind::Unknown);
        assert!(!failed.cancelled, "Failed should not be cancelled");

        let cancelled = TaskAttemptFailure::cancelled(
            "Cancelled during shutdown".to_string(),
            TaskErrorKind::Timeout,
        );
        assert!(cancelled.cancelled, "Should be explicitly cancelled");
        assert_eq!(cancelled.kind, TaskErrorKind::Timeout);
    }

    #[test]
    fn test_cancelled_tasks_never_mark_session_unhealthy() {
        let error_kinds = [
            TaskErrorKind::Timeout,
            TaskErrorKind::Navigation,
            TaskErrorKind::Session,
            TaskErrorKind::Browser,
            TaskErrorKind::ExternalService,
        ];

        for kind in error_kinds {
            assert!(
                !super::should_mark_session_unhealthy(kind, true),
                "Cancelled task with {:?} should not mark session unhealthy",
                kind
            );
            assert!(
                super::should_mark_session_unhealthy(kind, false),
                "Non-cancelled task with {:?} should mark session unhealthy",
                kind
            );
        }

        assert!(!super::should_mark_session_unhealthy(
            TaskErrorKind::Validation,
            false
        ));
        assert!(!super::should_mark_session_unhealthy(
            TaskErrorKind::Validation,
            true
        ));
        assert!(!super::should_mark_session_unhealthy(
            TaskErrorKind::Unknown,
            false
        ));
        assert!(!super::should_mark_session_unhealthy(
            TaskErrorKind::Unknown,
            true
        ));
    }
}
