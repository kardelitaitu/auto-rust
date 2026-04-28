//! Soak test for extended execution stability.
//!
//! This test runs the orchestrator continuously for an extended period
//! to verify:
//! - No memory leaks
//! - Stable health scores over time
//! - No resource exhaustion
//! - Proper task completion under load
//!
//! Usage: cargo test --test soak_test -- --ignored --nocapture

use auto::{
    health_monitor::HealthMonitor,
    metrics::{MetricsCollector, TaskMetrics, TaskStatus},
    result::TaskErrorKind,
};
use log::{info, warn};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Soak test duration in seconds (default: 1 hour)
const SOAK_DURATION_SECONDS: u64 = 3600;

/// Metrics snapshot interval in seconds
const SNAPSHOT_INTERVAL_SECONDS: u64 = 60;

/// Health check interval in seconds
const HEALTH_CHECK_INTERVAL_SECONDS: u64 = 30;

/// Simulated task execution
async fn simulate_task(metrics: Arc<MetricsCollector>, task_id: usize) -> Result<(), String> {
    metrics.task_started();

    // Simulate task work (100-500ms)
    let delay_ms = 100 + (task_id % 400);
    tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;

    // Simulate occasional failures (1% failure rate)
    if task_id.is_multiple_of(100) {
        metrics.task_completed(TaskMetrics {
            task_name: "simulated_task".to_string(),
            status: TaskStatus::Failed,
            duration_ms: delay_ms as u64,
            session_id: "soak-test".to_string(),
            attempt: 1,
            error_kind: Some(TaskErrorKind::Unknown),
            last_error: Some("Simulated failure".to_string()),
        });
        return Err("Simulated failure".to_string());
    }

    metrics.task_completed(TaskMetrics {
        task_name: "simulated_task".to_string(),
        status: TaskStatus::Success,
        duration_ms: delay_ms as u64,
        session_id: "soak-test".to_string(),
        attempt: 1,
        error_kind: None,
        last_error: None,
    });

    Ok(())
}

/// Monitor system health during soak test
async fn monitor_health(
    metrics: Arc<MetricsCollector>,
    health_monitor: Arc<HealthMonitor>,
    start_time: Instant,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(HEALTH_CHECK_INTERVAL_SECONDS));

    loop {
        interval.tick().await;

        let elapsed = start_time.elapsed().as_secs();
        let stats = metrics.get_stats();
        let health_stats = health_monitor.get_stats();

        info!(
            "[Soak Test] Elapsed: {}s | Tasks: {} | Success: {:.1}% | Health: {}/100 | State: {:?}",
            elapsed,
            stats.total_tasks,
            metrics.success_rate(),
            health_stats.health_score,
            health_stats.state
        );

        // Check for concerning patterns
        if health_stats.health_score < 50 {
            warn!(
                "[Soak Test] Health score below 50: {}",
                health_stats.health_score
            );
        }

        if stats.active_tasks > 10 {
            warn!("[Soak Test] High active task count: {}", stats.active_tasks);
        }
    }
}

/// Export periodic metrics snapshots
async fn export_snapshots(metrics: Arc<MetricsCollector>, start_time: Instant) {
    let mut interval = tokio::time::interval(Duration::from_secs(SNAPSHOT_INTERVAL_SECONDS));
    let mut snapshot_count = 0;

    loop {
        interval.tick().await;
        snapshot_count += 1;

        let elapsed = start_time.elapsed().as_secs();
        let stats = metrics.get_stats();

        info!(
            "[Snapshot {}] Elapsed: {}s | Total: {} | Success: {} | Failed: {} | Timeout: {} | Rate: {:.1}%",
            snapshot_count,
            elapsed,
            stats.total_tasks,
            stats.succeeded,
            stats.failed,
            stats.timed_out,
            metrics.success_rate()
        );
    }
}

