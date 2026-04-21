use log::warn;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{timeout, Duration};

pub struct WorkerPool {
    session_id: String,
    semaphore: Arc<Semaphore>,
    active_workers: std::sync::atomic::AtomicUsize,
}

impl WorkerPool {
    pub fn new(session_id: String, max_workers: usize) -> Self {
        Self {
            session_id,
            semaphore: Arc::new(Semaphore::new(max_workers)),
            active_workers: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn active_count(&self) -> usize {
        self.active_workers
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub async fn acquire(&self, timeout_ms: u64) -> Option<tokio::sync::SemaphorePermit<'_>> {
        match timeout(Duration::from_millis(timeout_ms), self.semaphore.acquire()).await {
            Ok(Ok(permit)) => {
                self.active_workers
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Some(permit)
            }
            Ok(Err(_)) => {
                warn!("[{}] Semaphore closed", self.session_id);
                None
            }
            Err(_) => {
                warn!("[{}] Worker acquire timeout", self.session_id);
                None
            }
        }
    }

    pub fn release(&self, _permit: tokio::sync::SemaphorePermit<'_>) {}
}
