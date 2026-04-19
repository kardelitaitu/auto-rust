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

/// Configuration for the health logger.
#[derive(Debug, Clone)]
pub struct HealthLoggerConfig {
    /// How often to log health metrics (e.g., 60 seconds)
    pub interval: Duration,
    /// Memory usage threshold in bytes; warnings emitted if exceeded
    pub memory_warning_threshold_bytes: u64,
    /// Whether to include detailed memory info in logs
    pub verbose: bool,
}

impl Default for HealthLoggerConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(60),
            memory_warning_threshold_bytes: 1024 * 1024 * 1024, // 1 GiB
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
                    if mem > config.memory_warning_threshold_bytes {
                        warn!(
                            "[health] Memory usage {:.1} MiB exceeds threshold {:.1} MiB",
                            mem as f64 / 1024.0 / 1024.0,
                            config.memory_warning_threshold_bytes as f64 / 1024.0 / 1024.0
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
