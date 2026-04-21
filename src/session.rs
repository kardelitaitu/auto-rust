//! Browser session lifecycle management module.
//!
//! Manages individual browser sessions including:
//! - Session creation and initialization
//! - Worker/page allocation with semaphore-based concurrency control
//! - Health monitoring and failure tracking
//! - Graceful shutdown and cleanup

use crate::internal::profile::{random_preset, randomize_profile, BrowserProfile, ProfileRuntime};
use crate::state::{bind_page_overlay, unbind_page_overlay, SessionOverlayState};
use chromiumoxide::cdp::browser_protocol::target::TargetId;
use chromiumoxide::{Browser, Handler};
use dashmap::DashSet;
use futures::StreamExt;
use log::{info, warn};
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Represents the current operational state of a browser session.
/// Used to track session health and availability for task assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SessionState {
    /// Session is available and ready to accept tasks
    Idle,
    /// Session is currently executing a task
    Busy,
    /// Session has failed and is not available for tasks
    Failed,
}

/// Represents a browser session with connection management and health monitoring.
/// A session encapsulates a browser instance and manages its lifecycle, worker allocation,
/// and health status for reliable task execution.
pub struct Session {
    /// Unique identifier for this session
    pub id: String,
    /// Human-readable name for this session
    pub name: String,
    /// Browser profile type (e.g., "chrome", "brave")
    /// Stored for logging/debugging purposes
    pub profile_type: String,
    /// Behavioral profile for human-like interactions (cursor, typing, etc.)
    pub behavior_profile: BrowserProfile,
    /// Session-stable derived behavior snapshot.
    pub behavior_runtime: ProfileRuntime,
    /// The underlying Chromium Oxide browser instance
    pub browser: Browser,
    /// Background task handle for event handling (internal use)
    #[allow(dead_code)]
    handler_task: Option<tokio::task::JoinHandle<()>>,
    /// Cursor overlay sync interval (0 = disabled)
    pub cursor_overlay_ms: u64,
    /// Session-owned cursor overlay state
    overlay_state: Arc<SessionOverlayState>,
    /// Background overlay synchronizer bound to this session
    overlay_task: Option<tokio::task::JoinHandle<()>>,

    /// Semaphore controlling concurrent page access within this session
    worker_semaphore: Arc<Semaphore>,
    /// Number of currently active worker threads/pages
    pub active_workers: std::sync::atomic::AtomicUsize,

    /// Count of consecutive failures for health monitoring
    failure_count: std::sync::atomic::AtomicUsize,
    /// Whether this session is considered healthy for task execution
    is_healthy: std::sync::atomic::AtomicBool,

    /// Current operational state of the session
    state: parking_lot::Mutex<SessionState>,

