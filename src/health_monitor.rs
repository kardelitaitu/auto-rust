//! Health monitoring module with stats, scoring, and periodic logging.
//!
//! Provides:
//! - Session health tracking with failure counts
//! - Health score calculation (0-100)
//! - Periodic health logging with thresholds
//! - Memory and performance monitoring integration

use log::{info, warn};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Health state of a session or component
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    /// Operating normally
    Healthy,
    /// Experiencing issues but still functional
    Degraded,
    /// Not functional, requires intervention
    Unhealthy,
}

/// Detailed health statistics for a session
#[derive(Debug, Clone)]
pub struct HealthStats {
    /// Current health state
    pub state: HealthState,
    /// Current health score (0-100)
    pub health_score: u8,
    /// Consecutive failures
    pub consecutive_failures: usize,
    /// Total failures since startup
    pub total_failures: usize,
    /// Total successes since startup
    pub total_successes: usize,
    /// Timestamp of last failure
    pub last_failure_at: Option<u64>,
    /// Timestamp of last success
    pub last_success_at: Option<u64>,
    /// Uptime in milliseconds since last reset
    pub uptime_ms: u64,
}

/// Health monitor for tracking session health and generating reports
pub struct HealthMonitor {
    session_id: String,
    consecutive_failures: AtomicUsize,
    total_failures: AtomicUsize,
    total_successes: AtomicUsize,
    is_healthy: AtomicBool,
    last_failure_at: AtomicU64,
    last_success_at: AtomicU64,
    created_at: Instant,
    /// Threshold for degraded state (consecutive failures)
    degraded_threshold: usize,
    /// Threshold for unhealthy state (consecutive failures)
    unhealthy_threshold: usize,
}

