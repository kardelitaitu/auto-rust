//! Current recovery state.

use std::time::{Duration, Instant};

/// Recovery mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryMode {
    Normal, Recovering, Degraded, Emergency,
}

/// Type of recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryType {
    Automatic, Manual, Hybrid,
}

/// Recovery status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryStatus {
    Initiated, InProgress, Completed, Failed, Pending,
}

/// Active recovery in progress.
#[derive(Debug, Clone)]
pub struct ActiveRecovery {
    pub id: String,
    pub recovery_type: RecoveryType,
    pub start_time: Instant,
    pub estimated_completion: Option<Instant>,
    pub status: RecoveryStatus,
}

/// Recovery progress tracking.
#[derive(Debug, Clone)]
pub struct RecoveryProgress {
    pub current_step: u32,
    pub total_steps: u32,
    pub completion: f32,
    pub estimated_remaining: Duration,
}

impl Default for RecoveryProgress {
    fn default() -> Self {
        Self {
            current_step: 0,
            total_steps: 1,
            completion: 0.0,
            estimated_remaining: Duration::from_secs(0),
        }
    }
}

/// Current recovery state.
#[derive(Debug, Clone)]
pub struct RecoveryState {
    pub mode: RecoveryMode,
    pub active_recoveries: Vec<ActiveRecovery>,
    pub progress: RecoveryProgress,
}

impl RecoveryState {
    pub fn new() -> Self {
        Self {
            mode: RecoveryMode::Normal,
            active_recoveries: vec![],
            progress: RecoveryProgress::default(),
        }
    }
}

impl Default for RecoveryState {
    fn default() -> Self { Self::new() }
}
