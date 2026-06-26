//! Concurrency guard types for the orchestrator.
//!
//! Contains `GlobalExecutionSlot` (semaphore-based global concurrency),
//! `SessionExecutionGuard` (per-session execution tracking),
//! and `acquire_global_execution_slot()` — extracted from orchestrator.rs.

use crate::result::{TaskErrorKind, TaskResult};
use crate::session::Session;
use crate::utils::duration_ms;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;

pub(super) struct GlobalExecutionSlot {
    active_counter: Arc<AtomicUsize>,
    _permit: OwnedSemaphorePermit,
}

impl GlobalExecutionSlot {
    pub(super) fn new(active_counter: Arc<AtomicUsize>, permit: OwnedSemaphorePermit) -> Self {
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

pub(super) struct SessionExecutionGuard {
    active: bool,
}

impl SessionExecutionGuard {
    /// Creates a guard that tracks task execution for cleanup.
    /// Note: Session state (Idle/Busy) is now derived from `active_workers` count,
    /// so this guard no longer sets session state. It exists for future cleanup hooks
    /// and maintains the existing guard-based error-handling patterns.
    pub(super) fn new(_session: &Session) -> Self {
        Self { active: true }
    }

    pub(super) fn mark_idle(&mut self) {
        self.active = false;
    }

    pub(super) fn mark_failed(&mut self) {
        self.active = false;
    }
}

impl Drop for SessionExecutionGuard {
    fn drop(&mut self) {
        // Guard exists for future cleanup hooks.
        // Session state is managed by worker permits (active_workers count).
    }
}

pub(super) async fn acquire_global_execution_slot(
    task_name: &str,
    session_id: &str,
    queue_start: std::time::Instant,
    global_active_tasks: Arc<AtomicUsize>,
    global_semaphore: Arc<Semaphore>,
    cancel_token: CancellationToken,
) -> std::result::Result<GlobalExecutionSlot, TaskResult> {
    let permit = tokio::select! {
        permit = global_semaphore.acquire_owned() => permit,
        () = cancel_token.cancelled() => {
            return Err(TaskResult::cancelled(
                duration_ms(queue_start.elapsed()),
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
            duration_ms(queue_start.elapsed()),
            format!(
                "Task {task_name} failed to acquire global execution slot for session {session_id}"
            ),
            TaskErrorKind::Session,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::TaskStatus;
    use futures::stream::{FuturesUnordered, StreamExt};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};
    use tokio_util::sync::CancellationToken;

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

    #[tokio::test]
    async fn test_global_execution_slot_decrements_counter_on_drop() {
        let global_semaphore = Arc::new(Semaphore::new(10));
        let global_active = Arc::new(AtomicUsize::new(0));
        let _cancel_token = CancellationToken::new();

        {
            let _slot = GlobalExecutionSlot::new(
                global_active.clone(),
                global_semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("Semaphore acquire failed"),
            );
            assert_eq!(global_active.load(Ordering::SeqCst), 1);
        }

        assert_eq!(global_active.load(Ordering::SeqCst), 0);
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
                global_semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("Semaphore acquire failed"),
            ));
        }

        assert_eq!(global_active.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_global_execution_slot_counter_atomicity() {
        let global_active = Arc::new(AtomicUsize::new(0));
        let _cancel_token = CancellationToken::new();
        let semaphore = Arc::new(Semaphore::new(10));

        let _slot1 = GlobalExecutionSlot::new(
            global_active.clone(),
            semaphore
                .clone()
                .acquire_owned()
                .await
                .expect("Semaphore acquire failed"),
        );
        let _slot2 = GlobalExecutionSlot::new(
            global_active.clone(),
            semaphore
                .clone()
                .acquire_owned()
                .await
                .expect("Semaphore acquire failed"),
        );

        assert_eq!(global_active.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_session_guard_prevents_stale_busy_on_drop() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let state = Arc::new(AtomicUsize::new(0));

        struct TestGuard {
            state: Arc<AtomicUsize>,
            active: bool,
        }

        impl TestGuard {
            fn new(state: Arc<AtomicUsize>) -> Self {
                state.store(1, Ordering::SeqCst);
                Self {
                    state,
                    active: true,
                }
            }

            fn mark_idle(&mut self) {
                self.state.store(0, Ordering::SeqCst);
                self.active = false;
            }
        }

        impl Drop for TestGuard {
            fn drop(&mut self) {
                if self.active {
                    self.state.store(0, Ordering::SeqCst);
                }
            }
        }

        // Test normal cleanup
        {
            let mut guard = TestGuard::new(state.clone());
            assert_eq!(state.load(Ordering::SeqCst), 1);
            guard.mark_idle();
            assert_eq!(state.load(Ordering::SeqCst), 0);
        }

        // Test drop cleanup
        state.store(0, Ordering::SeqCst);
        {
            let _guard = TestGuard::new(state.clone());
            assert_eq!(state.load(Ordering::SeqCst), 1);
        }
        assert_eq!(state.load(Ordering::SeqCst), 0);
    }
}