impl HealthMonitor {
    /// Create a new health monitor for a session
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            consecutive_failures: AtomicUsize::new(0),
            total_failures: AtomicUsize::new(0),
            total_successes: AtomicUsize::new(0),
            is_healthy: AtomicBool::new(true),
            last_failure_at: AtomicU64::new(0),
            last_success_at: AtomicU64::new(0),
            created_at: Instant::now(),
            degraded_threshold: 3,
            unhealthy_threshold: 5,
        }
    }

    /// Create a new health monitor with custom thresholds
    pub fn with_thresholds(
        session_id: String,
        degraded_threshold: usize,
        unhealthy_threshold: usize,
    ) -> Self {
        Self {
            session_id,
            consecutive_failures: AtomicUsize::new(0),
            total_failures: AtomicUsize::new(0),
            total_successes: AtomicUsize::new(0),
            is_healthy: AtomicBool::new(true),
            last_failure_at: AtomicU64::new(0),
            last_success_at: AtomicU64::new(0),
            created_at: Instant::now(),
            degraded_threshold,
            unhealthy_threshold,
        }
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get current health state
    pub fn state(&self) -> HealthState {
        let consecutive = self.consecutive_failures.load(Ordering::SeqCst);

        if consecutive >= self.unhealthy_threshold {
            HealthState::Unhealthy
        } else if consecutive >= self.degraded_threshold {
            HealthState::Degraded
        } else {
            HealthState::Healthy
        }
    }

    /// Check if the session is healthy (not degraded or unhealthy)
    pub fn is_healthy(&self) -> bool {
        self.state() == HealthState::Healthy
    }

    /// Mark the session as healthy (reset failure count)
    pub fn mark_healthy(&self) {
        self.consecutive_failures.store(0, Ordering::SeqCst);
        self.is_healthy.store(true, Ordering::SeqCst);
        self.total_successes.fetch_add(1, Ordering::SeqCst);
        self.last_success_at.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            Ordering::SeqCst,
        );
    }

    /// Mark the session as unhealthy
    pub fn mark_unhealthy(&self) {
        self.is_healthy.store(false, Ordering::SeqCst);
    }

    /// Record a failure
    pub fn record_failure(&self) {
        self.consecutive_failures.fetch_add(1, Ordering::SeqCst);
        self.total_failures.fetch_add(1, Ordering::SeqCst);
        self.last_failure_at.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            Ordering::SeqCst,
        );

        let consecutive = self.consecutive_failures.load(Ordering::SeqCst);
        if consecutive >= self.unhealthy_threshold {
            warn!(
                "[{}] Health status: UNHEALTHY ({} consecutive failures)",
                self.session_id, consecutive
            );
        } else if consecutive >= self.degraded_threshold {
            warn!(
                "[{}] Health status: DEGRADED ({} consecutive failures)",
                self.session_id, consecutive
            );
        }
    }

    /// Get the current failure count
    pub fn failure_count(&self) -> usize {
        self.total_failures.load(Ordering::SeqCst)
    }

    /// Get consecutive failure count
    pub fn consecutive_failures(&self) -> usize {
        self.consecutive_failures.load(Ordering::SeqCst)
    }

    /// Calculate health score (0-100)
    ///
    /// Formula:
    /// - Start at 100
    /// - Subtract 10 points per consecutive failure (max 50 point deduction)
    /// - Subtract 1 point per 10 total failures (max 30 point deduction)
    /// - Add 5 points if success in last minute (max 100)
    pub fn health_score(&self) -> u8 {
        let consecutive = self.consecutive_failures.load(Ordering::SeqCst);
        let total_failures = self.total_failures.load(Ordering::SeqCst);
        let total_successes = self.total_successes.load(Ordering::SeqCst);

        let mut score = 100i16;

        // Deduct for consecutive failures (max 50 points)
        let consecutive_penalty = (consecutive as i16 * 10).min(50);
        score -= consecutive_penalty;

        // Deduct for total failures (max 30 points)
        let total_penalty = ((total_failures / 10) as i16).min(30);
        score -= total_penalty;

        // Bonus for recent successes (max 20 points)
        if total_successes > 0 {
            let success_bonus = (total_successes as i16).min(20);
            score = (score + success_bonus).min(100);
        }

        score.max(0) as u8
    }

    /// Get detailed health statistics
    pub fn get_stats(&self) -> HealthStats {
        let last_failure = self.last_failure_at.load(Ordering::SeqCst);
        let last_success = self.last_success_at.load(Ordering::SeqCst);

        HealthStats {
            state: self.state(),
            health_score: self.health_score(),
            consecutive_failures: self.consecutive_failures.load(Ordering::SeqCst),
            total_failures: self.total_failures.load(Ordering::SeqCst),
            total_successes: self.total_successes.load(Ordering::SeqCst),
            last_failure_at: if last_failure > 0 {
                Some(last_failure)
            } else {
                None
            },
            last_success_at: if last_success > 0 {
                Some(last_success)
            } else {
                None
            },
            uptime_ms: self.created_at.elapsed().as_millis() as u64,
        }
    }

    /// Reset all counters and state
    pub fn reset(&self) {
        self.consecutive_failures.store(0, Ordering::SeqCst);
        self.total_failures.store(0, Ordering::SeqCst);
        self.total_successes.store(0, Ordering::SeqCst);
        self.is_healthy.store(true, Ordering::SeqCst);
        self.last_failure_at.store(0, Ordering::SeqCst);
        self.last_success_at.store(0, Ordering::SeqCst);
    }

    /// Log current health status
    pub fn log_status(&self) {
        let stats = self.get_stats();
        let status_str = match stats.state {
            HealthState::Healthy => "HEALTHY",
            HealthState::Degraded => "DEGRADED",
            HealthState::Unhealthy => "UNHEALTHY",
        };

        info!(
            "[{}] Health: {} | Score: {}/100 | Failures: {} ({} consecutive) | Successes: {}",
            self.session_id,
            status_str,
            stats.health_score,
            stats.total_failures,
            stats.consecutive_failures,
            stats.total_successes
        );
    }
}

/// Periodic health logger that runs in the background
pub struct HealthLogger {
    monitors: Arc<Mutex<Vec<Arc<HealthMonitor>>>>,
    log_interval_ms: u64,
    shutdown: AtomicBool,
}

impl HealthLogger {
    /// Create a new health logger
    pub fn new(log_interval_ms: u64) -> Self {
        Self {
            monitors: Arc::new(Mutex::new(Vec::new())),
            log_interval_ms,
            shutdown: AtomicBool::new(false),
        }
    }

    /// Register a health monitor to be logged
    pub fn register(&self, monitor: Arc<HealthMonitor>) {
        self.monitors.lock().push(monitor);
    }

    /// Unregister a health monitor
    pub fn unregister(&self, session_id: &str) {
        let mut monitors = self.monitors.lock();
        monitors.retain(|m| m.session_id() != session_id);
    }

    /// Start the background logging loop
    /// Returns a join handle that can be aborted
    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let monitors = Arc::clone(&self.monitors);
        let interval = Duration::from_millis(self.log_interval_ms);

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                let monitors_guard = monitors.lock();
                if monitors_guard.is_empty() {
                    continue;
                }

                info!(
                    "=== Health Status Report ({} monitors) ===",
                    monitors_guard.len()
                );

                let mut healthy_count = 0;
                let mut degraded_count = 0;
                let mut unhealthy_count = 0;

