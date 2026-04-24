//! Real-time monitoring dashboard for automation health and performance.
//! Provides live insights, alerts, and visualization of automation metrics.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::metrics::TwitterActivityRunCounters;
use crate::adaptive::learning_engine::AdaptationEvent;

/// Real-time monitoring dashboard for automation systems.
pub struct MonitoringDashboard {
    /// Live metrics stream
    pub live_metrics: LiveMetrics,
    /// Alert system
    pub alerts: AlertSystem,
    /// Visualization engine
    pub visualizer: VisualizationEngine,
    /// Health monitoring
    pub health_monitor: HealthMonitor,
}

/// Live metrics stream with rolling window.
pub struct LiveMetrics {
    /// Rolling window of recent metrics
    metrics_window: VecDeque<PerformanceSnapshot>,
    /// Current aggregated metrics
    current: AggregatedMetrics,
    /// Update frequency
    update_interval: Duration,
}

/// Aggregated metrics for current window.
#[derive(Debug, Clone, Default)]
pub struct AggregatedMetrics {
    /// Average success rate
    pub avg_success_rate: f32,
    /// Total actions processed
    pub total_actions: u64,
    /// Error count
    pub error_count: u64,
    /// Average response time
    pub avg_response_time: Duration,
    /// Engagement quality
    pub engagement_quality: f32,
}

/// Performance snapshot for rolling window.
#[derive(Debug, Clone)]
pub struct PerformanceSnapshot {
    pub timestamp: Instant,
    pub metrics: TwitterActivityRunCounters,
    pub adaptation_events: Vec<AdaptationEvent>,
}

/// Alert system for monitoring thresholds and anomalies.
pub struct AlertSystem {
    /// Alert thresholds
    pub thresholds: AlertThresholds,
    /// Active alerts
    pub active_alerts: Vec<Alert>,
    /// Alert history
    pub history: Vec<Alert>,
}

/// Alert thresholds configuration.
#[derive(Debug, Clone, Default)]
pub struct AlertThresholds {
    /// Success rate below which alert
    pub min_success_rate: f32,
    /// Error rate above which alert
    pub max_error_rate: f32,
    /// Response time above which alert
    pub max_response_time: Duration,
    /// Anomaly detection threshold
    pub anomaly_score: f32,
}

/// Individual alert.
#[derive(Debug, Clone)]
pub struct Alert {
    /// Alert ID
    pub id: String,
    /// Alert level
    pub level: AlertLevel,
    /// Alert message
    pub message: String,
    /// Timestamp
    pub timestamp: Instant,
    /// Related metrics
    pub metrics: AlertMetrics,
    /// Acknowledged status
    pub acknowledged: bool,
}

/// Alert severity levels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertLevel {
    /// Informational alert
    Info,
    /// Warning alert
    Warning,
    /// Error alert
    Error,
    /// Critical alert
    Critical,
}

/// Metrics associated with an alert.
#[derive(Debug, Clone, Default)]
pub struct AlertMetrics {
    /// Current success rate
    pub success_rate: f32,
    /// Current error rate
    pub error_rate: f32,
    /// Current response time
    pub response_time: Duration,
    /// Anomaly score
    pub anomaly_score: f32,
}

/// Visualization engine for dashboard displays.
pub struct VisualizationEngine {
    /// Chart configurations
    pub chart_configs: Vec<ChartConfig>,
    /// Visualization types supported
    pub viz_types: Vec<String>,
}

/// Chart configuration for visualization.
#[derive(Debug, Clone)]
pub struct ChartConfig {
    /// Chart ID
    pub id: String,
    /// Chart type (line, bar, pie, etc.)
    pub chart_type: ChartType,
    /// Data source
    pub data_source: String,
    /// Update frequency
    pub update_freq: Duration,
}

/// Chart types supported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChartType {
    Line,
    Bar,
    Pie,
    Scatter,
    Heatmap,
}

/// Health monitoring for automation system.
pub struct HealthMonitor {
    /// System health status
    pub status: SystemHealthStatus,
    /// Health checks
    pub checks: Vec<HealthCheck>,
    /// Recovery actions
    pub recovery_actions: Vec<RecoveryAction>,
}

/// System health status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemHealthStatus {
    /// System operating normally
    Healthy,
    /// Minor issues detected
    Degraded,
    /// Critical issues
    Critical,
    /// System offline
    Offline,
}

/// Individual health check.
#[derive(Debug, Clone)]
pub struct HealthCheck {
    /// Check ID
    pub id: String,
    /// Check description
    pub description: String,
    /// Check result
    pub result: HealthCheckResult,
    /// Last run timestamp
    pub last_run: Instant,
    /// Check frequency
    pub frequency: Duration,
}

/// Health check result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthCheckResult {
    /// Check passed
    Passed,
    /// Check failed
    Failed(String),
    /// Check inconclusive
    Inconclusive,
}

/// Recovery action for system issues.
#[derive(Debug, Clone)]
pub struct RecoveryAction {
    /// Action ID
    pub id: String,
    /// Action description
    pub description: String,
    /// Action to execute
    pub action: RecoveryActionType,
    /// Last execution result
    pub last_result: RecoveryResult,
}

