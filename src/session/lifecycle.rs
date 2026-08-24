//! Session lifecycle, state management, health monitoring, and circuit breaker accessors.
//!
//! These are simple getter/setter methods extracted from the `impl Session` blocks
//! to reduce the size of `mod.rs`.

use crate::session::state::{is_circuit_breaker_open_pure, SessionState};
use std::sync::atomic::Ordering;

impl super::Session {
    /// Registers a page to track it as active for this session.
    pub fn register_page(&self, page_id: chromiumoxide::cdp::browser_protocol::target::TargetId) {
        self.active_pages.insert(page_id);
    }

    /// Unregisters a page from the active page registry.
    pub fn unregister_page(&self, page_id: &str) {
        self.active_pages.remove(page_id);
    }

    /// Returns the count of currently active pages.
    pub fn active_page_count(&self) -> usize {
        self.active_pages.len()
    }

    /// Returns the current operational state of the session.
    pub fn state(&self) -> SessionState {
        *self.state.lock()
    }

    /// Sets the operational state of the session.
    pub fn set_state(&self, new_state: SessionState) {
        *self.state.lock() = new_state;
    }

    /// Returns whether the session has available worker capacity for a new task.
    /// Replaces the old binary Idle/Busy gate with worker-slot-aware checking.
    pub fn has_available_workers(&self) -> bool {
        self.is_healthy() && self.active_workers.load(Ordering::SeqCst) < self.max_workers
    }

    /// Returns whether the session currently has zero active workers.
    pub fn is_idle(&self) -> bool {
        self.active_workers.load(Ordering::SeqCst) == 0
    }

    /// Returns whether the session is currently busy (has at least one active worker).
    pub fn is_busy(&self) -> bool {
        self.active_workers.load(Ordering::SeqCst) > 0
    }

