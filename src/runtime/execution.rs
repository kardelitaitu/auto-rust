//! Task group execution runtime module.
//!
//! This module provides the core execution logic for running task groups
//! with graceful shutdown support.

use async_trait::async_trait;
use log::{info, warn};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::cli::TaskDefinition;
use crate::metrics::MetricsCollector;
use crate::orchestrator::Orchestrator;
use crate::session::Session;

/// Trait for running task groups.
#[async_trait(?Send)]
pub trait TaskGroupRunner {
    /// Execute a single task group.
    async fn run_group(&mut self, index: usize, group: &[TaskDefinition]);
}

/// Runtime implementation of task group runner.
pub struct RuntimeGroupRunner<'a> {
    /// The orchestrator for task execution.
    pub orchestrator: &'a mut Orchestrator,
    /// Available browser sessions.
    pub sessions: &'a [Session],
    /// Metrics collector for task statistics.
    pub metrics: Arc<MetricsCollector>,
    /// Total number of groups to execute.
    pub total_groups: usize,
}

#[async_trait(?Send)]
impl TaskGroupRunner for RuntimeGroupRunner<'_> {
    async fn run_group(&mut self, index: usize, group: &[TaskDefinition]) {
        use crate::cli::format_task_groups;

        let task_groups_display = format_task_groups(&[group.to_vec()]);
        info!(
            "Executing group {}/{}: {task_groups_display}",
            index + 1,
            self.total_groups
        );

        let result = self
            .orchestrator
            .execute_group(group, self.sessions, self.metrics.clone())
            .await;
        if let Err(e) = result {
            warn!("Group {} failed: {}", index + 1, e);
        }
    }
}

/// Outcome of executing task groups.
#[derive(Debug, Clone, Copy)]
pub struct GroupExecutionOutcome {
    /// Number of groups that completed execution.
    pub completed_groups: usize,
    /// Whether shutdown was requested during execution.
    pub shutdown_requested: bool,
}

/// Execute task groups with graceful shutdown support.
///
/// This function runs each task group sequentially, checking for shutdown
/// signals before and during each group's execution.
pub async fn execute_task_groups_with_shutdown<R>(
    groups: &[Vec<TaskDefinition>],
    shutdown_rx: &mut broadcast::Receiver<()>,
    runner: &mut R,
) -> GroupExecutionOutcome
where
    R: TaskGroupRunner + ?Sized,
{
    let mut group_index = 0;
    let mut shutdown_requested = false;

    for (i, group) in groups.iter().enumerate() {
        if shutdown_rx.try_recv().is_ok() {
            info!("Shutdown requested, stopping before group {}", i + 1);
            shutdown_requested = true;
            break;
        }

        group_index = i + 1;
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("Shutdown requested, stopping during group {}", i + 1);
                shutdown_requested = true;
                break;
            }
            _ = runner.run_group(i, group) => {}
        }
    }

    GroupExecutionOutcome {
        completed_groups: group_index,
        shutdown_requested,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio::sync::oneshot;

    struct MockTaskGroupRunner {
        started: Option<oneshot::Sender<()>>,
        finished: Arc<AtomicUsize>,
        delay_ms: u64,
    }

    #[async_trait(?Send)]
    impl TaskGroupRunner for MockTaskGroupRunner {
        async fn run_group(&mut self, _index: usize, _group: &[TaskDefinition]) {
            if let Some(started) = self.started.take() {
                let _ = started.send(());
            }
            tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
            self.finished.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn test_execute_task_groups_with_shutdown_normal_completion() {
        let groups = vec![
            vec![TaskDefinition {
                name: "cookiebot".to_string(),
                payload: Default::default(),
            }],
            vec![TaskDefinition {
                name: "pageview".to_string(),
                payload: Default::default(),
            }],
        ];

        let (_tx, mut rx) = broadcast::channel::<()>(1);
        let finished = Arc::new(AtomicUsize::new(0));
        let mut runner = MockTaskGroupRunner {
            started: None,
            finished: finished.clone(),
            delay_ms: 1,
        };
        let outcome = execute_task_groups_with_shutdown(&groups, &mut rx, &mut runner).await;

        assert_eq!(outcome.completed_groups, 2);
        assert!(!outcome.shutdown_requested);
        assert_eq!(finished.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_execute_task_groups_with_shutdown_ctrl_c_during_group() {
        let groups = vec![
            vec![TaskDefinition {
                name: "cookiebot".to_string(),
                payload: Default::default(),
            }],
            vec![TaskDefinition {
                name: "pageview".to_string(),
                payload: Default::default(),
            }],
        ];

        let (tx, mut rx) = broadcast::channel::<()>(1);
        let (started_tx, started_rx) = oneshot::channel::<()>();
        let finished = Arc::new(AtomicUsize::new(0));
        let mut runner = MockTaskGroupRunner {
            started: Some(started_tx),
            finished: finished.clone(),
            delay_ms: 250,
        };

        let sender_task = tokio::spawn(async move {
            let _ = started_rx.await;
            let _ = tx.send(());
        });

        let outcome = execute_task_groups_with_shutdown(&groups, &mut rx, &mut runner).await;

        sender_task.await.expect("sender task should finish");
        assert_eq!(outcome.completed_groups, 1);
        assert!(outcome.shutdown_requested);
        assert_eq!(finished.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_execute_task_groups_with_shutdown_before_first_group() {
        let groups = vec![vec![TaskDefinition {
            name: "cookiebot".to_string(),
            payload: Default::default(),
        }]];

        let (tx, mut rx) = broadcast::channel::<()>(1);
        let _ = tx.send(());

        let finished = Arc::new(AtomicUsize::new(0));
        let mut runner = MockTaskGroupRunner {
            started: None,
            finished: finished.clone(),
            delay_ms: 1,
        };

        let outcome = execute_task_groups_with_shutdown(&groups, &mut rx, &mut runner).await;

        assert_eq!(outcome.completed_groups, 0);
        assert!(outcome.shutdown_requested);
        assert_eq!(finished.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_execute_task_groups_with_shutdown_empty_groups() {
        let groups: Vec<Vec<TaskDefinition>> = vec![];

        let (_tx, mut rx) = broadcast::channel::<()>(1);
        let finished = Arc::new(AtomicUsize::new(0));
        let mut runner = MockTaskGroupRunner {
            started: None,
            finished: finished.clone(),
            delay_ms: 1,
        };

        let outcome = execute_task_groups_with_shutdown(&groups, &mut rx, &mut runner).await;

        assert_eq!(outcome.completed_groups, 0);
        assert!(!outcome.shutdown_requested);
        assert_eq!(finished.load(Ordering::SeqCst), 0);
    }
}