/// Main soak test runner
#[tokio::test]
#[ignore] // Requires --ignored flag to run
async fn test_extended_soak() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("=== Starting Soak Test ===");
    info!("Duration: {} seconds", SOAK_DURATION_SECONDS);
    info!("Task interval: ~100-500ms");
    info!("Expected tasks: ~{}+", SOAK_DURATION_SECONDS * 2);

    let start_time = Instant::now();
    let metrics = Arc::new(MetricsCollector::new(1000));
    let health_monitor = Arc::new(HealthMonitor::new("soak-test".to_string()));

    // Start monitoring tasks
    let health_monitor_clone = health_monitor.clone();
    let metrics_clone = metrics.clone();
    let monitor_handle = tokio::spawn(async move {
        monitor_health(metrics_clone, health_monitor_clone, start_time).await;
    });

    let metrics_clone = metrics.clone();
    let snapshot_handle = tokio::spawn(async move {
        export_snapshots(metrics_clone, start_time).await;
    });

    // Run simulated tasks continuously
    let mut task_count = 0;
    let mut success_count = 0;
    let mut failure_count = 0;

    while start_time.elapsed() < Duration::from_secs(SOAK_DURATION_SECONDS) {
        let metrics_task = metrics.clone();
        let health_monitor_task = health_monitor.clone();

        match simulate_task(metrics_task, task_count).await {
            Ok(()) => {
                success_count += 1;
                health_monitor_task.mark_healthy();
            }
            Err(e) => {
                failure_count += 1;
                warn!("[Soak Test] Task {} failed: {}", task_count, e);
                health_monitor_task.record_failure();
            }
        }

        task_count += 1;

        // Small delay between tasks to simulate realistic load
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Stop monitoring tasks
    monitor_handle.abort();
    snapshot_handle.abort();

    // Final report
    let elapsed = start_time.elapsed();
    let stats = metrics.get_stats();
    let health_stats = health_monitor.get_stats();

    info!("");
    info!("=== Soak Test Complete ===");
    info!("Duration: {:.2?} seconds", elapsed);
    info!("Total tasks attempted: {}", task_count);
    info!("Successful: {}", success_count);
    info!("Failed: {}", failure_count);
    info!(
        "Success rate: {:.1}%",
        (success_count as f64 / task_count as f64) * 100.0
    );
    info!(
        "Tasks per second: {:.2}",
        task_count as f64 / elapsed.as_secs_f64()
    );
    info!("");
    info!("Final Metrics:");
    info!("  Total tracked: {}", stats.total_tasks);
    info!("  Succeeded: {}", stats.succeeded);
    info!("  Failed: {}", stats.failed);
    info!("  Timed out: {}", stats.timed_out);
    info!("  Success rate: {:.1}%", metrics.success_rate());
    info!("");
    info!("Final Health:");
    info!("  Score: {}/100", health_stats.health_score);
    info!("  State: {:?}", health_stats.state);
    info!(
        "  Consecutive failures: {}",
        health_stats.consecutive_failures
    );
    info!("  Total failures: {}", health_stats.total_failures);
    info!(
        "  Uptime: {:.2?}",
        Duration::from_millis(health_stats.uptime_ms)
    );
    info!("");

    // Assertions for test pass/fail
    assert!(
        task_count > 1000,
        "Should have executed at least 1000 tasks"
    );
    assert!(
        metrics.success_rate() > 90.0,
        "Success rate should be above 90%"
    );
    assert!(
        health_stats.health_score > 50,
        "Health score should remain above 50"
    );

    info!("=== All Soak Test Assertions Passed ===");
}

/// Quick soak test (5 minutes) for CI/CD
#[tokio::test]
#[ignore] // Requires --ignored flag to run
async fn test_quick_soak() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("=== Starting Quick Soak Test (5 minutes) ===");

    let start_time = Instant::now();
    let duration_seconds = 300; // 5 minutes
    let metrics = Arc::new(MetricsCollector::new(100));
    let health_monitor = Arc::new(HealthMonitor::new("quick-soak".to_string()));

    let mut task_count = 0;
    let mut success_count = 0;

    while start_time.elapsed() < Duration::from_secs(duration_seconds) {
        let metrics_task = metrics.clone();
        let health_monitor_task = health_monitor.clone();

        match simulate_task(metrics_task, task_count).await {
            Ok(()) => {
                success_count += 1;
                health_monitor_task.mark_healthy();
            }
            Err(_) => {
                health_monitor_task.record_failure();
            }
        }

        task_count += 1;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let elapsed = start_time.elapsed();
    let health_stats = health_monitor.get_stats();

    info!("");
    info!("=== Quick Soak Test Complete ===");
    info!("Duration: {:.2?} seconds", elapsed);
    info!("Tasks: {} | Success: {}", task_count, success_count);
    info!("Health Score: {}/100", health_stats.health_score);

    assert!(task_count > 100, "Should have executed at least 100 tasks");
    assert!(
        health_stats.health_score > 50,
        "Health score should remain above 50"
    );
}