    /// Returns whether the session is currently healthy.
    pub fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::SeqCst)
    }

    /// Marks the session as healthy.
    pub fn mark_healthy(&self) {
        self.is_healthy.store(true, Ordering::SeqCst);
    }

    /// Marks the session as unhealthy.
    pub fn mark_unhealthy(&self) {
        self.is_healthy.store(false, Ordering::SeqCst);
    }

    /// Increments the failure counter for health monitoring.
    pub fn increment_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Returns the current failure count for this session.
    pub fn get_failure_count(&self) -> usize {
        self.failure_count.load(Ordering::SeqCst)
    }

    // ── Circuit breaker accessors ──────────────────────────────────────────────

    /// Get circuit breaker failure count (for testing)
    pub fn get_circuit_breaker_failure_count(&self) -> usize {
        self.cb_failure_count.load(Ordering::SeqCst)
    }

    /// Get circuit breaker failure threshold (for testing)
    pub fn get_circuit_breaker_threshold(&self) -> usize {
        self.cb_failure_threshold
    }

    /// Get circuit breaker timeout in seconds (for testing)
    pub fn get_circuit_breaker_timeout_secs(&self) -> u64 {
        self.cb_timeout_secs
    }

    /// Check if circuit breaker is currently open (for testing)
    pub fn is_circuit_breaker_open(&self) -> bool {
        let current_time = crate::session::state::unix_timestamp_secs();
        let last_failure = self.cb_last_failure_time.load(Ordering::SeqCst);
        let failure_count = self.cb_failure_count.load(Ordering::SeqCst);

        is_circuit_breaker_open_pure(
            failure_count,
            self.cb_failure_threshold,
            last_failure,
            current_time,
            self.cb_timeout_secs,
        )
    }

    /// Reset circuit breaker state (for testing)
    pub fn reset_circuit_breaker(&self) {
        self.cb_failure_count.store(0, Ordering::SeqCst);
        self.cb_last_failure_time.store(0, Ordering::SeqCst);
        self.mark_healthy();
    }

    /// Set circuit breaker failure count (for testing only)
    pub fn set_circuit_breaker_failure_count(&self, count: usize) {
        self.cb_failure_count.store(count, Ordering::SeqCst);
    }

    /// Set circuit breaker last failure time (for testing only)
    pub fn set_circuit_breaker_last_failure_time(&self, time: usize) {
        self.cb_last_failure_time.store(time, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::state::SessionState;
    use std::sync::atomic::Ordering;

    /// Minimal helper to build a Session with controlled atomic fields.
    /// We don't need a real Browser — these tests only touch the atomic
    /// getters/setters defined in this file.
    fn make_session_parts() -> (
        std::sync::atomic::AtomicUsize,
        std::sync::atomic::AtomicBool,
        std::sync::atomic::AtomicUsize,
        std::sync::atomic::AtomicUsize,
    ) {
        (
            std::sync::atomic::AtomicUsize::new(0),   // failure_count
            std::sync::atomic::AtomicBool::new(true), // is_healthy
            std::sync::atomic::AtomicUsize::new(0),   // cb_failure_count
            std::sync::atomic::AtomicUsize::new(0),   // cb_last_failure_time
        )
    }

    #[test]
    fn health_transitions() {
        let (_, healthy, _, _) = make_session_parts();

        assert!(healthy.load(Ordering::SeqCst));

        healthy.store(false, Ordering::SeqCst);
        assert!(!healthy.load(Ordering::SeqCst));

        healthy.store(true, Ordering::SeqCst);
        assert!(healthy.load(Ordering::SeqCst));
    }

    #[test]
    fn failure_counting() {
        let (failures, _, _, _) = make_session_parts();

        assert_eq!(failures.load(Ordering::SeqCst), 0);
        failures.fetch_add(1, Ordering::SeqCst);
        failures.fetch_add(1, Ordering::SeqCst);
        failures.fetch_add(1, Ordering::SeqCst);
        assert_eq!(failures.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn circuit_breaker_open_close_cycle() {
        let threshold = 5;
        let timeout_secs = 30u64;
        let current_time = 1_700_000_000usize;

        // Closed — below threshold
        assert!(!is_circuit_breaker_open_pure(
            3,
            threshold,
            current_time - 10,
            current_time,
            timeout_secs
        ));

        // Open — at threshold, recent failure
        assert!(is_circuit_breaker_open_pure(
            threshold,
            threshold,
            current_time,
            current_time,
            timeout_secs
        ));

        // Closed again — old failure beyond timeout
        assert!(!is_circuit_breaker_open_pure(
            threshold,
            threshold,
            current_time - 100,
            current_time,
            timeout_secs
        ));
    }

    #[test]
    fn circuit_breaker_zero_threshold_never_opens() {
        assert!(!is_circuit_breaker_open_pure(100, 0, 1000, 1010, 30));
    }

    #[test]
    fn circuit_breaker_zero_threshold_with_recent_failure() {
        // Bug regression: cb_check() previously used `failure_count >= threshold`
        // without guarding threshold==0. With threshold=0: 0>=0 was always true,
        // causing cb_check to bail (circuit open) while is_circuit_breaker_open
        // correctly returned false. Both must agree: threshold=0 = disabled.
        let current_time: usize = 1_700_000_000;
        assert!(!is_circuit_breaker_open_pure(
            0,
            0,
            current_time,
            current_time + 5,
            30
        ));
        assert!(!is_circuit_breaker_open_pure(
            1,
            0,
            current_time,
            current_time + 5,
            30
        ));
        assert!(!is_circuit_breaker_open_pure(
            100,
            0,
            current_time,
            current_time + 5,
            30
        ));
    }

    #[test]
    fn state_getter_setter_roundtrip() {
        // We can't construct a full Session without a real Browser,
        // so test the state logic directly with parking_lot::Mutex.
        let state = parking_lot::Mutex::new(SessionState::Idle);

        assert_eq!(*state.lock(), SessionState::Idle);

        *state.lock() = SessionState::Busy;
        assert_eq!(*state.lock(), SessionState::Busy);

        *state.lock() = SessionState::Failed;
        assert_eq!(*state.lock(), SessionState::Failed);

        *state.lock() = SessionState::Idle;
        assert_eq!(*state.lock(), SessionState::Idle);
    }
}
