//! Health monitoring for self-healing.

use std::time::Instant;
use super::strategy::RecoveryActionType;

/// System health status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemHealth {
    Healthy, Degraded, Recovering, Critical, Offline,
}

/// Type of health check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthCheckType {
    Connection, Resource, Performance, ErrorRate, Api,
}

/// Health check status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthCheckStatus {
    Passed, Failed, Skipped,
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
    fn default() -> Self { Self::new() }
}
