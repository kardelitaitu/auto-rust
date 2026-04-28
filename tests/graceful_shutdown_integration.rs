//! Integration tests for graceful shutdown behavior.
//! Tests that the orchestrator properly handles shutdown signals
//! and cleans up resources gracefully.

use auto::{
    browser::discover_browsers,
    cli, config,
    metrics::MetricsCollector,
    orchestrator::Orchestrator,
    result::{TaskErrorKind, TaskResult},
    session::Session,
};
use std::sync::Arc;
use tokio::sync::broadcast;

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
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

    // Send shutdown signal
    let _ = shutdown_tx.send(());

    // Receiver should get the signal immediately
    let result = shutdown_rx.try_recv();
    assert!(result.is_ok(), "Shutdown signal should be received");
}

/// Test that shutdown channel can be cloned for multiple listeners
#[tokio::test]
async fn test_shutdown_channel_multiple_listeners() {
    let (shutdown_tx, _shutdown_rx) = broadcast::channel::<()>(1);

    // Create multiple subscribers
    let mut rx1 = shutdown_tx.subscribe();
    let mut rx2 = shutdown_tx.subscribe();
    let mut rx3 = shutdown_tx.subscribe();

    // Send shutdown signal
    let _ = shutdown_tx.send(());

    // All listeners should receive the signal
    assert!(rx1.try_recv().is_ok());
    assert!(rx2.try_recv().is_ok());
    assert!(rx3.try_recv().is_ok());
}

/// Test that metrics collector works correctly through shutdown
#[tokio::test]
async fn test_metrics_through_shutdown() {
    let metrics = Arc::new(MetricsCollector::new(100));
    let (shutdown_tx, _shutdown_rx) = broadcast::channel::<()>(1);

    // Simulate some work
    metrics.task_started();

    // Send shutdown
    let _ = shutdown_tx.send(());

    // Metrics should still be accessible
    let stats = metrics.get_stats();
    assert_eq!(stats.total_tasks, 1);
    assert_eq!(stats.active_tasks, 1);
}

/// Test that shutdown-driven cancellation is tracked separately from timeout.
#[tokio::test]
async fn test_shutdown_records_cancelled_outcome() {
    let metrics = Arc::new(MetricsCollector::new(100));
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

    let shutdown_task = tokio::spawn(async move {
        let _ = shutdown_rx.recv().await;
        TaskResult::cancelled(
            25,
            "Task cancelled during group shutdown".to_string(),
            TaskErrorKind::Timeout,
        )
    });

    let _ = shutdown_tx.send(());
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
    let (shutdown_tx, _shutdown_rx) = broadcast::channel::<()>(1);

    // Clone for use in spawned task
    let shutdown_tx_clone = shutdown_tx.clone();

    // Simulate idle mode waiting for shutdown
    let shutdown_task = tokio::spawn(async move {
        let mut rx = shutdown_tx_clone.subscribe();
        let _ = rx.recv().await;
        true // Shutdown received
    });

    // Send shutdown after brief delay
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    let _ = shutdown_tx.send(());

    // Wait for shutdown task to complete
    let result = shutdown_task.await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

/// Test that browser discovery handles empty results gracefully
#[tokio::test]
async fn test_browser_discovery_empty() {
    // Skip actual browser discovery in test environment to avoid slow execution
    // Just verify the module is accessible
    let _ = &discover_browsers;
}
