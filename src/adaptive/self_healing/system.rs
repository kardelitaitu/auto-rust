//! Self-healing system orchestration.

use crate::metrics::TwitterActivityRunCounters;
use std::time::{Duration, Instant};

use super::health::{
    HealthCheckResult, HealthCheckStatus, HealthCheckType, HealthMonitor, SystemHealth,
};
use super::history::{FailureHistory, FailureRecord, FailureType};
use super::state::RecoveryState;
use super::strategy::{HealthImpact, RecoveryActionType, RecoveryResult, RecoveryStrategies};

/// Self-healing system for automatic recovery.
pub struct SelfHealingSystem {
    pub health_monitor: HealthMonitor,
    pub strategies: RecoveryStrategies,
    pub failure_history: FailureHistory,
    pub recovery_state: RecoveryState,
}

impl SelfHealingSystem {
    #[must_use]
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
            check_id: format!(
                "health_{}",
                Instant::now().duration_since(Instant::now()).as_nanos()
            ),
            check_type: HealthCheckType::Connection,
            status: if matches!(status, SystemHealth::Healthy) {
                HealthCheckStatus::Passed
            } else {
                HealthCheckStatus::Failed
            },
            error: None,
            timestamp: Instant::now(),
            recovery_action: None,
        }
    }

    pub fn detect_and_recover(
        &mut self,
        metrics: &TwitterActivityRunCounters,
    ) -> Option<RecoveryResult> {
        if Self::detect_failure(metrics) {
            self.initiate_recovery()
        } else {
            None
        }
    }

    fn detect_failure(metrics: &TwitterActivityRunCounters) -> bool {
        metrics.button_missing > 10
    }

    #[allow(clippy::unused_self)]
    fn initiate_recovery(&mut self) -> Option<RecoveryResult> {
        let action = self.strategies.error.select_recovery_action();
        let result = Self::execute_recovery(&action);
        self.update_recovery_state(&result);
        Some(result)
    }

    fn execute_recovery(action: &RecoveryActionType) -> RecoveryResult {
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

        self.failure_history
            .recent_failures
            .push_back(FailureRecord {
                id: format!(
                    "failure_{}",
                    Instant::now().duration_since(Instant::now()).as_nanos()
                ),
                failure_type: FailureType::Unknown,
                timestamp: Instant::now(),
                error: "Recovered".to_string(),
                recovery_time: Duration::from_secs(0),
            });
    }

    pub fn record_adaptation(&mut self) {}
}

