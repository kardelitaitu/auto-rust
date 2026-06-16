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
