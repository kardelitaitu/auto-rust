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
