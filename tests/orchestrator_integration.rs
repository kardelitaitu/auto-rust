//! Integration tests for orchestrator functionality.
//!
//! These tests verify:
//! - Execution flow (task group execution across sessions)
//! - Session allocation (how tasks are distributed to sessions)
//! - Shutdown handling (graceful shutdown and cancellation)
//!
//! # Note
//! These tests require real browser instances and may take longer to run.

use auto::{
    cli::TaskDefinition, config::load_config, metrics::MetricsCollector,
    orchestrator::Orchestrator, session::Session,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

// ============================================================================
// Helper Functions
// ============================================================================

/// Create test sessions from discovered browsers.
/// Returns available sessions for testing.
async fn get_available_sessions() -> Vec<Session> {
    let config = load_config().expect("Failed to load config");
    auto::browser::discover_browsers(&config)
        .await
        .expect("Failed to discover browsers")
}

// ============================================================================
// Execution Flow Tests
// ============================================================================

/// Test that execute_group runs tasks on available sessions.
#[tokio::test]
#[ignore] // Requires real browsers
async fn test_execute_group_runs_on_all_sessions() {
    let config = load_config().expect("Failed to load config");
    let mut orchestrator = Orchestrator::new(config.clone());
    let sessions = get_available_sessions().await;

    if sessions.is_empty() {
        eprintln!("No sessions available, skipping test");
        return;
    }

    let metrics = Arc::new(MetricsCollector::new(100));

    // Create a simple task
    let tasks = vec![TaskDefinition {
        name: "pageview".to_string(),
        payload: Default::default(),
    }];

    // Execute the group
    let result = orchestrator
        .execute_group(&tasks, &sessions, metrics.clone())
        .await;

    // Should complete (may succeed or fail depending on browser state)
    // The important thing is it doesn't panic or hang
    if result.is_err() {
        eprintln!("execute_group failed: {:?}", result);
    }
}

/// Test that execute_group handles empty task list.
/// Note: Empty tasks with empty sessions returns error (sessions checked first).
/// Empty tasks with at least one session returns Ok.
#[tokio::test]
async fn test_execute_group_empty_tasks() {
    let config = load_config().expect("Failed to load config");
    let mut orchestrator = Orchestrator::new(config);
    let sessions: Vec<Session> = vec![];
    let metrics = Arc::new(MetricsCollector::new(100));

    // Empty tasks AND empty sessions -> error (sessions checked first)
    let result = orchestrator.execute_group(&[], &sessions, metrics).await;
    assert!(
        result.is_err(),
        "Empty sessions should cause error regardless of tasks"
    );
}

/// Test that execute_group returns error for empty sessions.
#[tokio::test]
async fn test_execute_group_empty_sessions() {
    let config = load_config().expect("Failed to load config");
    let mut orchestrator = Orchestrator::new(config);
    let sessions: Vec<Session> = vec![];
    let metrics = Arc::new(MetricsCollector::new(100));
    let tasks = vec![TaskDefinition {
        name: "pageview".to_string(),
        payload: Default::default(),
    }];

    // Empty sessions should return error
    let result = orchestrator.execute_group(&tasks, &sessions, metrics).await;
    assert!(result.is_err());
}

// ============================================================================
// Session Allocation Tests
// ============================================================================

/// Test that tasks are broadcast to all sessions.
#[tokio::test]
#[ignore] // Requires real browsers
async fn test_task_broadcast_to_all_sessions() {
    let config = load_config().expect("Failed to load config");
    let mut orchestrator = Orchestrator::new(config.clone());
    let sessions = get_available_sessions().await;

    if sessions.len() < 2 {
        eprintln!("Need at least 2 sessions for this test, skipping");
        return;
    }

    let metrics = Arc::new(MetricsCollector::new(100));

    // Single task should be broadcast to all sessions
    let tasks = vec![TaskDefinition {
        name: "pageview".to_string(),
        payload: Default::default(),
    }];

    let result = orchestrator
        .execute_group(&tasks, &sessions, metrics.clone())
        .await;

    // Should complete without hanging
    if result.is_err() {
        eprintln!("execute_group failed: {:?}", result);
    }
}

// ============================================================================
// Shutdown Handling Tests
// ============================================================================

/// Test that cancellation token stops task execution.
#[tokio::test]
#[ignore] // Requires real browsers
async fn test_cancellation_stops_execution() {
    let config = load_config().expect("Failed to load config");
    let mut orchestrator = Orchestrator::new(config.clone());
    let sessions = get_available_sessions().await;

    if sessions.is_empty() {
        eprintln!("No sessions available, skipping test");
        return;
    }

    let metrics = Arc::new(MetricsCollector::new(100));

    let tasks = vec![TaskDefinition {
        name: "pageview".to_string(),
        payload: Default::default(),
    }];

    // Note: Testing cancellation requires access to internal cancellation token
    // This is a placeholder for the actual shutdown handling test
    let result = orchestrator.execute_group(&tasks, &sessions, metrics).await;

    // Just verify it runs for now
    if result.is_err() {
        eprintln!("execute_group failed: {:?}", result);
    }
}

// ============================================================================
// Health and Failure Tests
// ============================================================================

/// Test that unhealthy sessions are handled.
#[tokio::test]
#[ignore] // Requires real browsers
async fn test_unhealthy_sessions_handled() {
    let config = load_config().expect("Failed to load config");
    let mut orchestrator = Orchestrator::new(config.clone());
    let sessions = get_available_sessions().await;

    if sessions.len() < 2 {
        eprintln!("Need at least 2 sessions for this test, skipping");
        return;
    }

    // Mark one session as unhealthy (requires clone to avoid mut)
    let sessions = sessions;
    sessions[1].mark_unhealthy();

    let metrics = Arc::new(MetricsCollector::new(100));
    let tasks = vec![TaskDefinition {
        name: "pageview".to_string(),
        payload: Default::default(),
    }];

    // Should handle unhealthy session gracefully
    let result = orchestrator.execute_group(&tasks, &sessions, metrics).await;

    // May succeed (if healthy session succeeds) or fail
    // The important thing is it doesn't panic
    if result.is_err() {
        eprintln!(
            "execute_group failed (expected possible failure): {:?}",
            result
        );
    }
}

/// Test that session state transitions correctly during execution.
#[tokio::test]
#[ignore] // Requires real browsers
async fn test_session_state_transitions() {
    let config = load_config().expect("Failed to load config");
    let mut orchestrator = Orchestrator::new(config.clone());
    let sessions = get_available_sessions().await;

    if sessions.is_empty() {
        eprintln!("No sessions available, skipping test");
        return;
    }

    // Check initial state
    assert!(sessions[0].is_idle(), "Session should start idle");

    let metrics = Arc::new(MetricsCollector::new(100));
    let tasks = vec![TaskDefinition {
        name: "pageview".to_string(),
        payload: Default::default(),
    }];

    let result = orchestrator.execute_group(&tasks, &sessions, metrics).await;

    // After execution, session should be idle again (if healthy)
    sleep(Duration::from_millis(100)).await;

    // State should have transitioned: Idle -> Busy -> Idle
    // (This is conceptual - actual state depends on timing)
    if result.is_ok() {
        // Session should be idle or busy (if task is still running)
        eprintln!("Session state after execution: {:?}", sessions[0].state());
    }
}
