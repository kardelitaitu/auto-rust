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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn worker_permit_drop_decrements_counter() {
        let semaphore = tokio::sync::Semaphore::new(5);
        let active = AtomicUsize::new(0);
        let permit = semaphore.acquire().await.unwrap();
        active.fetch_add(1, Ordering::SeqCst);

        assert_eq!(active.load(Ordering::SeqCst), 1);

        let worker_permit = WorkerPermit::new(permit, &active);
        assert_eq!(active.load(Ordering::SeqCst), 1);

        drop(worker_permit);
        assert_eq!(active.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn worker_permit_multiple_permits() {
        let semaphore = tokio::sync::Semaphore::new(5);
        let active = AtomicUsize::new(0);

        let p1 = semaphore.acquire().await.unwrap();
        active.fetch_add(1, Ordering::SeqCst);
        let wp1 = WorkerPermit::new(p1, &active);

        let p2 = semaphore.acquire().await.unwrap();
        active.fetch_add(1, Ordering::SeqCst);
        let wp2 = WorkerPermit::new(p2, &active);

        assert_eq!(active.load(Ordering::SeqCst), 2);

        drop(wp1);
        assert_eq!(active.load(Ordering::SeqCst), 1);

        drop(wp2);
        assert_eq!(active.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn worker_permit_drop_is_idempotent_for_counter() {
        let semaphore = tokio::sync::Semaphore::new(1);
        let active = AtomicUsize::new(0);
        let permit = semaphore.acquire().await.unwrap();
        active.fetch_add(1, Ordering::SeqCst);

        let wp = WorkerPermit::new(permit, &active);
        assert_eq!(active.load(Ordering::SeqCst), 1);

        drop(wp);
        assert_eq!(active.load(Ordering::SeqCst), 0);
    }
}
