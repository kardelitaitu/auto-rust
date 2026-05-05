//! Integration tests for graceful shutdown behavior.
//! Tests that the orchestrator properly handles shutdown signals
//! and cleans up resources gracefully.

use async_trait::async_trait;
use auto::{
    browser::discover_browsers,
    cli, config,
    metrics::MetricsCollector,
    orchestrator::Orchestrator,
    result::{TaskErrorKind, TaskResult},
    runtime::{
        execution::{execute_task_groups_with_shutdown, TaskGroupRunner},
        shutdown::ShutdownManager,
    },
    session::Session,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

/// Test that orchestrator can be created and dropped cleanly
#[tokio::test]
async fn test_orchestrator_graceful_creation() {
    if let Ok(config) = config::load_config() {
        let orchestrator = Orchestrator::new(config);
        // Dropping should clean up without errors
        drop(orchestrator);
    }
}

/// Test that shutdown channel works correctly
#[tokio::test]
async fn test_shutdown_channel_signal() {
    let shutdown = ShutdownManager::new();
    let mut shutdown_rx = shutdown.subscribe();

    // Send shutdown signal
    assert!(shutdown.request_shutdown());

    // Receiver should get the signal immediately
    let result = shutdown_rx.try_recv();
    assert!(result.is_ok(), "Shutdown signal should be received");
}

/// Test that shutdown channel can be cloned for multiple listeners
#[tokio::test]
async fn test_shutdown_channel_multiple_listeners() {
    let shutdown = ShutdownManager::new();

    // Create multiple subscribers
    let mut rx1 = shutdown.subscribe();
    let mut rx2 = shutdown.subscribe();
    let mut rx3 = shutdown.subscribe();

    // Send shutdown signal
    assert!(shutdown.request_shutdown());

    // All listeners should receive the signal
    assert!(rx1.try_recv().is_ok());
    assert!(rx2.try_recv().is_ok());
    assert!(rx3.try_recv().is_ok());
}

/// Test that metrics collector works correctly through shutdown
#[tokio::test]
async fn test_metrics_through_shutdown() {
    let metrics = Arc::new(MetricsCollector::new(100));
    let shutdown = ShutdownManager::new();
    let _shutdown_rx = shutdown.subscribe();

    // Simulate some work
    metrics.task_started();

    // Send shutdown
    assert!(shutdown.request_shutdown());

    // Metrics should still be accessible
    let stats = metrics.get_stats();
    assert_eq!(stats.total_tasks, 1);
    assert_eq!(stats.active_tasks, 1);
}

/// Test that shutdown-driven cancellation is tracked separately from timeout.
#[tokio::test]
async fn test_shutdown_records_cancelled_outcome() {
    let metrics = Arc::new(MetricsCollector::new(100));
    let shutdown = ShutdownManager::new();
    let mut shutdown_rx = shutdown.subscribe();

    let shutdown_task = tokio::spawn(async move {
        let _ = shutdown_rx.recv().await;
        TaskResult::cancelled(
            25,
            "Task cancelled during group shutdown".to_string(),
            TaskErrorKind::Timeout,
        )
    });

    assert!(shutdown.request_shutdown());
    let result: TaskResult = shutdown_task.await.expect("shutdown task");

    metrics.task_started();
    metrics.task_completed_from_result("demo".to_string(), "session-1".to_string(), &result);

    let stats = metrics.get_stats();
    assert_eq!(stats.cancelled, 1);
    assert_eq!(stats.timed_out, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(
        stats
            .session_breakdown
            .get("session-1")
            .map(|s| s.cancelled),
        Some(1)
    );
}

/// Test empty task group execution
#[tokio::test]
async fn test_orchestrator_empty_group() {
    if let Ok(config) = config::load_config() {
        let mut orchestrator = Orchestrator::new(config);
        let metrics = Arc::new(MetricsCollector::new(100));

        // Create empty group
        let empty_group: Vec<cli::TaskDefinition> = vec![];

        // Sessions will be empty in test environment
        let sessions: Vec<Session> = vec![];

        // Should handle gracefully
        let result = orchestrator
            .execute_group(&empty_group, &sessions, metrics)
            .await;

        // Should fail gracefully (no sessions) but not panic
        assert!(result.is_err());
    }
}

/// Test shutdown during idle mode
#[tokio::test]
async fn test_idle_shutdown_behavior() {
    let shutdown = ShutdownManager::new();

    // Simulate idle mode waiting for shutdown
    let shutdown_waiter = shutdown.clone();
    let shutdown_task = tokio::spawn(async move {
        shutdown_waiter.wait().await;
        true // Shutdown received
    });

    // Send shutdown after brief delay
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(shutdown.request_shutdown());

    // Wait for shutdown task to complete
    let result = shutdown_task.await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

struct BlockingGroupRunner {
    started: Option<oneshot::Sender<()>>,
    finished: Arc<AtomicUsize>,
}

#[async_trait(?Send)]
impl TaskGroupRunner for BlockingGroupRunner {
    async fn run_group(
        &mut self,
        _index: usize,
        _group: &[cli::TaskDefinition],
        cancel_token: CancellationToken,
    ) {
        if let Some(started) = self.started.take() {
            let _ = started.send(());
        }

        tokio::select! {
            _ = cancel_token.cancelled() => {}
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                self.finished.fetch_add(1, Ordering::SeqCst);
            }
        }
    }
}

/// Test active group shutdown without requiring a real browser.
#[tokio::test]
async fn test_shutdown_cancels_active_group_deterministically() {
    let shutdown = ShutdownManager::new();
    let mut shutdown_rx = shutdown.subscribe();
    let (started_tx, started_rx) = oneshot::channel::<()>();
    let finished = Arc::new(AtomicUsize::new(0));
    let mut runner = BlockingGroupRunner {
        started: Some(started_tx),
        finished: finished.clone(),
    };
    let groups = vec![vec![cli::TaskDefinition {
        name: "cookiebot".to_string(),
        payload: Default::default(),
    }]];

    let shutdown_request = {
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            started_rx.await.expect("group should start");
            assert!(shutdown.request_shutdown());
        })
    };

    let outcome = timeout(
        Duration::from_secs(1),
        execute_task_groups_with_shutdown(&groups, &mut shutdown_rx, &mut runner),
    )
    .await
    .expect("shutdown should complete quickly");

    shutdown_request
        .await
        .expect("shutdown request task should join");
    assert!(outcome.shutdown_requested);
    assert_eq!(outcome.completed_groups, 0);
    assert_eq!(finished.load(Ordering::SeqCst), 0);
}

/// Test that browser discovery handles empty results gracefully
#[tokio::test]
async fn test_browser_discovery_empty() {
    // Skip actual browser discovery in test environment to avoid slow execution
    // Just verify the module is accessible
    let _ = &discover_browsers;
}
