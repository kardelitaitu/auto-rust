//! Session state types and pure circuit breaker helpers.
//!
//! Extracted from `session/mod.rs` — spec 0019.

/// Represents the current operational state of a browser session.
/// Used to track session health and availability for task assignment.
///
/// `#[non_exhaustive]` — match with wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionState {
    /// Session is available and ready to accept tasks
    Idle,
    /// Session is currently executing a task
    Busy,
    /// Session has failed and is not available for tasks
    Failed,
}

/// Returns the current Unix timestamp in seconds, using a safe fallback.
/// Uses `unwrap_or_default()` instead of `expect()` to avoid panicking
/// if the system clock is set before UNIX epoch.
pub(crate) fn unix_timestamp_secs() -> usize {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as usize
}

/// Pure function to determine if circuit breaker should be open.
/// This logic is extracted for testability without requiring `SystemTime` calls.
#[must_use]
pub fn is_circuit_breaker_open_pure(
    failure_count: usize,
    failure_threshold: usize,
    last_failure_time: usize,
    current_time: usize,
    timeout_secs: u64,
) -> bool {
    if failure_threshold == 0 {
        return false; // No threshold means circuit never opens
    }

    failure_count >= failure_threshold
        && current_time.saturating_sub(last_failure_time) < timeout_secs as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_state_debug_all_variants() {
        assert!(format!("{:?}", SessionState::Idle).contains("Idle"));
        assert!(format!("{:?}", SessionState::Busy).contains("Busy"));
        assert!(format!("{:?}", SessionState::Failed).contains("Failed"));
    }

    #[test]
    fn session_state_eq_and_ne() {
        assert_eq!(SessionState::Idle, SessionState::Idle);
        assert_ne!(SessionState::Idle, SessionState::Busy);
        assert_ne!(SessionState::Busy, SessionState::Failed);
    }

    #[test]
    fn session_state_is_copy() {
        let s = SessionState::Idle;
        let s2 = s;
        assert_eq!(s, s2);
    }

    #[test]
    fn unix_timestamp_secs_returns_reasonable_value() {
        let ts = unix_timestamp_secs();
        // Should be after 2020-01-01 (1577836800)
        assert!(ts > 1_577_836_800, "timestamp {ts} seems too old");
        // Should be before 2100-01-01 (4102444800)
        assert!(
            ts < 4_102_444_800,
            "timestamp {ts} seems too far in the future"
        );
    }

    #[test]
    fn circuit_breaker_open_below_threshold() {
        assert!(!is_circuit_breaker_open_pure(3, 5, 1000, 1500, 30));
    }

    #[test]
    fn circuit_breaker_open_at_threshold_recent() {
        assert!(is_circuit_breaker_open_pure(5, 5, 1000, 1010, 30));
    }

    #[test]
    fn circuit_breaker_open_above_threshold() {
        assert!(is_circuit_breaker_open_pure(10, 5, 1000, 1010, 30));
    }

    #[test]
    fn circuit_breaker_closed_after_timeout() {
        assert!(!is_circuit_breaker_open_pure(5, 5, 1000, 1060, 30));
    }

    #[test]
    fn circuit_breaker_closed_zero_failures() {
        assert!(!is_circuit_breaker_open_pure(0, 5, 0, 1000, 30));
    }

    #[test]
    fn circuit_breaker_zero_threshold_never_opens() {
        assert!(!is_circuit_breaker_open_pure(100, 0, 1000, 1010, 30));
    }

    #[test]
    fn circuit_breaker_exact_timeout_boundary() {
        // Exactly at boundary: diff == timeout -> NOT open (strict <)
        assert!(!is_circuit_breaker_open_pure(5, 5, 970, 1000, 30));
    }

    #[test]
    fn circuit_breaker_wraparound_saturating_sub() {
        // last_failure > current_time -> saturating_sub returns 0 -> open
        assert!(is_circuit_breaker_open_pure(5, 5, usize::MAX - 10, 100, 30));
    }
}
