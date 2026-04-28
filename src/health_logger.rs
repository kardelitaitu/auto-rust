//! Periodic health and memory monitoring.
//!
//! Spawns a background task that logs system health metrics (memory usage,
//! active tasks, success rate) at a configurable interval. Emits warnings when
//! memory usage exceeds a defined threshold.

use std::sync::Arc;
use std::time::Duration;
use sysinfo::System;
use tracing::{info, warn};

use crate::metrics::MetricsCollector;
use crate::utils::mouse::native_input_lock_metrics_snapshot;

/// Configuration for the health logger.
#[derive(Debug, Clone)]
pub struct HealthLoggerConfig {
    /// How often to log health metrics (e.g., 60 seconds)
    pub interval: Duration,
    /// Memory usage threshold as percentage (0-100); warnings emitted if exceeded
    pub memory_warning_percentage: f64,
    /// Whether to include detailed memory info in logs
    pub verbose: bool,
}

impl Default for HealthLoggerConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(60),
            memory_warning_percentage: 86.0, // 86%
            verbose: false,
        }
    }
}

/// Background health monitor that periodically logs metrics.
pub struct HealthLogger {
    config: HealthLoggerConfig,
    metrics: Arc<MetricsCollector>,
    shutdown: Arc<tokio::sync::Notify>,
}

