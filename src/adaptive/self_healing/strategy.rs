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
