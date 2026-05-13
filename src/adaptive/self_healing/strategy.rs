//! Recovery strategies and classification.

use super::health::SystemHealth;
use std::time::{Duration, Instant};

/// Recovery action type.
#[derive(Debug, Clone)]
pub enum RecoveryActionType {
    RestartService,
    ScaleResources(f32),
    SwitchToBackup(String),
    ResetState,
    AlertOperator(String),
    Custom(String),
}

/// Recovery execution result.
#[derive(Debug, Clone)]
pub struct RecoveryResult {
    pub action: RecoveryActionType,
    pub success: bool,
    pub timestamp: Instant,
    pub details: String,
    pub health_impact: HealthImpact,
}

/// Health impact of recovery action.
#[derive(Debug, Clone, Default)]
pub struct HealthImpact {
    pub improvement: f32,
    pub resource_cost: f32,
    pub risk: f32,
}

/// Error category.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ErrorCategory {
    Connection,
    Resource,
    Api,
    Data,
    Logic,
    #[default]
    Unknown,
}

/// Error severity level.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ErrorSeverity {
    #[default]
    Low,
    Medium,
    High,
    Critical,
}

/// Error classification.
pub struct ErrorClassification {
    pub pattern: String,
    pub category: ErrorCategory,
    pub severity: ErrorSeverity,
}

/// Recovery step.
pub struct RecoveryStep {
    pub id: String,
    pub description: String,
    pub action: RecoveryActionType,
    pub expected_outcome: String,
}

/// Recovery conditions.
pub struct RecoveryConditions {
    pub required_health: SystemHealth,
    pub failure_threshold: u32,
    pub time_window: Duration,
}

/// Error handling procedure.
pub struct ErrorProcedure {
    pub id: String,
    pub name: String,
    pub steps: Vec<RecoveryStep>,
    pub conditions: RecoveryConditions,
}

/// Error recovery strategy.
#[derive(Default)]
pub struct ErrorRecovery {
    pub classifications: Vec<ErrorClassification>,
    pub procedures: Vec<ErrorProcedure>,
}

impl ErrorRecovery {
    pub fn select_recovery_action(&self) -> RecoveryActionType {
        RecoveryActionType::RestartService
    }
}

/// Connection recovery strategy.
pub struct ConnectionRecovery {
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub backoff_factor: f32,
    pub fallback_endpoints: Vec<String>,
}

impl Default for ConnectionRecovery {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
            backoff_factor: 2.0,
            fallback_endpoints: vec![],
        }
    }
}

/// Resource scaling configuration.
#[derive(Debug, Clone)]
pub struct ResourceScaling {
    pub scale_up_threshold: f32,
    pub scale_down_threshold: f32,
    pub max_scale_factor: f32,
}

impl Default for ResourceScaling {
    fn default() -> Self {
        Self {
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.2,
            max_scale_factor: 2.0,
        }
    }
}

/// Resource cleanup settings.
#[derive(Debug, Clone)]
pub struct ResourceCleanup {
    pub interval: Duration,
    pub threshold: f32,
}

impl Default for ResourceCleanup {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(300),
            threshold: 0.9,
        }
    }
}

/// Resource recovery strategy.
#[derive(Default)]
pub struct ResourceRecovery {
    pub scaling: ResourceScaling,
    pub cleanup: ResourceCleanup,
}

/// Performance tuning parameters.
#[derive(Debug, Clone, Default)]
pub struct PerformanceTuning {
    pub target_level: f32,
    pub adjustment_factor: f32,
}

/// Performance recovery strategy.
#[derive(Default)]
pub struct PerformanceRecovery {
    pub tuning: PerformanceTuning,
}

/// Recovery strategies available.
pub struct RecoveryStrategies {
    pub connection: ConnectionRecovery,
    pub resource: ResourceRecovery,
    pub error: ErrorRecovery,
    pub performance: PerformanceRecovery,
}

