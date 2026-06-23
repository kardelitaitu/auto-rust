//! Session worker allocation, page acquire/release, circuit breaker internals, and shutdown.
//!
//! Contains the more complex `impl Session` methods for concurrent worker management,
//! page lifecycle, circuit breaker record keeping, and graceful shutdown.
//! Extracted from `mod.rs` to reduce file size.

use crate::session::state;
use crate::session::state::SessionState;
use crate::session::WorkerPermit;
use crate::state::{bind_page_overlay, unbind_page_overlay};
use log::{info, warn};
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

impl super::Session {
    /// Acquires a worker permit from the semaphore for concurrent page access.
    pub async fn acquire_worker(&self, timeout_ms: u64) -> Option<WorkerPermit<'_>> {
        use tokio::time::{timeout, Duration};

        // Fast-fail if circuit breaker is open
        if self.is_circuit_breaker_open() {
            warn!(
                "[{}] Circuit breaker is open, rejecting task assignment",
                self.id
            );
            return None;
        }

        match timeout(
            Duration::from_millis(timeout_ms),
            self.worker_semaphore.acquire(),
        )
        .await
        {
            Ok(Ok(permit)) => {
                self.active_workers.fetch_add(1, Ordering::SeqCst);
                Some(WorkerPermit::new(permit, &self.active_workers))
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

    pub async fn release_worker(&self, _permit: WorkerPermit<'_>) {
        // Worker released via guard drop
    }

    // ── Circuit breaker internals ────────────────────────────────────────────

    /// Check circuit breaker state. Returns `current_time` if closed, bails if open.
    fn cb_check(&self) -> anyhow::Result<usize> {
        let current_time = state::unix_timestamp_secs();
        let last_failure = self.cb_last_failure_time.load(Ordering::SeqCst);
        let failure_count = self.cb_failure_count.load(Ordering::SeqCst);

        if failure_count >= self.cb_failure_threshold
            && current_time.saturating_sub(last_failure) < self.cb_timeout_secs as usize
        {
            self.mark_unhealthy();
            anyhow::bail!(
                "Circuit breaker open ({} failures, {}s timeout), rejecting page acquisition for session {}",
                failure_count,
                self.cb_timeout_secs,
                self.id
            );
        }
        Ok(current_time)
    }

    /// Record a circuit breaker success (reset counters).
    fn cb_record_success(&self) {
        self.cb_failure_count.store(0, Ordering::SeqCst);
    }

    /// Record a circuit breaker failure — atomically increments and checks threshold.
    fn cb_record_failure(&self, current_time: usize) {
        let new_count = self.cb_failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        self.cb_last_failure_time
            .store(current_time, Ordering::SeqCst);
        warn!(
            "[{}] Circuit breaker failure count: {}/{}",
            self.id, new_count, self.cb_failure_threshold
        );
    }

    // ── Page acquire / release ───────────────────────────────────────────────

    /// Acquires a new browser page for task execution with circuit breaker protection.
    pub async fn acquire_page(&self) -> anyhow::Result<Arc<chromiumoxide::Page>> {
        self.cb_check()?;

        let page = match self.browser.new_page("about:blank").await {
            Ok(page) => {
                self.cb_record_success();
                page
            }
            Err(e) => {
                self.cb_record_failure(state::unix_timestamp_secs());
                return Err(e.into());
            }
        };

        let page = Arc::new(page);
        self.register_page(page.target_id().clone());
        let page_id = page.target_id().as_ref().to_string();
        self.overlay_state.set_active_page(page.clone());
        bind_page_overlay(page_id, self.overlay_state.clone());
        Ok(page)
    }

    /// Acquires a new browser page that opens directly on the target URL.
    pub async fn acquire_page_at(&self, url: &str) -> anyhow::Result<Arc<chromiumoxide::Page>> {
        self.cb_check()?;

        let page = match self.browser.new_page(url).await {
            Ok(page) => {
                self.cb_record_success();
                page
            }
            Err(e) => {
                self.cb_record_failure(state::unix_timestamp_secs());
                return Err(e.into());
            }
        };

        let page = Arc::new(page);
        self.register_page(page.target_id().clone());
        let page_id = page.target_id().as_ref().to_string();
        self.overlay_state.set_active_page(page.clone());
        bind_page_overlay(page_id, self.overlay_state.clone());
        Ok(page)
    }

    /// Releases a page by closing it and cleaning up associated resources.
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

    // ── Page cleanup ──────────────────────────────────────────────────────────

    /// Closes only pages created and tracked by this session.
    pub async fn cleanup_managed_pages(&self) -> anyhow::Result<usize> {
        let tracked_ids: Vec<String> = self
            .active_pages
            .iter()
            .map(|id| id.as_ref().to_string())
            .collect();
        if tracked_ids.is_empty() {
            return Ok(0);
        }

        let tracked_set: HashSet<String> = tracked_ids.iter().cloned().collect();
        let pages = self.browser.pages().await?;
        let mut closed = 0usize;

        for page in pages {
            let page_id = page.target_id().clone();
            let page_id_text = page_id.as_ref().to_string();
            if !tracked_set.contains(&page_id_text) {
                continue;
            }

            self.overlay_state.clear_active_page_if(&page_id_text);
            unbind_page_overlay(&page_id_text);
            let page_to_close = page.clone();
            if let Err(e) = page_to_close.close().await {
                warn!(
                    "[{}] Error closing managed page {:?}: {}",
                    self.id, page_id, e
                );
            } else {
                closed += 1;
            }
        }

        for page_id in tracked_ids {
            self.unregister_page(&page_id);
        }

        Ok(closed)
    }

    // ── Graceful shutdown ─────────────────────────────────────────────────────

    /// Performs a graceful shutdown of the session, cleaning up all resources.
    pub async fn graceful_shutdown(&mut self) -> anyhow::Result<()> {
        info!("[{}] Starting graceful shutdown", self.id);

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

        // Close the browser
        match tokio::time::timeout(Duration::from_secs(10), self.browser.close()).await {
            Ok(Ok(_)) => {
                info!("[{}] Browser closed successfully", self.id);
            }
            Ok(Err(e)) => {
                warn!("[{}] Error closing browser: {}", self.id, e);
            }
            Err(_) => {
                warn!("[{}] Browser close timeout after 10s", self.id);
            }
        }

        if let Some(task) = self.overlay_task.take() {
            task.abort();
        }

        if let Some(task) = self.handler_task.take() {
            task.abort();
        }

        info!("[{}] Shutdown complete", self.id);
        Ok(())
    }
}
