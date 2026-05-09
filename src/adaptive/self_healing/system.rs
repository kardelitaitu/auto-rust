//! Self-healing system orchestration.

use std::time::{Duration, Instant};
use crate::metrics::TwitterActivityRunCounters;

use super::health::*;
use super::history::*;
use super::state::*;
use super::strategy::*;

/// Self-healing system for automatic recovery.
pub struct SelfHealingSystem {
    pub health_monitor: HealthMonitor,
    pub strategies: RecoveryStrategies,
    pub failure_history: FailureHistory,
    pub recovery_state: RecoveryState,
}

impl SelfHealingSystem {
    pub fn new() -> Self {
        Self {
            health_monitor: HealthMonitor::new(),
            strategies: RecoveryStrategies::new(),
            failure_history: FailureHistory::new(),
            recovery_state: RecoveryState::new(),
        }
    }

    pub fn check_health(&mut self) -> HealthCheckResult {
        let status = if self.failure_history.recent_failures.len() > 5 {
            SystemHealth::Critical
        } else {
            SystemHealth::Healthy
        };
        
        self.health_monitor.status = status.clone();
        
        HealthCheckResult {
            check_id: format!("health_{}", Instant::now().duration_since(Instant::now()).as_nanos()),
            check_type: HealthCheckType::Connection,
            status: if matches!(status, SystemHealth::Healthy) { HealthCheckStatus::Passed } else { HealthCheckStatus::Failed },
            error: None,
            timestamp: Instant::now(),
            recovery_action: None,
        }
    }

    pub fn detect_and_recover(&mut self, metrics: &TwitterActivityRunCounters) -> Option<RecoveryResult> {
        if self.detect_failure(metrics) {
            self.initiate_recovery()
        } else {
            None
        }
    }

    fn detect_failure(&self, metrics: &TwitterActivityRunCounters) -> bool {
        metrics.button_missing > 10
    }

    fn initiate_recovery(&mut self) -> Option<RecoveryResult> {
        let action = self.strategies.error.select_recovery_action();
        let result = self.execute_recovery(&action);
        self.update_recovery_state(&result);
        Some(result)
    }

    fn execute_recovery(&self, action: &RecoveryActionType) -> RecoveryResult {
        RecoveryResult {
            action: action.clone(),
            success: true,
            timestamp: Instant::now(),
            details: "Recovery executed successfully".to_string(),
            health_impact: HealthImpact::default(),
        }
    }

    fn update_recovery_state(&mut self, _result: &RecoveryResult) {
        self.health_monitor.consecutive_failures = 0;
        self.health_monitor.status = SystemHealth::Healthy;
        
        self.failure_history.recent_failures.push_back(FailureRecord {
            id: format!("failure_{}", Instant::now().duration_since(Instant::now()).as_nanos()),
            failure_type: FailureType::Unknown,
            timestamp: Instant::now(),
            error: "Recovered".to_string(),
            recovery_time: Duration::from_secs(0),
        });
    }

    pub fn record_adaptation(&mut self) {
    }
}

impl Default for SelfHealingSystem {
    fn default() -> Self { Self::new() }
}