impl RecoveryStrategies {
    pub fn new() -> Self {
        Self {
            connection: ConnectionRecovery::default(),
            resource: ResourceRecovery::default(),
            error: ErrorRecovery::default(),
            performance: PerformanceRecovery::default(),
        }
    }
}

impl Default for RecoveryStrategies {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // ConnectionRecovery Tests
    // ========================================================================

    #[test]
    fn test_connection_recovery_defaults() {
        let recovery = ConnectionRecovery::default();
        assert_eq!(recovery.max_retries, 3);
        assert_eq!(recovery.retry_delay, Duration::from_secs(1));
        assert_eq!(recovery.backoff_factor, 2.0);
        assert!(recovery.fallback_endpoints.is_empty());
    }

    #[test]
    fn test_connection_recovery_custom() {
        let recovery = ConnectionRecovery {
            max_retries: 5,
            retry_delay: Duration::from_millis(500),
            backoff_factor: 1.5,
            fallback_endpoints: vec!["backup.example.com".to_string()],
        };
        assert_eq!(recovery.max_retries, 5);
        assert_eq!(recovery.retry_delay, Duration::from_millis(500));
        assert_eq!(recovery.backoff_factor, 1.5);
        assert_eq!(recovery.fallback_endpoints.len(), 1);
    }

    // ========================================================================
    // ResourceScaling Tests
    // ========================================================================

    #[test]
    fn test_resource_scaling_defaults() {
        let scaling = ResourceScaling::default();
        assert_eq!(scaling.scale_up_threshold, 0.8);
        assert_eq!(scaling.scale_down_threshold, 0.2);
        assert_eq!(scaling.max_scale_factor, 2.0);
    }

    #[test]
    fn test_resource_scaling_custom() {
        let scaling = ResourceScaling {
            scale_up_threshold: 0.9,
            scale_down_threshold: 0.1,
            max_scale_factor: 4.0,
        };
        assert_eq!(scaling.scale_up_threshold, 0.9);
        assert_eq!(scaling.max_scale_factor, 4.0);
    }

    // ========================================================================
    // ResourceCleanup Tests
    // ========================================================================

    #[test]
    fn test_resource_cleanup_defaults() {
        let cleanup = ResourceCleanup::default();
        assert_eq!(cleanup.interval, Duration::from_secs(300));
        assert_eq!(cleanup.threshold, 0.9);
    }

    // ========================================================================
    // ErrorRecovery Tests
    // ========================================================================

    #[test]
    fn test_error_recovery_default() {
        let recovery = ErrorRecovery::default();
        assert!(recovery.classifications.is_empty());
        assert!(recovery.procedures.is_empty());
    }

    #[test]
    fn test_error_recovery_select_action() {
        let recovery = ErrorRecovery::default();
        let action = recovery.select_recovery_action();
        assert!(matches!(action, RecoveryActionType::RestartService));
    }

    // ========================================================================
    // RecoveryActionType Tests
    // ========================================================================

    #[test]
    fn test_recovery_action_type_debug() {
        let action = RecoveryActionType::RestartService;
        assert_eq!(format!("{:?}", action), "RestartService");

        let scaled = RecoveryActionType::ScaleResources(2.5);
        assert_eq!(format!("{:?}", scaled), "ScaleResources(2.5)");

        let backup = RecoveryActionType::SwitchToBackup("primary".to_string());
        assert_eq!(format!("{:?}", backup), "SwitchToBackup(\"primary\")");

        let alert = RecoveryActionType::AlertOperator("disk full".to_string());
        assert_eq!(format!("{:?}", alert), "AlertOperator(\"disk full\")");
    }

    // ========================================================================
    // PerformanceTuning Tests
    // ========================================================================

    #[test]
    fn test_performance_tuning_defaults() {
        let tuning = PerformanceTuning::default();
        assert_eq!(tuning.target_level, 0.0);
        assert_eq!(tuning.adjustment_factor, 0.0);
    }

    #[test]
    fn test_performance_tuning_custom() {
        let tuning = PerformanceTuning {
            target_level: 0.9,
            adjustment_factor: 0.1,
        };
        assert_eq!(tuning.target_level, 0.9);
        assert_eq!(tuning.adjustment_factor, 0.1);
    }

