//! Current recovery state.

// last audited 26-06-26 by Buffy

use std::time::{Duration, Instant};

/// Recovery mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryMode {
    Normal,
    Recovering,
    Degraded,
    Emergency,
}

/// Type of recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryType {
    Automatic,
    Manual,
    Hybrid,
}

/// Recovery status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryStatus {
    Initiated,
    InProgress,
    Completed,
    Failed,
    Pending,
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
    #[must_use]
    pub fn new() -> Self {
        Self {
            mode: RecoveryMode::Normal,
            active_recoveries: vec![],
            progress: RecoveryProgress::default(),
        }
    }
}

impl Default for RecoveryState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // -------------------------------------------------------------------------
    // RecoveryMode enum
    // -------------------------------------------------------------------------

    #[test]
    fn recovery_mode_variants_exist() {
        let modes = [
            RecoveryMode::Normal,
            RecoveryMode::Recovering,
            RecoveryMode::Degraded,
            RecoveryMode::Emergency,
        ];
        assert_eq!(modes.len(), 4);

        // Equality checks
        assert_eq!(RecoveryMode::Normal, RecoveryMode::Normal);
        assert_ne!(RecoveryMode::Normal, RecoveryMode::Recovering);
    }

    // -------------------------------------------------------------------------
    // RecoveryType enum
    // -------------------------------------------------------------------------

    #[test]
    fn recovery_type_variants_exist() {
        let t1 = RecoveryType::Automatic;
        let t2 = RecoveryType::Manual;
        let t3 = RecoveryType::Hybrid;
        assert_ne!(t1, t2);
        assert_ne!(t1, t3);
        assert_ne!(t2, t3);
    }

    #[test]
    fn recovery_type_is_cloneable() {
        let t = RecoveryType::Automatic;
        let c = t.clone();
        assert_eq!(t, c);
    }

    // -------------------------------------------------------------------------
    // RecoveryStatus enum
    // -------------------------------------------------------------------------

    #[test]
    fn recovery_status_variants_exist() {
        let statuses = [
            RecoveryStatus::Initiated,
            RecoveryStatus::InProgress,
            RecoveryStatus::Completed,
            RecoveryStatus::Failed,
            RecoveryStatus::Pending,
        ];
        assert_eq!(statuses.len(), 5);
        assert_eq!(RecoveryStatus::Initiated, RecoveryStatus::Initiated);
        assert_ne!(RecoveryStatus::Pending, RecoveryStatus::Completed);
    }

    // -------------------------------------------------------------------------
    // ActiveRecovery
    // -------------------------------------------------------------------------

    #[test]
    fn active_recovery_construction() {
        let ar = ActiveRecovery {
            id: "recovery-1".to_string(),
            recovery_type: RecoveryType::Automatic,
            start_time: Instant::now(),
            estimated_completion: Some(Instant::now() + Duration::from_secs(30)),
            status: RecoveryStatus::InProgress,
        };

        assert_eq!(ar.id, "recovery-1");
        assert_eq!(ar.recovery_type, RecoveryType::Automatic);
        assert_eq!(ar.status, RecoveryStatus::InProgress);
        assert!(ar.estimated_completion.is_some());
    }

    #[test]
    fn active_recovery_no_estimated_completion() {
        let ar = ActiveRecovery {
            id: "recovery-2".to_string(),
            recovery_type: RecoveryType::Manual,
            start_time: Instant::now(),
            estimated_completion: None,
            status: RecoveryStatus::Initiated,
        };

        assert!(ar.estimated_completion.is_none());
        assert_eq!(ar.status, RecoveryStatus::Initiated);
    }

    #[test]
    fn active_recovery_is_cloneable_and_debuggable() {
        let ar = ActiveRecovery {
            id: "r1".to_string(),
            recovery_type: RecoveryType::Hybrid,
            start_time: Instant::now(),
            estimated_completion: None,
            status: RecoveryStatus::Failed,
        };
        let cloned = ar.clone();
        assert_eq!(ar.id, cloned.id);
        assert_eq!(ar.status, cloned.status);
        let _ = format!("{:?}", ar);
    }

    // -------------------------------------------------------------------------
    // RecoveryProgress
    // -------------------------------------------------------------------------

    #[test]
    fn recovery_progress_default_values() {
        let p = RecoveryProgress::default();
        assert_eq!(p.current_step, 0);
        assert_eq!(p.total_steps, 1);
        assert_eq!(p.completion, 0.0);
        assert_eq!(p.estimated_remaining, Duration::from_secs(0));
    }

    #[test]
    fn recovery_progress_partial_progress() {
        let p = RecoveryProgress {
            current_step: 3,
            total_steps: 10,
            completion: 0.3,
            estimated_remaining: Duration::from_secs(20),
        };

        assert_eq!(p.current_step, 3);
        assert_eq!(p.total_steps, 10);
        assert!((p.completion - 0.3).abs() < f32::EPSILON);
        assert_eq!(p.estimated_remaining, Duration::from_secs(20));
    }

    #[test]
    fn recovery_progress_full_completion() {
        let p = RecoveryProgress {
            current_step: 5,
            total_steps: 5,
            completion: 1.0,
            estimated_remaining: Duration::from_secs(0),
        };

        assert_eq!(p.completion, 1.0);
        assert_eq!(p.current_step, p.total_steps);
    }

    #[test]
    fn recovery_progress_each_field_accessible() {
        let p = RecoveryProgress {
            current_step: 2,
            total_steps: 4,
            completion: 0.5,
            estimated_remaining: Duration::from_secs(10),
        };
        assert_eq!(p.current_step, 2);
        assert_eq!(p.total_steps, 4);
        assert!((p.completion - 0.5).abs() < f32::EPSILON);
        assert_eq!(p.estimated_remaining, Duration::from_secs(10));
    }

    // -------------------------------------------------------------------------
    // RecoveryState
    // -------------------------------------------------------------------------

    #[test]
    fn recovery_state_new_has_normal_mode() {
        let state = RecoveryState::new();
        assert_eq!(state.mode, RecoveryMode::Normal);
        assert!(state.active_recoveries.is_empty());
    }

    #[test]
    fn recovery_state_default_same_as_new() {
        let a = RecoveryState::new();
        let b = RecoveryState::default();
        assert_eq!(a.mode, b.mode);
        assert_eq!(a.active_recoveries.len(), b.active_recoveries.len());
    }

    #[test]
    fn recovery_state_can_store_active_recoveries() {
        let mut state = RecoveryState::new();
        let ar = ActiveRecovery {
            id: "ar-1".to_string(),
            recovery_type: RecoveryType::Automatic,
            start_time: Instant::now(),
            estimated_completion: None,
            status: RecoveryStatus::InProgress,
        };
        state.active_recoveries.push(ar.clone());

        assert_eq!(state.active_recoveries.len(), 1);
        assert_eq!(state.active_recoveries[0].id, "ar-1");
    }

    #[test]
    fn recovery_state_default_progress_fields() {
        let state = RecoveryState::new();
        assert_eq!(state.progress.current_step, 0);
        assert_eq!(state.progress.total_steps, 1);
        assert_eq!(state.progress.completion, 0.0);
    }

    #[test]
    fn recovery_state_mode_can_mutate() {
        let mut state = RecoveryState::new();
        state.mode = RecoveryMode::Emergency;
        assert_eq!(state.mode, RecoveryMode::Emergency);

        state.mode = RecoveryMode::Degraded;
        assert_eq!(state.mode, RecoveryMode::Degraded);
    }

    #[test]
    fn recovery_state_is_cloneable() {
        let s1 = RecoveryState::new();
        let s2 = s1.clone();
        assert_eq!(s1.mode, s2.mode);
        assert_eq!(s1.active_recoveries.len(), s2.active_recoveries.len());
        assert_eq!(s1.progress.completion, s2.progress.completion);
    }

    // -------------------------------------------------------------------------
    // Debug / display
    // -------------------------------------------------------------------------

    #[test]
    fn all_state_enums_debuggable() {
        let _ = format!("{:?}", RecoveryMode::Normal);
        let _ = format!("{:?}", RecoveryType::Automatic);
        let _ = format!("{:?}", RecoveryStatus::Completed);
    }

    #[test]
    fn recovery_state_struct_debuggable() {
        let state = RecoveryState::new();
        let _ = format!("{:?}", state);
    }

    #[test]
    fn active_recovery_struct_debuggable() {
        let ar = ActiveRecovery {
            id: "x".to_string(),
            recovery_type: RecoveryType::Manual,
            start_time: Instant::now(),
            estimated_completion: None,
            status: RecoveryStatus::Pending,
        };
        let _ = format!("{:?}", ar);
    }
}