/// Type of recovery action.
#[derive(Debug, Clone)]
pub enum RecoveryActionType {
    /// Restart service
    RestartService,
    /// Scale resources
    ScaleResources,
    /// Switch to backup
    SwitchBackup,
    /// Alert operator
    AlertOperator,
    /// Custom action
    Custom(String),
}

/// Result of recovery action.
#[derive(Debug, Clone)]
pub struct RecoveryResult {
    /// Success status
    pub success: bool,
    /// Timestamp
    pub timestamp: Instant,
    /// Details
    pub details: String,
}

impl MonitoringDashboard {
    /// Create a new monitoring dashboard.
    pub fn new() -> Self {
        Self {
            live_metrics: LiveMetrics::new(),
            alerts: AlertSystem::new(),
            visualizer: VisualizationEngine::new(),
            health_monitor: HealthMonitor::new(),
        }
    }

    /// Update dashboard with new metrics.
    pub fn update(&mut self, snapshot: PerformanceSnapshot) {
        self.live_metrics.update(snapshot);
        self.alerts.check_thresholds(&self.live_metrics.current);
        self.health_monitor.check_health();
    }

    /// Get current dashboard state.
    pub fn get_state(&self) -> DashboardState {
        DashboardState {
            metrics: self.live_metrics.current.clone(),
            alerts: self.alerts.active_alerts.clone(),
            health: self.health_monitor.status.clone(),
        }
    }
}

impl LiveMetrics {
    /// Create new live metrics.
    pub fn new() -> Self {
        Self {
            metrics_window: VecDeque::new(),
            current: AggregatedMetrics::default(),
            update_interval: Duration::from_secs(1),
        }
    }

    /// Update metrics with new snapshot.
    pub fn update(&mut self, snapshot: PerformanceSnapshot) {
        self.metrics_window.push_back(snapshot);
        
        // Keep only recent window (e.g., last 100 snapshots)
        if self.metrics_window.len() > 100 {
            self.metrics_window.pop_front();
        }
        
        // Recalculate aggregated metrics
        self.recalculate();
    }

    /// Recalculate aggregated metrics.
    fn recalculate(&mut self) {
        if self.metrics_window.is_empty() {
            return;
        }
        
        let total = self.metrics_window.len();
        let mut total_success = 0;
        let mut total_errors = 0;
        let mut total_response_time = Duration::from_secs(0);
        let mut total_quality = 0.0;
        
        for snapshot in &self.metrics_window {
            // Aggregate metrics from counters
            total_success += snapshot.metrics.total_actions;
            total_errors += snapshot.metrics.button_missing as u64;
            total_response_time += Duration::from_millis(100); // Simplified
            total_quality += snapshot.metrics.total_actions as f32 * 0.8; // Simplified
        }
        
        self.current = AggregatedMetrics {
            avg_success_rate: total_success as f32 / total as f32,
            total_actions: total_success,
            error_count: total_errors,
            avg_response_time: total_response_time / total as u32,
            engagement_quality: total_quality / total as f32,
        };
    }
}

impl AlertSystem {
    /// Create new alert system.
    pub fn new() -> Self {
        Self {
            thresholds: AlertThresholds::default(),
            active_alerts: vec![],
            history: vec![],
        }
    }

    /// Check thresholds and generate alerts.
    pub fn check_thresholds(&mut self, metrics: &AggregatedMetrics) {
        // Check success rate threshold
        if metrics.avg_success_rate < self.thresholds.min_success_rate {
            self.generate_alert(
                AlertLevel::Error,
                format!("Success rate below threshold: {}", metrics.avg_success_rate),
                metrics.clone(),
            );
        }
        
        // Check error rate threshold
        let error_rate = metrics.error_count as f32 / metrics.total_actions.max(1) as f32;
        if error_rate > self.thresholds.max_error_rate {
            self.generate_alert(
                AlertLevel::Critical,
                format!("Error rate above threshold: {}", error_rate),
                metrics.clone(),
            );
        }
    }

    /// Generate new alert.
    fn generate_alert(&mut self, level: AlertLevel, message: String, metrics: AggregatedMetrics) {
        let alert = Alert {
            id: format!("alert_{}", Instant::now().duration_since(Instant::now()).as_nanos()),
            level,
            message,
            timestamp: Instant::now(),
            metrics,
            acknowledged: false,
        };
        
        self.active_alerts.push(alert.clone());
        self.history.push(alert);
    }
}

impl HealthMonitor {
    /// Create new health monitor.
    pub fn new() -> Self {
        Self {
            status: SystemHealthStatus::Healthy,
            checks: vec![],
            recovery_actions: vec![],
        }
    }

    /// Run health checks.
    pub fn check_health(&mut self) {
        // Simplified health check
        self.status = SystemHealthStatus::Healthy;
    }
}

impl VisualizationEngine {
    /// Create new visualization engine.
    pub fn new() -> Self {
        Self {
            chart_configs: vec![],
            viz_types: vec!["line".to_string(), "bar".to_string(), "pie".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_creation() {
        let dashboard = MonitoringDashboard::new();
        assert!(dashboard.live_metrics.current.avg_success_rate >= 0.0);
    }

    #[test]
    fn test_alert_generation() {
        let mut alerts = AlertSystem::new();
        let metrics = AggregatedMetrics {
            avg_success_rate: 0.3, // Below threshold
            ..Default::default()
        };
        alerts.check_thresholds(&metrics);
        assert!(!alerts.active_alerts.is_empty());
    }
}