    /// Registry of active page IDs (currently unused)
    #[allow(dead_code)]
    active_pages: DashSet<TargetId>,
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
        cursor_overlay_ms: u64,
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
            log::debug!("Handler task ended for session {id_clone}");
        });

        let behavior_profile = randomize_profile(&random_preset());
        let behavior_runtime = behavior_profile.runtime();
        let overlay_state = Arc::new(SessionOverlayState::new(cursor_overlay_ms > 0));
        let overlay_task = if cursor_overlay_ms > 0 {
            let overlay_for_task = overlay_state.clone();
            let session_id_for_overlay = id.clone();
            Some(tokio::spawn(async move {
                crate::utils::mouse::run_cursor_overlay_background(
                    overlay_for_task,
                    cursor_overlay_ms,
                    session_id_for_overlay,
                )
                .await;
            }))
        } else {
            None
        };

        Self {
            id,
            name,
            profile_type,
            behavior_profile,
            behavior_runtime,
            browser,
            handler_task: Some(handler_task),
            cursor_overlay_ms,
            overlay_state,
            overlay_task,
            worker_semaphore: Arc::new(Semaphore::new(max_workers)),
            active_workers: std::sync::atomic::AtomicUsize::new(0),
            failure_count: std::sync::atomic::AtomicUsize::new(0),
            is_healthy: std::sync::atomic::AtomicBool::new(true),
            state: parking_lot::Mutex::new(SessionState::Idle),
            active_pages: DashSet::new(),
        }
    }

    /// Register a page to track it
    #[allow(dead_code)]
    pub fn register_page(&self, page_id: TargetId) {
        self.active_pages.insert(page_id);
    }

    /// Unregister a page (release it)
    #[allow(dead_code)]
    pub fn unregister_page(&self, page_id: &str) {
        self.active_pages.remove(page_id);
    }

    /// Get count of active pages
    #[allow(dead_code)]
    pub fn active_page_count(&self) -> usize {
        self.active_pages.len()
    }

    /// State management
    #[allow(dead_code)]
    pub fn state(&self) -> SessionState {
        *self.state.lock()
    }

    #[allow(dead_code)]
    pub fn set_state(&self, new_state: SessionState) {
        *self.state.lock() = new_state;
    }

    #[allow(dead_code)]
    pub fn is_idle(&self) -> bool {
        *self.state.lock() == SessionState::Idle
    }

    #[allow(dead_code)]
    pub fn is_busy(&self) -> bool {
        *self.state.lock() == SessionState::Busy
    }

    pub fn is_healthy(&self) -> bool {
        self.is_healthy.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn mark_healthy(&self) {
        self.is_healthy
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn mark_unhealthy(&self) {
        self.is_healthy
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn increment_failure(&self) {
        self.failure_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    #[allow(dead_code)]
    pub fn get_failure_count(&self) -> usize {
        self.failure_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Acquire a worker permit from the semaphore
    /// Equivalent to Node.js `sessionManager.acquireWorker()`
    pub async fn acquire_worker(
        &self,
        timeout_ms: u64,
    ) -> Option<tokio::sync::SemaphorePermit<'_>> {
        use tokio::time::{timeout, Duration};

        match timeout(
            Duration::from_millis(timeout_ms),
            self.worker_semaphore.acquire(),
        )
        .await
        {
            Ok(Ok(permit)) => {
                self.active_workers
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
    /// Equivalent to Node.js `sessionManager.acquirePage()`
    pub async fn acquire_page(&self) -> anyhow::Result<Arc<chromiumoxide::Page>> {
        // Create new page (equivalent to browser.newPage())
        let page = self.browser.new_page("about:blank").await?;
        let page = Arc::new(page);
        self.register_page(page.target_id().clone());
        let page_id = page.target_id().as_ref().to_string();
        self.overlay_state.set_active_page(page.clone());
        bind_page_overlay(page_id, self.overlay_state.clone());
        Ok(page)
    }

    /// Acquire a page that opens directly on the target URL.
    pub async fn acquire_page_at(&self, url: &str) -> anyhow::Result<Arc<chromiumoxide::Page>> {
        let page = self.browser.new_page(url).await?;
        let page = Arc::new(page);
        self.register_page(page.target_id().clone());
        let page_id = page.target_id().as_ref().to_string();
        self.overlay_state.set_active_page(page.clone());
        bind_page_overlay(page_id, self.overlay_state.clone());
        Ok(page)
    }

    /// Release a page (close it)
    /// Equivalent to Node.js `sessionManager.releasePage()`
    pub async fn release_page(&self, page: Arc<chromiumoxide::Page>) {
        let page_id = page.target_id().clone();
        let page_id_text = page_id.as_ref().to_string();
        self.overlay_state.clear_active_page_if(&page_id_text);
        unbind_page_overlay(&page_id_text);
        let page_to_close = (*page).clone();
        if let Err(e) = page_to_close.close().await {
            warn!("[{}] Error closing page {:?}: {}", self.id, page_id, e);
        }
        self.unregister_page(page_id.as_ref());
    }

    /// Graceful shutdown - cancel tasks, close pages, close browser
    #[allow(dead_code)]
    pub async fn graceful_shutdown(&mut self) -> anyhow::Result<()> {
        info!("[{}] Starting graceful shutdown", self.id);

        // Mark as failed to stop new tasks
        self.set_state(SessionState::Failed);

        // Close any remaining open pages first
        if let Ok(pages) = self.browser.pages().await {
            for page in pages {
                let page_id = page.target_id().clone();
                let page_id_text = page_id.as_ref().to_string();
                self.overlay_state.clear_active_page_if(&page_id_text);
                unbind_page_overlay(&page_id_text);
                let page_to_close = page.clone();
                if let Err(e) = page_to_close.close().await {
                    warn!(
                        "[{}] Error closing page {:?} during shutdown: {}",
                        self.id, page_id, e
                    );
                }
                self.unregister_page(page_id.as_ref());
            }
        }

        // Close the browser (use inner Arc)
        if let Err(e) = self.browser.close().await {
            warn!("[{}] Error closing browser: {}", self.id, e);
        }

        if let Some(task) = self.overlay_task.take() {
            task.abort();
        }

        // Cancel handler task
        if let Some(task) = self.handler_task.take() {
            task.abort();
        }

        info!("[{}] Shutdown complete", self.id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state_variants() {
        assert_eq!(SessionState::Idle, SessionState::Idle);
        assert_eq!(SessionState::Busy, SessionState::Busy);
        assert_eq!(SessionState::Failed, SessionState::Failed);
    }

    #[test]
    fn test_session_state_inequality() {
        assert_ne!(SessionState::Idle, SessionState::Busy);
        assert_ne!(SessionState::Busy, SessionState::Failed);
        assert_ne!(SessionState::Idle, SessionState::Failed);
    }
}