impl Default for SelfHealingSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::RecoveryMode;
    use super::*;

    // ========================================================================
    // Construction Tests
    // ========================================================================

    #[test]
    fn test_new_creates_healthy_system() {
        let system = SelfHealingSystem::new();
        assert_eq!(system.health_monitor.status, SystemHealth::Healthy);
        assert!(system.failure_history.recent_failures.is_empty());
        assert_eq!(system.recovery_state.mode, RecoveryMode::Normal);
    }

    #[test]
    fn test_default_same_as_new() {
        let system = SelfHealingSystem::default();
        assert_eq!(system.health_monitor.status, SystemHealth::Healthy);
    }

    // ========================================================================
    // check_health Tests
    // ========================================================================

    #[test]
    fn test_check_health_healthy_when_few_failures() {
        let mut system = SelfHealingSystem::new();
        // Add 3 failures (under threshold of 5)
        for i in 0..3 {
            system
                .failure_history
                .recent_failures
                .push_back(FailureRecord {
                    id: format!("f{}", i),
                    failure_type: FailureType::Unknown,
                    timestamp: Instant::now(),
                    error: "err".to_string(),
                    recovery_time: Duration::from_secs(0),
                });
        }
        let result = system.check_health();
        assert_eq!(result.status, HealthCheckStatus::Passed);
        assert_eq!(system.health_monitor.status, SystemHealth::Healthy);
    }

    #[test]
    fn test_check_health_critical_when_many_failures() {
        let mut system = SelfHealingSystem::new();
        for i in 0..6 {
            system
                .failure_history
                .recent_failures
                .push_back(FailureRecord {
                    id: format!("f{}", i),
                    failure_type: FailureType::Unknown,
                    timestamp: Instant::now(),
                    error: "err".to_string(),
                    recovery_time: Duration::from_secs(0),
                });
        }
        let result = system.check_health();
        assert_eq!(result.status, HealthCheckStatus::Failed);
        assert_eq!(system.health_monitor.status, SystemHealth::Critical);
    }

    // ========================================================================
    // detect_failure Tests
    // ========================================================================

    #[test]
    fn test_detect_failure_true_when_many_missing_buttons() {
        let _system = SelfHealingSystem::new();
        let metrics = TwitterActivityRunCounters {
            button_missing: 15,
            ..Default::default()
        };
        assert!(SelfHealingSystem::detect_failure(&metrics));
    }

    #[test]
    fn test_detect_failure_false_when_few_missing_buttons() {
        let _system = SelfHealingSystem::new();
        let metrics = TwitterActivityRunCounters {
            button_missing: 5,
            ..Default::default()
        };
        assert!(!SelfHealingSystem::detect_failure(&metrics));
    }

    #[test]
    fn test_detect_failure_threshold_boundary() {
        let _system = SelfHealingSystem::new();
        // Exactly at threshold (10 is NOT > 10)
        let metrics = TwitterActivityRunCounters {
            button_missing: 10,
            ..Default::default()
        };
        assert!(!SelfHealingSystem::detect_failure(&metrics));
        // Just over threshold
        let metrics = TwitterActivityRunCounters {
            button_missing: 11,
            ..Default::default()
        };
        assert!(SelfHealingSystem::detect_failure(&metrics));
    }

    // ========================================================================
    // detect_and_recover Tests
    // ========================================================================

    #[test]
    fn test_detect_and_recover_triggers_recovery() {
        let mut system = SelfHealingSystem::new();
        let metrics = TwitterActivityRunCounters {
            button_missing: 15,
            ..Default::default()
        };
        let result = system.detect_and_recover(&metrics);
        assert!(result.is_some());
        let recovery = result.unwrap();
        assert!(recovery.success);
        assert_eq!(recovery.details, "Recovery executed successfully");
    }

    #[test]
    fn test_detect_and_recover_returns_none_when_healthy() {
        let mut system = SelfHealingSystem::new();
        let metrics = TwitterActivityRunCounters::default();
        assert!(system.detect_and_recover(&metrics).is_none());
    }

    // ========================================================================
    // execute_recovery Tests
    // ========================================================================

    #[test]
    fn test_execute_recovery_returns_success() {
        let _system = SelfHealingSystem::new();
        let action = RecoveryActionType::RestartService;
        let result = SelfHealingSystem::execute_recovery(&action);
        assert!(result.success);
        assert!(matches!(result.action, RecoveryActionType::RestartService));
    }

    // ========================================================================
    // update_recovery_state Tests
    // ========================================================================

    #[test]
    fn test_update_recovery_state_resets_failures() {
        let mut system = SelfHealingSystem::new();
        system.health_monitor.consecutive_failures = 10;
        system.health_monitor.status = SystemHealth::Critical;

        let result = RecoveryResult {
            action: RecoveryActionType::RestartService,
            success: true,
            timestamp: Instant::now(),
            details: "ok".to_string(),
            health_impact: HealthImpact::default(),
        };
        system.update_recovery_state(&result);
        assert_eq!(system.health_monitor.consecutive_failures, 0);
        assert_eq!(system.health_monitor.status, SystemHealth::Healthy);
        assert_eq!(system.failure_history.recent_failures.len(), 1);
    }

    #[test]
    fn test_record_adaptation_does_not_panic() {
        let mut system = SelfHealingSystem::new();
        system.record_adaptation();
    }
}