    // ========================================================================
    // RecoveryStrategies Tests
    // ========================================================================

    #[test]
    fn test_recovery_strategies_new() {
        let strategies = RecoveryStrategies::new();
        assert_eq!(
            strategies.connection.max_retries,
            ConnectionRecovery::default().max_retries
        );
        assert_eq!(
            strategies.resource.scaling.scale_up_threshold,
            ResourceScaling::default().scale_up_threshold
        );
    }

    #[test]
    fn test_recovery_strategies_default() {
        let strategies = RecoveryStrategies::default();
        let new_strategies = RecoveryStrategies::new();
        assert_eq!(
            strategies.connection.max_retries,
            new_strategies.connection.max_retries
        );
    }

    // ========================================================================
    // ErrorCategory Tests
    // ========================================================================

    #[test]
    fn test_error_category_default() {
        let category = ErrorCategory::default();
        assert_eq!(category, ErrorCategory::Unknown);
    }

    #[test]
    fn test_error_category_variants() {
        assert_eq!(ErrorCategory::Connection as u8, 0);
        assert_eq!(ErrorCategory::Resource as u8, 1);
        assert_eq!(ErrorCategory::Api as u8, 2);
        assert_eq!(ErrorCategory::Data as u8, 3);
        assert_eq!(ErrorCategory::Logic as u8, 4);
        assert_eq!(ErrorCategory::Unknown as u8, 5);
    }

    // ========================================================================
    // ErrorSeverity Tests
    // ========================================================================

    #[test]
    fn test_error_severity_default() {
        let severity = ErrorSeverity::default();
        assert_eq!(severity, ErrorSeverity::Low);
    }

    #[test]
    fn test_error_severity_ordering() {
        assert_eq!(ErrorSeverity::Low as u8, 0);
        assert_eq!(ErrorSeverity::Medium as u8, 1);
        assert_eq!(ErrorSeverity::High as u8, 2);
        assert_eq!(ErrorSeverity::Critical as u8, 3);
    }

    // ========================================================================
    // RecoveryResult Tests
    // ========================================================================

    #[test]
    fn test_recovery_result_creation() {
        let result = RecoveryResult {
            action: RecoveryActionType::RestartService,
            success: true,
            timestamp: Instant::now(),
            details: "Service restarted successfully".to_string(),
            health_impact: HealthImpact {
                improvement: 1.0,
                resource_cost: 0.0,
                risk: 0.0,
            },
        };
        assert!(result.success);
        assert_eq!(result.details, "Service restarted successfully");
        assert_eq!(result.health_impact.improvement, 1.0);
    }

    #[test]
    fn test_recovery_result_failed() {
        let result = RecoveryResult {
            action: RecoveryActionType::AlertOperator("timeout".to_string()),
            success: false,
            timestamp: Instant::now(),
            details: "Failed to recover".to_string(),
            health_impact: HealthImpact::default(),
        };
        assert!(!result.success);
        assert_eq!(result.health_impact.risk, 0.0);
    }

    // ========================================================================
    // HealthImpact Tests
    // ========================================================================

    #[test]
    fn test_health_impact_default() {
        let impact = HealthImpact::default();
        assert_eq!(impact.improvement, 0.0);
        assert_eq!(impact.resource_cost, 0.0);
        assert_eq!(impact.risk, 0.0);
    }

    // ========================================================================
    // PerformanceRecovery Tests
    // ========================================================================

    #[test]
    fn test_performance_recovery_default() {
        let recovery = PerformanceRecovery::default();
        assert_eq!(recovery.tuning.target_level, 0.0);
    }

    // ========================================================================
    // ResourceRecovery Tests
    // ========================================================================

    #[test]
    fn test_resource_recovery_default() {
        let recovery = ResourceRecovery::default();
        assert_eq!(recovery.scaling.scale_up_threshold, 0.8);
        assert_eq!(recovery.cleanup.interval, Duration::from_secs(300));
    }
}
