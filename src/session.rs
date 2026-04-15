use chromiumoxide::{Browser, Handler, Page};
use std::sync::Arc;
use tokio::sync::Semaphore;
use log::warn;
use anyhow;
use futures::StreamExt;

pub struct Session {
    pub id: String,
    pub name: String,
    #[allow(dead_code)]
    pub profile_type: String,
    pub browser: Browser,
    handler_task: Option<tokio::task::JoinHandle<()>>,

    // Worker semaphore (controls concurrent page access)
    worker_semaphore: Arc<Semaphore>,
    pub active_workers: std::sync::atomic::AtomicUsize,
    
    // Health tracking
    failure_count: std::sync::atomic::AtomicUsize,
    is_healthy: std::sync::atomic::AtomicBool,
}

impl Session {
    #[allow(dead_code)]
    pub fn new(
        id: String,
        name: String,
        profile_type: String,
        browser: Browser,
        handler: Handler,
        max_workers: usize,
    ) -> Self {
        let id_clone = id.clone();
        
        // Spawn handler polling task - keep it alive for the lifetime of the session
        let handler_task = tokio::spawn(async move {
            let mut handler = handler;
            loop {
                match handler.next().await {
                    Some(Ok(_)) => {
                        // Event received, continue
                    }
                    Some(Err(_)) => {
                        // Ignore errors and continue - keep handler alive
                    }
                    None => {
                        // Handler stream ended
                        break;
                    }
                }
            }
            log::debug!("Handler task ended for session {}", id_clone);
        });

        Self {
            id,
            name,
            profile_type,
            browser,
            handler_task: Some(handler_task),
            worker_semaphore: Arc::new(Semaphore::new(max_workers)),
            active_workers: std::sync::atomic::AtomicUsize::new(0),
            failure_count: std::sync::atomic::AtomicUsize::new(0),
            is_healthy: std::sync::atomic::AtomicBool::new(true),
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.is_healthy.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn mark_healthy(&self) {
        self.is_healthy.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn mark_unhealthy(&self) {
        self.is_healthy.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn increment_failure(&self) {
        self.failure_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn get_failure_count(&self) -> usize {
        self.failure_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Acquire a worker permit from the semaphore
    /// Equivalent to Node.js sessionManager.acquireWorker()
    pub async fn acquire_worker(&self, timeout_ms: u64) -> Option<tokio::sync::SemaphorePermit<'_>> {
        use tokio::time::{timeout, Duration};

        match timeout(
            Duration::from_millis(timeout_ms),
            self.worker_semaphore.acquire(),
        )
        .await
        {
            Ok(Ok(permit)) => {
                self.active_workers.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Some(permit)
            }
            Ok(Err(_)) => {
                warn!("[{}] Semaphore closed, cannot acquire worker", self.id);
                None
            }
            Err(_) => {
                warn!(
                    "[{}] Worker acquisition timeout after {}ms",
                    self.id, timeout_ms
                );
                None
            }
        }
    }

    #[allow(dead_code)]
    pub async fn release_worker(&self, _permit: tokio::sync::SemaphorePermit<'_>) {
        // Worker released via permit drop
    }

    /// Acquire a page for task execution
    /// Equivalent to Node.js sessionManager.acquirePage()
    pub async fn acquire_page(&self) -> anyhow::Result<Arc<chromiumoxide::Page>> {
        // Create new page (equivalent to browser.newPage())
        let page = self.browser.new_page("about:blank").await?;
        Ok(Arc::new(page))
    }

    /// Release a page (close it)
    /// Equivalent to Node.js sessionManager.releasePage()
    pub async fn release_page(&self, page: Arc<chromiumoxide::Page>) {
        // Close the page
        if let Err(e) = Arc::try_unwrap(page).unwrap().close().await {
            warn!("[{}] Error closing page: {}", self.id, e);
        }
    }
}