impl HealthLogger {
    /// Creates a new HealthLogger.
    pub fn new(config: HealthLoggerConfig, metrics: Arc<MetricsCollector>) -> Self {
        Self {
            config,
            metrics,
            shutdown: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Starts the background monitoring loop.
    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let config = self.config.clone();
        let metrics = self.metrics.clone();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            let mut sys = System::new_all();
            sys.refresh_all();

            loop {
                tokio::select! {
                _ = tokio::time::sleep(config.interval) => {
                    let stats = metrics.get_stats();

                    // Memory info
                    let mem = sys.used_memory();
                    let total_mem = sys.total_memory();
                    let mem_pct = if total_mem > 0 {
                        (mem as f64 / total_mem as f64) * 100.0
                    } else {
                        0.0
                    };

                    let success_rate = if stats.total_tasks > 0 {
                        (stats.succeeded as f64 / stats.total_tasks as f64) * 100.0
                    } else {
                        0.0
                    };
                    let avg_duration = if stats.total_tasks > 0 {
                        stats.total_duration_ms as f64 / stats.total_tasks as f64
                    } else {
                        0.0
                    };

                    info!(
                        "[health] active_tasks={} total_tasks={} success_rate={:.1}% memory={:.1}MiB/{:.1}MiB ({:.1}%)",
                        stats.active_tasks,
                        stats.total_tasks,
                        success_rate,
                        mem as f64 / 1024.0 / 1024.0,
                        total_mem as f64 / 1024.0 / 1024.0,
                        mem_pct
                    );

                    let native_lock = native_input_lock_metrics_snapshot();
                    if native_lock.acquisitions > 0 {
                        info!(
                            "[health-native-lock] acquisitions={} contentions={} avg_wait_ms={:.1} max_wait_ms={} avg_hold_ms={:.1} max_hold_ms={}",
                            native_lock.acquisitions,
                            native_lock.contentions,
                            native_lock.avg_wait_ms,
                            native_lock.max_wait_ms,
                            native_lock.avg_hold_ms,
                            native_lock.max_hold_ms
                        );
                    }

                    if config.verbose {
                        info!(
                            "[health-detail] failures={} timeouts={} avg_duration_ms={:.0}",
                            stats.failed,
                            stats.timed_out,
                            avg_duration
                        );
                        if !stats.failure_breakdown.is_empty() {
                            info!("[health-detail] failure_breakdown={:?}", stats.failure_breakdown);
                        }
                    }

                    // Threshold warning
                    if mem_pct > config.memory_warning_percentage {
                        warn!(
                            "[health] Memory usage {:.1}% ({:.1} MiB) exceeds threshold {:.1}%",
                            mem_pct,
                            mem as f64 / 1024.0 / 1024.0,
                            config.memory_warning_percentage
                        );
                    }

                    // Refresh system info for next cycle
                    sys.refresh_memory();
                    if let Some(process) = sys.processes().get(&sysinfo::Pid::from_u32(
                        std::process::id()
                    )) {
                        let proc_mem = process.memory();
                        if config.verbose {
                            info!("[health] Process memory: {:.1} MiB", proc_mem as f64 / 1024.0 / 1024.0);
                        }
                    }
                }
                    _ = shutdown.notified() => {
                        info!("[health] Monitor shutting down");
                        break;
                    }
                }
            }
        })
    }

    /// Signals the background task to stop.
    pub fn stop(&self) {
        self.shutdown.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_health_logger_config_default() {
        let config = HealthLoggerConfig::default();
        assert_eq!(config.interval, Duration::from_secs(60));
        assert_eq!(config.memory_warning_percentage, 86.0);
        assert!(!config.verbose);
    }

    #[test]
    fn test_health_logger_config_custom() {
        let config = HealthLoggerConfig {
            interval: Duration::from_secs(30),
            memory_warning_percentage: 90.0,
            verbose: true,
        };
        assert_eq!(config.interval, Duration::from_secs(30));
        assert_eq!(config.memory_warning_percentage, 90.0);
        assert!(config.verbose);
    }

    #[test]
    fn test_health_logger_new() {
        let config = HealthLoggerConfig::default();
        let metrics = Arc::new(MetricsCollector::new(100));
        let logger = HealthLogger::new(config, metrics);

        // Logger should be created successfully
        // We can't easily test the internals without accessing private fields
        let _ = logger;
    }

    #[tokio::test]
    async fn test_health_logger_start_returns_handle() {
        let config = HealthLoggerConfig::default();
        let metrics = Arc::new(MetricsCollector::new(100));
        let logger = HealthLogger::new(config, metrics);

        let handle = logger.start();
        // Handle should be valid
        assert!(!handle.is_finished());

        // Clean up
        logger.stop();
        let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
    }

    #[tokio::test]
    async fn test_health_logger_stop() {
        let config = HealthLoggerConfig {
            interval: Duration::from_millis(100), // Short interval for testing
            memory_warning_percentage: 86.0,
            verbose: false,
        };
        let metrics = Arc::new(MetricsCollector::new(100));
        let logger = HealthLogger::new(config.clone(), metrics);

        let handle = logger.start();

        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Stop should not panic
        logger.stop();

        // Handle should complete within reasonable time
        let result = tokio::time::timeout(Duration::from_secs(2), handle).await;
        assert!(result.is_ok(), "Handle should complete after stop");
    }

    #[tokio::test]
    async fn test_health_logger_shutdown_signal() {
        let config = HealthLoggerConfig {
            interval: Duration::from_secs(60),
            memory_warning_percentage: 86.0,
            verbose: false,
        };
        let metrics = Arc::new(MetricsCollector::new(100));
        let logger = HealthLogger::new(config, metrics);

        let handle = logger.start();

        // Immediate stop should work
        logger.stop();

        let result = tokio::time::timeout(Duration::from_secs(1), handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_health_logger_with_metrics() {
        let config = HealthLoggerConfig {
            interval: Duration::from_millis(100),
            memory_warning_percentage: 86.0,
            verbose: false,
        };
        let metrics = Arc::new(MetricsCollector::new(100));

        // Add some metrics
        metrics.task_started();
        metrics.task_completed(crate::metrics::TaskMetrics {
            task_name: Arc::new("test".to_string()),
            status: crate::metrics::TaskStatus::Success,
            duration_ms: 100,
            session_id: Arc::new("test-session".to_string()),
            attempt: 1,
            error_kind: None,
            last_error: None,
        });

        let logger = HealthLogger::new(config, metrics.clone());
        let handle = logger.start();

        // Let it run for a bit
        tokio::time::sleep(Duration::from_millis(150)).await;

        logger.stop();
        let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;

        // Metrics should still be accessible
        let stats = metrics.get_stats();
        assert_eq!(stats.total_tasks, 1);
    }

    #[tokio::test]
    async fn test_health_logger_multiple_stops() {
        let config = HealthLoggerConfig::default();
        let metrics = Arc::new(MetricsCollector::new(100));
        let logger = HealthLogger::new(config, metrics);

        let handle = logger.start();

        // Multiple stops should not panic
        logger.stop();
        logger.stop();
        logger.stop();

        let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
    }

    #[test]
    fn test_health_logger_config_clone() {
        let config = HealthLoggerConfig::default();
        let cloned = config.clone();
        assert_eq!(config.interval, cloned.interval);
        assert_eq!(
            config.memory_warning_percentage,
            cloned.memory_warning_percentage
        );
        assert_eq!(config.verbose, cloned.verbose);
    }

    #[test]
    fn test_health_logger_config_debug() {
        let config = HealthLoggerConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("HealthLoggerConfig"));
    }
}
