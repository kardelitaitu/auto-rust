//! Health monitoring for self-healing.

// last audited 26-06-26 by Buffy

use super::strategy::RecoveryActionType;
use std::time::Instant;

/// System health status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemHealth {
    Healthy,
    Degraded,
    Recovering,
    Critical,
    Offline,
}

/// Type of health check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthCheckType {
    Connection,
    Resource,
    Performance,
    ErrorRate,
    Api,
}

/// Health check status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthCheckStatus {
    Passed,
    Failed,
    Skipped,
}

/// Individual health check result.
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub check_id: String,
    pub check_type: HealthCheckType,
    pub status: HealthCheckStatus,
    pub error: Option<String>,
    pub timestamp: Instant,
    pub recovery_action: Option<RecoveryActionType>,
}

/// Health monitoring for self-healing.
pub struct HealthMonitor {
    pub status: SystemHealth,
    pub checks: Vec<HealthCheckResult>,
    pub last_check: Instant,
    pub consecutive_failures: u32,
}

impl HealthMonitor {
    #[must_use]
    pub fn new() -> Self {
        Self {
            status: SystemHealth::Healthy,
            checks: vec![],
            last_check: Instant::now(),
            consecutive_failures: 0,
        }
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // -------------------------------------------------------------------------
    // SystemHealth enum
    // -------------------------------------------------------------------------

    #[test]
    fn system_health_variants_exist() {
        let h = SystemHealth::Healthy;
        let _d = SystemHealth::Degraded;
        let _r = SystemHealth::Recovering;
        let _c = SystemHealth::Critical;
        let _o = SystemHealth::Offline;
        assert_ne!(h, SystemHealth::Degraded);
    }

    // -------------------------------------------------------------------------
    // HealthCheckType enum
    // -------------------------------------------------------------------------

    #[test]
    fn health_check_type_variants_exist() {
        let _ = HealthCheckType::Connection;
        let _ = HealthCheckType::Resource;
        let _ = HealthCheckType::Performance;
        let _ = HealthCheckType::ErrorRate;
        let _ = HealthCheckType::Api;
    }

    #[test]
    fn health_check_type_equality() {
        assert_eq!(HealthCheckType::Connection, HealthCheckType::Connection);
        assert_ne!(HealthCheckType::Connection, HealthCheckType::Resource);
    }

    // -------------------------------------------------------------------------
    // HealthCheckStatus enum
    // -------------------------------------------------------------------------

    #[test]
    fn health_check_status_variants_exist() {
        assert_eq!(HealthCheckStatus::Passed, HealthCheckStatus::Passed);
        assert_eq!(HealthCheckStatus::Failed, HealthCheckStatus::Failed);
        assert_eq!(HealthCheckStatus::Skipped, HealthCheckStatus::Skipped);
    }

    // -------------------------------------------------------------------------
    // HealthCheckResult
    // -------------------------------------------------------------------------

    #[test]
    fn health_check_result_construction() {
        let result = HealthCheckResult {
            check_id: "check-1".to_string(),
            check_type: HealthCheckType::Connection,
            status: HealthCheckStatus::Passed,
            error: None,
            timestamp: Instant::now(),
            recovery_action: None,
        };

        assert_eq!(result.check_id, "check-1");
        assert_eq!(result.check_type, HealthCheckType::Connection);
        assert_eq!(result.status, HealthCheckStatus::Passed);
        assert!(result.error.is_none());
        assert!(result.recovery_action.is_none());
    }

    #[test]
    fn health_check_result_with_error() {
        let result = HealthCheckResult {
            check_id: "check-err".to_string(),
            check_type: HealthCheckType::Api,
            status: HealthCheckStatus::Failed,
            error: Some("API timeout".to_string()),
            timestamp: Instant::now(),
            recovery_action: Some(RecoveryActionType::RestartService),
        };

        assert_eq!(result.status, HealthCheckStatus::Failed);
        assert_eq!(result.error.as_ref().unwrap(), "API timeout");
        assert!(result.recovery_action.is_some());
    }

    // -------------------------------------------------------------------------
    // HealthMonitor
    // -------------------------------------------------------------------------

    #[test]
    fn health_monitor_new_starts_healthy() {
        let monitor = HealthMonitor::new();
        assert_eq!(monitor.status, SystemHealth::Healthy);
        assert!(monitor.checks.is_empty());
        assert_eq!(monitor.consecutive_failures, 0);
    }

    #[test]
    fn health_monitor_default_same_as_new() {
        let a = HealthMonitor::new();
        let b = HealthMonitor::default();
        assert_eq!(a.status, b.status);
        assert_eq!(a.checks.len(), b.checks.len());
        assert_eq!(a.consecutive_failures, b.consecutive_failures);
    }

    #[test]
    fn health_monitor_status_can_be_changed() {
        let mut monitor = HealthMonitor::new();
        monitor.status = SystemHealth::Degraded;
        assert_eq!(monitor.status, SystemHealth::Degraded);

        monitor.status = SystemHealth::Recovering;
        assert_eq!(monitor.status, SystemHealth::Recovering);

        monitor.status = SystemHealth::Critical;
        assert_eq!(monitor.status, SystemHealth::Critical);

        monitor.status = SystemHealth::Offline;
        assert_eq!(monitor.status, SystemHealth::Offline);
    }

    #[test]
    fn health_monitor_checks_can_be_pushed() {
        let mut monitor = HealthMonitor::new();
        let result = HealthCheckResult {
            check_id: "c1".to_string(),
            check_type: HealthCheckType::Resource,
            status: HealthCheckStatus::Failed,
            error: Some("OOM".into()),
            timestamp: Instant::now(),
            recovery_action: None,
        };
        monitor.checks.push(result.clone());

        assert_eq!(monitor.checks.len(), 1);
        assert_eq!(monitor.checks[0].check_id, "c1");
    }

    #[test]
    fn health_monitor_consecutive_failures_tracking() {
        let mut monitor = HealthMonitor::new();
        assert_eq!(monitor.consecutive_failures, 0);

        monitor.consecutive_failures = 1;
        assert_eq!(monitor.consecutive_failures, 1);

        monitor.consecutive_failures = 10;
        assert_eq!(monitor.consecutive_failures, 10);
    }

    #[test]
    fn health_monitor_last_check_is_recent() {
        let monitor = HealthMonitor::new();
        let elapsed = monitor.last_check.elapsed();
        // last_check is Instant::now() at construction — should be < 5 s ago
        assert!(elapsed < Duration::from_secs(5));
    }

    #[test]
    fn health_monitor_checks_preserved_across_access() {
        let mut monitor = HealthMonitor::new();
        monitor.checks.push(HealthCheckResult {
            check_id: "k1".to_string(),
            check_type: HealthCheckType::Connection,
            status: HealthCheckStatus::Passed,
            error: None,
            timestamp: Instant::now(),
            recovery_action: None,
        });
        assert_eq!(monitor.checks.len(), 1);
        assert_eq!(monitor.status, SystemHealth::Healthy);
    }

    // -------------------------------------------------------------------------
    // Debug / display sanity
    // -------------------------------------------------------------------------

    #[test]
    fn health_check_result_is_debuggable() {
        let result = HealthCheckResult {
            check_id: "dbg".to_string(),
            check_type: HealthCheckType::Performance,
            status: HealthCheckStatus::Skipped,
            error: None,
            timestamp: Instant::now(),
            recovery_action: None,
        };
        let _ = format!("{:?}", result);
    }

    // HealthMonitor does not derive Debug in the production struct;
    // verify that the struct is otherwise constructible and usable.
    #[test]
    fn health_monitor_full_construction() {
        let monitor = HealthMonitor {
            status: SystemHealth::Degraded,
            checks: vec![],
            last_check: Instant::now(),
            consecutive_failures: 3,
        };
        assert_eq!(monitor.consecutive_failures, 3);
        assert_eq!(monitor.status, SystemHealth::Degraded);
    }
}