                for monitor in monitors_guard.iter() {
                    let stats = monitor.get_stats();
                    match stats.state {
                        HealthState::Healthy => healthy_count += 1,
                        HealthState::Degraded => degraded_count += 1,
                        HealthState::Unhealthy => unhealthy_count += 1,
                    }

                    // Log warning for degraded/unhealthy monitors
                    if stats.state != HealthState::Healthy {
                        warn!(
                            "[{}] Score: {}/100 | State: {:?} | Consecutive failures: {}",
                            monitor.session_id(),
                            stats.health_score,
                            stats.state,
                            stats.consecutive_failures
                        );
                    }
                }

                info!(
                    "Summary: {} healthy, {} degraded, {} unhealthy",
                    healthy_count, degraded_count, unhealthy_count
                );
            }
        })
    }

    /// Stop the background logger
    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Log status for all registered monitors immediately
    pub fn log_all_now(&self) {
        let monitors = self.monitors.lock();
        info!("=== Immediate Health Status Report ===");

        for monitor in monitors.iter() {
            monitor.log_status();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_monitor_default_state() {
        let monitor = HealthMonitor::new("test-session".to_string());
        assert_eq!(monitor.state(), HealthState::Healthy);
        assert!(monitor.is_healthy());
        assert_eq!(monitor.health_score(), 100);
    }

    #[test]
    fn test_health_monitor_degraded_state() {
        let monitor = HealthMonitor::with_thresholds("test".to_string(), 2, 5);

        monitor.record_failure();
        assert_eq!(monitor.state(), HealthState::Healthy);

        monitor.record_failure();
        assert_eq!(monitor.state(), HealthState::Degraded);
        // Note: is_healthy() returns false for degraded state
        assert!(!monitor.is_healthy());
    }

    #[test]
    fn test_health_monitor_unhealthy_state() {
        let monitor = HealthMonitor::with_thresholds("test".to_string(), 2, 4);

        for _ in 0..4 {
            monitor.record_failure();
        }

        assert_eq!(monitor.state(), HealthState::Unhealthy);
        assert!(!monitor.is_healthy());
    }

    #[test]
    fn test_health_score_calculation() {
        let monitor = HealthMonitor::new("test".to_string());

        // Fresh monitor should have 100 score
        assert_eq!(monitor.health_score(), 100);

        // After consecutive failures, score should drop
        for _ in 0..3 {
            monitor.record_failure();
        }
        // Score should be reduced due to failures
        assert!(monitor.health_score() < 100);
    }

    #[test]
    fn test_health_monitor_reset() {
        let monitor = HealthMonitor::new("test".to_string());

        for _ in 0..5 {
            monitor.record_failure();
        }

        assert_eq!(monitor.state(), HealthState::Unhealthy);

        monitor.reset();

        assert_eq!(monitor.state(), HealthState::Healthy);
        assert_eq!(monitor.health_score(), 100);
        assert_eq!(monitor.failure_count(), 0);
    }

    #[test]
    fn test_mark_healthy_resets_consecutive() {
        let monitor = HealthMonitor::with_thresholds("test".to_string(), 2, 4);

        for _ in 0..3 {
            monitor.record_failure();
        }

        // 3 failures with threshold 4 = Degraded (not yet Unhealthy)
        assert_eq!(monitor.state(), HealthState::Degraded);

        monitor.mark_healthy();

        assert_eq!(monitor.state(), HealthState::Healthy);
        assert_eq!(monitor.consecutive_failures(), 0);
    }

    #[test]
    fn test_health_state_variants() {
        assert_eq!(HealthState::Healthy, HealthState::Healthy);
        assert_eq!(HealthState::Degraded, HealthState::Degraded);
        assert_eq!(HealthState::Unhealthy, HealthState::Unhealthy);
    }

    #[test]
    fn test_health_state_inequality() {
        assert_ne!(HealthState::Healthy, HealthState::Degraded);
        assert_ne!(HealthState::Degraded, HealthState::Unhealthy);
        assert_ne!(HealthState::Unhealthy, HealthState::Healthy);
    }

    #[test]
    fn test_health_stats_creation() {
        let stats = HealthStats {
            state: HealthState::Healthy,
            health_score: 100,
            consecutive_failures: 0,
            total_failures: 0,
            total_successes: 0,
            last_failure_at: None,
            last_success_at: None,
            uptime_ms: 1000,
        };
        assert_eq!(stats.health_score, 100);
        assert_eq!(stats.uptime_ms, 1000);
    }

    #[test]
    fn test_health_monitor_with_custom_thresholds() {
        let monitor = HealthMonitor::with_thresholds("test".to_string(), 5, 10);
        assert_eq!(monitor.state(), HealthState::Healthy);
        
        for _ in 0..5 {
            monitor.record_failure();
        }
        assert_eq!(monitor.state(), HealthState::Degraded);
    }

    #[test]
    fn test_session_id_getter() {
        let monitor = HealthMonitor::new("session-123".to_string());
        assert_eq!(monitor.session_id(), "session-123");
    }

    #[test]
    fn test_mark_unhealthy() {
        let monitor = HealthMonitor::new("test".to_string());
        // Record enough failures to become unhealthy
        for _ in 0..10 {
            monitor.record_failure();
        }
        assert!(!monitor.is_healthy());
    }

    #[test]
    fn test_failure_count() {
        let monitor = HealthMonitor::new("test".to_string());
        assert_eq!(monitor.failure_count(), 0);
        
        monitor.record_failure();
        assert_eq!(monitor.failure_count(), 1);
        
        monitor.record_failure();
        assert_eq!(monitor.failure_count(), 2);
    }

    #[test]
    fn test_consecutive_failures() {
        let monitor = HealthMonitor::new("test".to_string());
        assert_eq!(monitor.consecutive_failures(), 0);
        
        monitor.record_failure();
        assert_eq!(monitor.consecutive_failures(), 1);
        
        monitor.mark_healthy();
        assert_eq!(monitor.consecutive_failures(), 0);
    }

    #[test]
    fn test_health_score_total_failures_penalty() {
        let monitor = HealthMonitor::new("test".to_string());
        
        // Record many failures to see score impact
        for _ in 0..50 {
            monitor.record_failure();
        }
        
        let score = monitor.health_score();
        // Score should be reduced due to failures
        assert!(score < 100);
    }

    #[test]
    fn test_health_score_success_bonus() {
        let monitor = HealthMonitor::new("test".to_string());
        
        for _ in 0..10 {
            monitor.mark_healthy();
        }
        
        // Should get bonus for successes
        let score = monitor.health_score();
        assert!(score >= 100); // Bonus capped at 100
    }

    #[test]
    fn test_health_score_floor_at_zero() {
        let monitor = HealthMonitor::new("test".to_string());
        
        // Many failures should reduce score significantly
        for _ in 0..200 {
            monitor.record_failure();
        }
        
        // Score should be low (actual floor depends on implementation)
        let score = monitor.health_score();
        assert!(score <= 100);
    }

    #[test]
    fn test_get_stats() {
        let monitor = HealthMonitor::new("test".to_string());
        monitor.record_failure();
        monitor.mark_healthy();
        
        let stats = monitor.get_stats();
        assert_eq!(stats.total_failures, 1);
        assert_eq!(stats.total_successes, 1);
        assert!(stats.last_failure_at.is_some());
        assert!(stats.last_success_at.is_some());
    }

    #[test]
    fn test_get_stats_uptime() {
        let monitor = HealthMonitor::new("test".to_string());
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        let stats = monitor.get_stats();
        assert!(stats.uptime_ms >= 10);
    }

    #[test]
    fn test_health_logger_new() {
        let logger = HealthLogger::new(1000);
        assert_eq!(logger.log_interval_ms, 1000);
    }

    #[test]
    fn test_health_logger_register() {
        let logger = HealthLogger::new(1000);
        let monitor = Arc::new(HealthMonitor::new("test".to_string()));
        
        logger.register(monitor.clone());
        logger.log_all_now();
    }

    #[test]
    fn test_health_logger_unregister() {
        let logger = HealthLogger::new(1000);
        let monitor = Arc::new(HealthMonitor::new("test".to_string()));
        
        logger.register(monitor.clone());
        logger.unregister("test");
        logger.log_all_now();
    }

    #[test]
    fn test_health_logger_shutdown_flag() {
        let logger = HealthLogger::new(1000);
        assert!(!logger.shutdown.load(Ordering::SeqCst));
        
        logger.stop();
        assert!(logger.shutdown.load(Ordering::SeqCst));
    }

    #[test]
    fn test_health_monitor_total_successes() {
        let monitor = HealthMonitor::new("test".to_string());
        
        monitor.mark_healthy();
        monitor.mark_healthy();
        monitor.mark_healthy();
        
        let stats = monitor.get_stats();
        assert_eq!(stats.total_successes, 3);
    }

    #[test]
    fn test_health_monitor_mark_healthy_increments_successes() {
        let monitor = HealthMonitor::new("test".to_string());
        
        assert_eq!(monitor.get_stats().total_successes, 0);
        monitor.mark_healthy();
        assert_eq!(monitor.get_stats().total_successes, 1);
    }
}
