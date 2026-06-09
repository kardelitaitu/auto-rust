//! Health and formatting helpers for the orchestrator.
//!
//! Contains `format_duration()`, `broadcast_execution_count()`,
//! and `should_mark_session_unhealthy()` — extracted from the
//! monolith orchestrator.rs.

use crate::result::TaskErrorKind;

/// Formats milliseconds into a human-readable duration string.
///
/// Converts a duration in milliseconds to a concise, human-readable format:
/// - < 1000ms: "500ms"
/// - < 60s: "30s"
/// - < 1h: "5min" or "5min 30s"
/// - >= 1h: "2h" or "2h 15min"
///
/// # Arguments
///
/// * `ms` - Duration in milliseconds
///
/// # Returns
///
/// A human-readable duration string.
pub(super) fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms}ms")
    } else if ms < 60000 {
        let secs = ms / 1000;
        format!("{secs}s")
    } else if ms < 3600000 {
        let mins = ms / 60000;
        let secs = (ms % 60000) / 1000;
        if secs == 0 {
            format!("{mins}min")
        } else {
            format!("{mins}min {secs}s")
        }
    } else {
        let hours = ms / 3600000;
        let mins = (ms % 3600000) / 60000;
        if mins == 0 {
            format!("{hours}h")
        } else {
            format!("{hours}h {mins}min")
        }
    }
}

pub(super) fn broadcast_execution_count(task_count: usize, session_count: usize) -> usize {
    task_count.saturating_mul(session_count)
}

pub(super) fn should_mark_session_unhealthy(kind: TaskErrorKind, was_cancelled: bool) -> bool {
    !was_cancelled
        && matches!(
            kind,
            TaskErrorKind::Timeout
                | TaskErrorKind::Navigation
                | TaskErrorKind::Session
                | TaskErrorKind::Browser
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_comprehensive() {
        // Milliseconds
        assert_eq!(format_duration(0), "0ms");
        assert_eq!(format_duration(1), "1ms");
        assert_eq!(format_duration(500), "500ms");
        assert_eq!(format_duration(999), "999ms");

        // Seconds
        assert_eq!(format_duration(1000), "1s");
        assert_eq!(format_duration(5000), "5s");
        assert_eq!(format_duration(45000), "45s");
        assert_eq!(format_duration(59999), "59s");

        // Minutes
        assert_eq!(format_duration(60000), "1min");
        assert_eq!(format_duration(65000), "1min 5s");
        assert_eq!(format_duration(120000), "2min");
        assert_eq!(format_duration(125000), "2min 5s");
        assert_eq!(format_duration(3599999), "59min 59s");

        // Hours
        assert_eq!(format_duration(3600000), "1h");
        assert_eq!(format_duration(3660000), "1h 1min");
        assert_eq!(format_duration(7200000), "2h");
        assert_eq!(format_duration(7320000), "2h 2min");
        assert_eq!(format_duration(36000000), "10h");
    }

    #[test]
    fn test_broadcast_execution_count_comprehensive() {
        // Edge cases
        assert_eq!(broadcast_execution_count(0, 0), 0);
        assert_eq!(broadcast_execution_count(0, 5), 0);
        assert_eq!(broadcast_execution_count(5, 0), 0);
        assert_eq!(broadcast_execution_count(1, 1), 1);

        // Normal cases
        assert_eq!(broadcast_execution_count(3, 4), 12);
        assert_eq!(broadcast_execution_count(10, 1), 10);
        assert_eq!(broadcast_execution_count(1, 10), 10);
        assert_eq!(broadcast_execution_count(100, 50), 5000);

        // Large numbers
        assert_eq!(broadcast_execution_count(1000, 1000), 1000000);
    }

    #[test]
    fn test_should_mark_session_unhealthy_comprehensive() {
        // Should mark unhealthy for these (when not cancelled)
        assert!(should_mark_session_unhealthy(TaskErrorKind::Timeout, false));
        assert!(should_mark_session_unhealthy(
            TaskErrorKind::Navigation,
            false
        ));
        assert!(should_mark_session_unhealthy(TaskErrorKind::Session, false));
        assert!(should_mark_session_unhealthy(TaskErrorKind::Browser, false));

        // Should NOT mark unhealthy for these
        assert!(!should_mark_session_unhealthy(
            TaskErrorKind::Validation,
            false
        ));
        assert!(!should_mark_session_unhealthy(
            TaskErrorKind::Unknown,
            false
        ));

        // Cancelled tasks should NEVER mark unhealthy
        assert!(!should_mark_session_unhealthy(TaskErrorKind::Timeout, true));
        assert!(!should_mark_session_unhealthy(
            TaskErrorKind::Navigation,
            true
        ));
        assert!(!should_mark_session_unhealthy(TaskErrorKind::Session, true));
        assert!(!should_mark_session_unhealthy(TaskErrorKind::Browser, true));
        assert!(!should_mark_session_unhealthy(
            TaskErrorKind::Validation,
            true
        ));
        assert!(!should_mark_session_unhealthy(TaskErrorKind::Unknown, true));
    }
}
