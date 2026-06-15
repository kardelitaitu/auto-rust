//! Worker permit type for session concurrency control.
//!
//! Extracted from `session/mod.rs` — spec 0019.

use tokio::sync::SemaphorePermit;

/// Represents a browser session with connection management and health monitoring.
/// A session encapsulates a browser instance and manages its lifecycle, worker allocation,
/// and health status for reliable task execution.
pub struct WorkerPermit<'a> {
    _permit: SemaphorePermit<'a>,
    active_workers: &'a std::sync::atomic::AtomicUsize,
}

impl<'a> WorkerPermit<'a> {
    /// Create a new `WorkerPermit` from a semaphore permit and active worker counter.
    pub(crate) fn new(
        permit: SemaphorePermit<'a>,
        active_workers: &'a std::sync::atomic::AtomicUsize,
    ) -> Self {
        Self {
            _permit: permit,
            active_workers,
        }
    }
}

impl Drop for WorkerPermit<'_> {
    fn drop(&mut self) {
        self.active_workers
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}
