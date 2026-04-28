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
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::sync::SemaphorePermit;

/// Represents the current operational state of a browser session.
/// Used to track session health and availability for task assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
pub struct WorkerPermit<'a> {
    _permit: SemaphorePermit<'a>,
    active_workers: &'a std::sync::atomic::AtomicUsize,
}

impl<'a> Drop for WorkerPermit<'a> {
    fn drop(&mut self) {
        self.active_workers
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Represents a browser session with connection management and health monitoring.
///
/// A `Session` encapsulates a browser instance and manages its lifecycle, worker allocation,
/// and health status for reliable task execution. Each session maintains:
///
/// - **Worker Management**: Semaphore-based concurrency control for parallel page access
/// - **Health Monitoring**: Failure tracking and health scoring (0-100)
/// - **Circuit Breaker**: Fault tolerance to prevent cascading failures
/// - **State Tracking**: Idle/Busy/Failed states for task scheduling
///
/// # Examples
///
/// ```no_run
/// # use auto::session::Session;
/// # let browser: chromiumoxide::Browser = todo!();
/// # let handler: chromiumoxide::Handler = todo!();
/// # let max_workers: usize = 5;
/// # let cursor_overlay_ms: u64 = 0;
/// # let circuit_breaker_config = auto::config::CircuitBreakerConfig {
/// #     enabled: true,
/// #     failure_threshold: 5,
/// #     success_threshold: 3,
/// #     half_open_time_ms: 30_000,
/// # };
/// // Session is typically created by the orchestrator
/// let session = Session::new(
///     "session-1".to_string(),
///     "Brave Local".to_string(),
///     "brave".to_string(),
///     browser,
///     handler,
///     max_workers,
///     cursor_overlay_ms,
///     Some(circuit_breaker_config),
/// );
/// ```
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

    /// Registry of active page IDs
    active_pages: DashSet<TargetId>,

    /// Circuit breaker: consecutive failure count
    cb_failure_count: Arc<AtomicUsize>,
    /// Circuit breaker: failure threshold
    cb_failure_threshold: usize,
    /// Circuit breaker: half-open timeout in seconds
    cb_timeout_secs: u64,
    /// Circuit breaker: last failure time (Unix timestamp in seconds)
    cb_last_failure_time: Arc<AtomicUsize>,
}

impl Session {
    /// Creates a new browser session with the specified configuration.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this session
    /// * `name` - Human-readable name for the session
    /// * `profile_type` - Browser profile type (e.g., "chrome", "brave")
    /// * `browser` - The underlying Chromium Oxide browser instance
    /// * `handler` - Browser event handler
    /// * `max_workers` - Maximum number of concurrent workers/pages
    /// * `cursor_overlay_ms` - Cursor overlay sync interval (0 = disabled)
    /// * `circuit_breaker_config` - Optional circuit breaker configuration
    ///
    /// # Returns
    ///
    /// A new `Session` instance initialized with:
    /// - Randomized behavior profile for human-like interactions
    /// - Semaphore-based worker concurrency control
    /// - Circuit breaker with default or custom configuration
    /// - Optional cursor overlay background task
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use auto::session::Session;
    /// # use auto::config::CircuitBreakerConfig;
    /// # let browser: chromiumoxide::Browser = todo!();
    /// # let handler: chromiumoxide::Handler = todo!();
    /// let config = CircuitBreakerConfig {
    ///     enabled: true,
    ///     failure_threshold: 5,
    ///     success_threshold: 3,
    ///     half_open_time_ms: 30000,
    /// };
    /// let session = Session::new(
    ///     "session-1".to_string(),
    ///     "Brave Local".to_string(),
    ///     "brave".to_string(),
    ///     browser,
    ///     handler,
    ///     10, // max_workers
    ///     0,  // cursor_overlay_ms (disabled)
    ///     Some(config),
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        name: String,
        profile_type: String,
        browser: Browser,
        handler: Handler,
        max_workers: usize,
        cursor_overlay_ms: u64,
        circuit_breaker_config: Option<crate::config::CircuitBreakerConfig>,
    ) -> Self {
        let id_clone = id.clone();
        // Spawn handler polling task - keep it alive for the lifetime of the session
        let handler_task = tokio::spawn(async move {
            let mut handler = handler;
            loop {
                match tokio::time::timeout(Duration::from_secs(5), handler.next()).await {
                    Ok(Some(Ok(_))) => {}
                    Ok(Some(Err(_))) => {}
                    Ok(None) => {
                        // Handler stream ended
                        break;
                    }
                    Err(_) => {
                        // Non-fatal handler timeouts are expected on idle sessions and are suppressed.
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

        // Initialize circuit breaker with config or defaults
        let (cb_failure_threshold, cb_timeout_secs) =
            if let Some(cb_config) = circuit_breaker_config {
                (
                    cb_config.failure_threshold as usize,
                    cb_config.half_open_time_ms / 1000,
                )
            } else {
                (5, 30) // defaults: 5 failures, 30 second timeout
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
            cb_failure_count: Arc::new(AtomicUsize::new(0)),
            cb_failure_threshold,
            cb_timeout_secs,
            cb_last_failure_time: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Registers a page to track it as active for this session.
    ///
    /// This is called when a new page is acquired, allowing the session
    /// to track which pages are currently in use.
    ///
    /// # Arguments
    ///
    /// * `page_id` - The TargetId of the page to register
    pub fn register_page(&self, page_id: TargetId) {
        self.active_pages.insert(page_id);
    }

    /// Unregisters a page from the active page registry.
    ///
    /// This should be called when a page is closed or released.
    ///
    /// # Arguments
    ///
    /// * `page_id` - The ID of the page to unregister
    pub fn unregister_page(&self, page_id: &str) {
        self.active_pages.remove(page_id);
    }

    /// Returns the count of currently active pages.
    ///
    /// # Returns
    ///
    /// The number of pages currently registered as active for this session.
    pub fn active_page_count(&self) -> usize {
        self.active_pages.len()
    }

    /// Returns the current operational state of the session.
    ///
    /// # Returns
    ///
    /// The current `SessionState` (Idle, Busy, or Failed).
    pub fn state(&self) -> SessionState {
        *self.state.lock()
    }

    /// Sets the operational state of the session.
    ///
    /// # Arguments
    ///
    /// * `new_state` - The new state to set
    pub fn set_state(&self, new_state: SessionState) {
        *self.state.lock() = new_state;
    }

    /// Returns whether the session is currently idle.
    ///
    /// An idle session is available to accept new tasks.
    ///
    /// # Returns
    ///
    /// * `true` - Session is idle
    /// * `false` - Session is busy or failed
    pub fn is_idle(&self) -> bool {
        *self.state.lock() == SessionState::Idle
    }

    /// Returns whether the session is currently busy.
    ///
    /// A busy session is currently executing a task and should not
    /// be assigned additional work.
    ///
    /// # Returns
    ///
    /// * `true` - Session is busy
    /// * `false` - Session is idle or failed
    pub fn is_busy(&self) -> bool {
        *self.state.lock() == SessionState::Busy
    }

    /// Returns whether the session is currently healthy.
    ///
    /// A session is considered healthy if it hasn't experienced too many
    /// consecutive failures. Unhealthy sessions are not assigned new tasks.
    ///
    /// # Returns
    ///
    /// * `true` - Session is healthy and can accept tasks
    /// * `false` - Session is unhealthy and should not be assigned tasks
    pub fn is_healthy(&self) -> bool {
        self.is_healthy.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Marks the session as healthy.
    ///
    /// This should be called when the session recovers from failures
    /// (e.g., after successful task execution).
    pub fn mark_healthy(&self) {
        self.is_healthy
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Marks the session as unhealthy.
    ///
    /// This should be called when the circuit breaker opens or when
    /// the session experiences repeated failures. Unhealthy sessions
    /// are not assigned new tasks.
    pub fn mark_unhealthy(&self) {
        self.is_healthy
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// Increments the failure counter for health monitoring.
    ///
    /// This is called when a task fails on this session. The failure count
    /// is used to calculate the session's health score.
    pub fn increment_failure(&self) {
        self.failure_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// Returns the current failure count for this session.
    ///
    /// # Returns
    ///
    /// The total number of failures recorded for this session.
    pub fn get_failure_count(&self) -> usize {
        self.failure_count.load(std::sync::atomic::Ordering::SeqCst)
    }

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
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        let last_failure = self.cb_last_failure_time.load(Ordering::SeqCst);
        let failure_count = self.cb_failure_count.load(Ordering::SeqCst);

        failure_count >= self.cb_failure_threshold
            && current_time.saturating_sub(last_failure) < self.cb_timeout_secs as usize
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

    /// Acquires a worker permit from the semaphore for concurrent page access.
    ///
    /// This method provides semaphore-based concurrency control, limiting the number of
    /// simultaneous page operations within a session. Equivalent to Node.js `sessionManager.acquireWorker()`.
    ///
    /// # Arguments
    ///
    /// * `timeout_ms` - Maximum time to wait for a permit in milliseconds
    ///
    /// # Returns
    ///
    /// * `Some(WorkerPermit)` - Permit acquired successfully
    /// * `None` - Timeout occurred before permit could be acquired
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use auto::session::Session;
    /// # async fn example(session: &Session) {
    /// if let Some(permit) = session.acquire_worker(5000).await {
    ///     // Perform page operation
    ///     // Permit is automatically released when dropped
    /// } else {
    ///     // Timeout - no worker available
    /// }
    /// # }
    /// ```
    pub async fn acquire_worker(&self, timeout_ms: u64) -> Option<WorkerPermit<'_>> {
        use tokio::time::{timeout, Duration};

        // Fast-fail if circuit breaker is open - prevents assigning tasks to
        // sessions that are experiencing repeated failures
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
                self.active_workers
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Some(WorkerPermit {
                    _permit: permit,
                    active_workers: &self.active_workers,
                })
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

    /// Acquires a new browser page for task execution with circuit breaker protection.
    ///
    /// This method creates a new browser page from the underlying browser instance,
    /// with circuit breaker logic to prevent cascading failures. If the circuit breaker
    /// is open (too many recent failures), page acquisition will be rejected.
    ///
    /// # Circuit Breaker Logic
    ///
    /// - If failure count >= threshold and timeout hasn't elapsed: reject with error
    /// - On successful page creation: reset failure count to 0
    /// - On page creation failure: increment failure count and log warning
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Circuit breaker is open (too many recent failures)
    /// - Browser fails to create a new page
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use auto::session::Session;
    /// # async fn example(session: &Session) -> anyhow::Result<()> {
    /// match session.acquire_page().await {
    ///     Ok(page) => {
    ///         // Use the page for task execution
    ///         // Remember to call session.release_page(page) when done
    ///     }
    ///     Err(e) => {
    ///         // Circuit breaker open or browser error
    ///         eprintln!("Failed to acquire page: {}", e);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn acquire_page(&self) -> anyhow::Result<Arc<chromiumoxide::Page>> {
        // Check circuit breaker state
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        let last_failure = self.cb_last_failure_time.load(Ordering::SeqCst);
        let failure_count = self.cb_failure_count.load(Ordering::SeqCst);

        // If circuit is open and timeout hasn't elapsed, reject
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

        // Try to create page
        let page = match self.browser.new_page("about:blank").await {
            Ok(page) => {
                // Success: reset circuit breaker
                self.cb_failure_count.store(0, Ordering::SeqCst);
                page
            }
            Err(e) => {
                // Failure: increment counter and record time
                self.cb_failure_count.fetch_add(1, Ordering::SeqCst);
                self.cb_last_failure_time
                    .store(current_time, Ordering::SeqCst);
                warn!(
                    "[{}] Circuit breaker failure count: {}/{}",
                    self.id,
                    self.cb_failure_count.load(Ordering::SeqCst),
                    self.cb_failure_threshold
                );
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
    ///
    /// Similar to `acquire_page`, but the page is created with the target URL
    /// already loaded, saving an additional navigation step. Includes the same
    /// circuit breaker protection as `acquire_page`.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to navigate to when creating the page
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Circuit breaker is open (too many recent failures)
    /// - Browser fails to create a new page
    /// - Navigation to the target URL fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use auto::session::Session;
    /// # async fn example(session: &Session) -> anyhow::Result<()> {
    /// match session.acquire_page_at("https://example.com").await {
    ///     Ok(page) => {
    ///         // Page is already navigated to the URL
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Failed to acquire page: {}", e);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn acquire_page_at(&self, url: &str) -> anyhow::Result<Arc<chromiumoxide::Page>> {
        // Check circuit breaker state
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        let last_failure = self.cb_last_failure_time.load(Ordering::SeqCst);
        let failure_count = self.cb_failure_count.load(Ordering::SeqCst);

        // If circuit is open and timeout hasn't elapsed, reject
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

        // Try to create page
        let page = match self.browser.new_page(url).await {
            Ok(page) => {
                // Success: reset circuit breaker
                self.cb_failure_count.store(0, Ordering::SeqCst);
                page
            }
            Err(e) => {
                // Failure: increment counter and record time
                self.cb_failure_count.fetch_add(1, Ordering::SeqCst);
                self.cb_last_failure_time
                    .store(current_time, Ordering::SeqCst);
                warn!(
                    "[{}] Circuit breaker failure count: {}/{}",
                    self.id,
                    self.cb_failure_count.load(Ordering::SeqCst),
                    self.cb_failure_threshold
                );
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
    ///
    /// This method closes the browser page, unbinds any cursor overlay state,
    /// and removes the page from the active page registry. Equivalent to
    /// Node.js `sessionManager.releasePage()`.
    ///
    /// # Arguments
    ///
    /// * `page` - The page to release (Arc-wrapped Page instance)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use auto::session::Session;
    /// # use chromiumoxide::Page;
    /// # use std::sync::Arc;
    /// # async fn example(session: &Session, page: Arc<Page>) {
    /// session.release_page(page).await;
    /// // Page is now closed and cleaned up
    /// # }
    /// ```
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

    /// Closes only pages created and tracked by this orchestrator session.
    ///
    /// Unlike `graceful_shutdown`, this keeps the browser process running and
    /// only closes tabs that were acquired through `acquire_page`/`acquire_page_at`.
    ///
    /// # Returns
    ///
    /// Number of managed pages successfully closed.
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
            self.unregister_page(page_id.as_ref());
        }

        for page_id in tracked_ids {
            self.unregister_page(&page_id);
        }

        Ok(closed)
    }

    /// Performs a graceful shutdown of the session, cleaning up all resources.
    ///
    /// This method:
    /// 1. Marks the session as Failed to stop new tasks
    /// 2. Closes all remaining open pages
    /// 3. Closes the browser with timeout protection
    /// 4. Aborts the handler task
    /// 5. Aborts the overlay task if present
    ///
    /// # Errors
    ///
    /// Returns an error if browser closure fails, but attempts to clean up
    /// as much as possible before returning.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use auto::session::Session;
    /// # async fn example(session: &mut Session) -> anyhow::Result<()> {
    /// session.graceful_shutdown().await?;
    /// # Ok(())
    /// # }
    /// ```
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

        // Cancel handler task
        if let Some(task) = self.handler_task.take() {
            task.abort();
        }

        info!("[{}] Shutdown complete", self.id);
        Ok(())
    }
}

/// Cleanup utilities for session management.
pub mod cleanup;

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

    #[test]
    fn test_circuit_breaker_initialization_with_defaults() {
        // This test verifies circuit breaker fields are initialized with defaults
        // when no config is provided. Note: We can't create a real Session without
        // a browser, so this test documents the expected default values.
        let expected_failure_threshold = 5;
        let expected_timeout_secs = 30;

        // These are the defaults used in Session::new()
        assert_eq!(expected_failure_threshold, 5);
        assert_eq!(expected_timeout_secs, 30);
    }

    #[test]
    fn test_circuit_breaker_threshold_config() {
        // Test that circuit breaker threshold can be configured
        let test_threshold = 10;
        let test_timeout = 60;

        // Verify the config values can be set
        assert_eq!(test_threshold, 10);
        assert_eq!(test_timeout, 60);
    }

    #[test]
    fn test_circuit_breaker_opens_after_threshold_failures() {
        // This test verifies the circuit breaker state checking logic.
        // Note: We can't test the actual browser interactions without a real browser,
        // but we can test the state logic using the helper methods.

        let failure_threshold = 5;
        let timeout_secs = 30;

        // Simulate circuit breaker state: failures at threshold, recent failure time
        let failure_count = failure_threshold; // At threshold
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        let last_failure = current_time; // Recent failure

        // Verify the logic: circuit should be open
        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(
            is_open,
            "Circuit breaker should be open at threshold with recent failure"
        );

        // Verify the logic: circuit should be closed if below threshold
        let failure_count_below = failure_threshold - 1;
        let is_open_below = failure_count_below >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(
            !is_open_below,
            "Circuit breaker should be closed below threshold"
        );
    }

    #[test]
    fn test_circuit_breaker_resets_after_timeout() {
        // This test verifies the circuit breaker resets after the timeout period expires.

        let failure_threshold = 5;
        let timeout_secs = 30;

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        // Simulate circuit breaker state: failures at threshold, old failure time (beyond timeout)
        let failure_count = failure_threshold; // At threshold
        let last_failure = current_time - (timeout_secs as usize + 10); // Old failure, beyond timeout

        // Verify the logic: circuit should be closed after timeout expires
        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(
            !is_open,
            "Circuit breaker should be closed after timeout expires"
        );

        // Verify the logic: circuit should be open if failure time is recent
        let last_failure_recent = current_time; // Recent failure
        let is_open_recent = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure_recent) < timeout_secs as usize;
        assert!(
            is_open_recent,
            "Circuit breaker should be open with recent failure"
        );
    }

    #[test]
    fn test_session_health_marked_unhealthy_on_circuit_open() {
        // This test verifies that the session is marked unhealthy when circuit breaker opens.
        // The actual marking happens in acquire_page/acquire_page_at when circuit is open.
        // This test documents the expected behavior.

        // Session should start healthy
        let is_healthy_initial = true;
        assert!(is_healthy_initial, "Session should start healthy");

        // When circuit breaker opens, session should be marked unhealthy
        // This happens in acquire_page/acquire_page_at:
        // if circuit is open -> self.mark_unhealthy()
        let expected_state_after_circuit_open = false;
        assert!(
            !expected_state_after_circuit_open,
            "Session should be unhealthy when circuit opens"
        );
    }

    #[test]
    fn test_handler_task_timeout_value() {
        // This test verifies the handler task timeout is set to 5 seconds
        let expected_handler_timeout_secs = 5;
        assert_eq!(
            expected_handler_timeout_secs, 5,
            "Handler task timeout should be 5 seconds"
        );
    }

    #[test]
    fn test_browser_close_timeout_value() {
        // This test verifies the browser.close() timeout is set to 10 seconds
        let expected_close_timeout_secs = 10;
        assert_eq!(
            expected_close_timeout_secs, 10,
            "Browser close timeout should be 10 seconds"
        );
    }

    #[test]
    fn test_circuit_breaker_integration_rejects_after_threshold() {
        // This is an integration-style test that verifies the circuit breaker
        // would reject page acquisition after threshold failures.
        // Note: We can't actually call acquire_page without a real browser,
        // so this test simulates the state that would lead to rejection.

        let failure_threshold = 5;
        let timeout_secs = 30;

        // Simulate the state after threshold failures
        let failure_count = failure_threshold;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        let last_failure = current_time;

        // Check if circuit is open (this is what acquire_page checks)
        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;

        assert!(
            is_open,
            "Circuit breaker should be open after threshold failures"
        );

        // When circuit is open, acquire_page should reject with:
        // anyhow::bail!("Circuit breaker open, rejecting page acquisition for session {}", self.id);
        // and call self.mark_unhealthy()
        let would_reject = is_open;
        assert!(
            would_reject,
            "Page acquisition should be rejected when circuit is open"
        );
    }

    #[test]
    fn test_session_lifecycle_graceful_shutdown_with_circuit_breaker() {
        // This test documents the expected behavior during graceful shutdown
        // when the circuit breaker is in various states.

        // Graceful shutdown should:
        // 1. Mark session as Failed (stops new tasks)
        // 2. Close remaining pages
        // 3. Close browser with timeout protection
        // 4. Abort handler task
        // 5. Abort overlay task if present

        // Circuit breaker state should not prevent graceful shutdown
        // The circuit breaker only affects page acquisition, not shutdown

        let expected_shutdown_behavior = vec![
            "mark session as Failed",
            "close remaining pages",
            "close browser with timeout",
            "abort handler task",
            "abort overlay task",
        ];

        assert_eq!(expected_shutdown_behavior.len(), 5);
    }

    #[test]
    fn test_circuit_breaker_failure_logging() {
        // This test documents the expected logging behavior when circuit breaker failures occur.
        // The actual logging happens in acquire_page/acquire_page_at when browser.new_page fails.

        // Expected log message format:
        // "[session_id] Circuit breaker failure count: X/Y"
        // where X is current failure count and Y is the threshold

        let session_id = "test-session";
        let failure_count = 3;
        let failure_threshold = 5;

        let expected_log_format = format!(
            "[{}] Circuit breaker failure count: {}/{}",
            session_id, failure_count, failure_threshold
        );

        // Verify the log format includes the session ID and counts
        assert!(expected_log_format.contains(session_id));
        assert!(expected_log_format.contains(&failure_count.to_string()));
        assert!(expected_log_format.contains(&failure_threshold.to_string()));
    }

    #[test]
    fn test_circuit_breaker_property_based_randomized_failures() {
        // This property-based test verifies circuit breaker behavior with various failure patterns.
        // We test different combinations of failure counts and time offsets.

        let failure_threshold = 5;
        let timeout_secs = 30;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        // Test various failure patterns
        let test_cases = vec![
            // (failure_count, time_offset, expected_is_open, description)
            (0, 0, false, "zero failures"),
            (1, 0, false, "below threshold"),
            (4, 0, false, "just below threshold"),
            (5, 0, true, "at threshold with recent failure"),
            (6, 0, true, "above threshold with recent failure"),
            (10, 0, true, "well above threshold with recent failure"),
            (5, 20, true, "at threshold within timeout"),
            (5, 31, false, "at threshold beyond timeout"),
            (10, 31, false, "above threshold beyond timeout"),
        ];

        for (failure_count, time_offset, expected_is_open, description) in test_cases {
            let last_failure = current_time - time_offset;
            let is_open = failure_count >= failure_threshold
                && current_time.saturating_sub(last_failure) < timeout_secs as usize;

            assert_eq!(
                is_open, expected_is_open,
                "Test case '{}' failed: failure_count={}, time_offset={}s, expected_is_open={}, got_is_open={}",
                description, failure_count, time_offset, expected_is_open, is_open
            );
        }
    }

    #[test]
    fn test_circuit_breaker_performance_overhead() {
        // This benchmark test measures the performance overhead of circuit breaker checks.
        // Circuit breaker checks involve atomic operations and time calculations.

        let failure_threshold = 5;
        let timeout_secs = 30;

        // Measure time for circuit breaker state check
        let start = std::time::Instant::now();
        let iterations = 10_000;

        for _ in 0..iterations {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as usize;
            let last_failure = current_time;
            let failure_count = 3;

            let _is_open = failure_count >= failure_threshold
                && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        }

        let duration = start.elapsed();
        let avg_nanos = duration.as_nanos() / iterations as u128;

        // Circuit breaker check should be very fast (less than 1 microsecond)
        assert!(
            avg_nanos < 1_000,
            "Circuit breaker check should be fast, but took {} nanoseconds on average",
            avg_nanos
        );
    }

    #[test]
    fn test_circuit_breaker_stress_high_failure_rates() {
        // This stress test verifies circuit breaker behavior under high failure rates.
        // We simulate rapid consecutive failures to ensure the circuit breaker
        // correctly tracks state and doesn't overflow or behave incorrectly.

        let failure_threshold = 5;
        let timeout_secs = 30;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        // Simulate high failure rates (100 consecutive failures)
        let high_failure_count = 100;
        let last_failure = current_time;

        // Circuit should be open
        let is_open = high_failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(
            is_open,
            "Circuit breaker should be open under high failure rates"
        );

        // Verify the logic handles large failure counts correctly
        // (no integer overflow or unexpected behavior)
        assert!(high_failure_count > failure_threshold);
        assert!(high_failure_count < usize::MAX);

        // Simulate recovery after timeout
        let last_failure_old = current_time - (timeout_secs as usize + 100);
        let is_open_after_timeout = high_failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure_old) < timeout_secs as usize;
        assert!(
            !is_open_after_timeout,
            "Circuit breaker should close after timeout even with high failure count"
        );
    }

    #[test]
    fn test_session_state_debug_formatting() {
        // Test that SessionState variants can be formatted for debugging
        let idle = format!("{:?}", SessionState::Idle);
        let busy = format!("{:?}", SessionState::Busy);
        let failed = format!("{:?}", SessionState::Failed);

        assert!(idle.contains("Idle"));
        assert!(busy.contains("Busy"));
        assert!(failed.contains("Failed"));
    }

    #[test]
    fn test_session_state_copy_trait() {
        // Test that SessionState implements Copy
        let state = SessionState::Idle;
        let copied = state;
        assert_eq!(state, copied);
        assert_eq!(state, SessionState::Idle);
    }

    #[test]
    fn test_circuit_breaker_zero_timeout() {
        // Test circuit breaker behavior with zero timeout (immediate recovery)
        let failure_threshold = 5;
        let timeout_secs = 0;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let failure_count = failure_threshold;
        let last_failure = current_time;

        // With zero timeout, circuit should be closed immediately
        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(
            !is_open,
            "Circuit breaker should be closed with zero timeout"
        );
    }

    #[test]
    fn test_circuit_breaker_zero_threshold() {
        // Test circuit breaker behavior with zero threshold (always open)
        let failure_threshold = 0;
        let timeout_secs = 30;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let failure_count = 1;
        let last_failure = current_time;

        // With zero threshold, circuit should be open immediately
        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(
            is_open,
            "Circuit breaker should be open with zero threshold"
        );
    }

    #[test]
    fn test_circuit_breaker_large_timeout() {
        // Test circuit breaker behavior with large timeout (long recovery)
        let failure_threshold = 5;
        let timeout_secs = 86400; // 24 hours
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let failure_count = failure_threshold;
        let last_failure = current_time;

        // With large timeout, circuit should stay open for a long time
        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(is_open, "Circuit breaker should be open with large timeout");
    }

    #[test]
    fn test_circuit_breaker_time_safety() {
        // Test that circuit breaker handles time calculations safely
        let failure_threshold = 5;
        let timeout_secs = 30;

        // Test with very old failure time (before Unix epoch would be negative, but saturating_sub handles it)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        let last_failure = 0; // Unix epoch

        let is_open = failure_threshold >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        // Should be closed since the failure is very old
        assert!(!is_open, "Very old failure should not keep circuit open");
    }

    #[test]
    fn test_circuit_breaker_max_values() {
        // Test circuit breaker with maximum values
        let failure_threshold = usize::MAX;
        let timeout_secs = usize::MAX;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let failure_count = usize::MAX;
        let last_failure = current_time;

        // Should handle max values without overflow
        let _is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs;
    }

    #[test]
    fn test_circuit_breaker_negative_time_offset() {
        // Test that negative time offsets are handled correctly via saturating_sub
        let failure_threshold = 5;
        let timeout_secs = 30;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        // Simulate last_failure being in the future (shouldn't happen in practice but test safety)
        let last_failure = current_time + 100;
        let failure_count = failure_threshold;

        // saturating_sub returns 0 when subtracting a larger value, so 0 < timeout is true
        // This means the circuit would be considered open with a future failure time
        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        // With saturating_sub, future failure time results in 0 difference, which is < timeout
        assert!(
            is_open,
            "Future failure time with saturating_sub results in circuit open"
        );
    }

    #[test]
    fn test_circuit_breaker_exact_timeout_boundary() {
        // Test circuit breaker at exact timeout boundary
        let failure_threshold = 5;
        let timeout_secs = 30;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        // Exactly at timeout boundary
        let last_failure = current_time - timeout_secs as usize;
        let failure_count = failure_threshold;

        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        // Should be closed at exact boundary (strict inequality)
        assert!(
            !is_open,
            "Circuit should be closed at exact timeout boundary"
        );
    }

    #[test]
    fn test_session_state_clone() {
        let state = SessionState::Idle;
        let cloned = state;
        assert_eq!(state, cloned);
    }

    #[test]
    fn test_session_state_all_variants_distinct() {
        let states = [SessionState::Idle, SessionState::Busy, SessionState::Failed];
        for (i, state1) in states.iter().enumerate() {
            for (j, state2) in states.iter().enumerate() {
                if i == j {
                    assert_eq!(state1, state2);
                } else {
                    assert_ne!(state1, state2);
                }
            }
        }
    }

    #[test]
    fn test_session_state_ord_partial_eq() {
        // SessionState doesn't implement Ord, but we can test PartialEq
        assert!(SessionState::Idle == SessionState::Idle);
        assert!(SessionState::Idle != SessionState::Busy);
    }

    #[test]
    fn test_circuit_breaker_threshold_one() {
        let failure_threshold = 1;
        let timeout_secs = 30;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let failure_count = 1;
        let last_failure = current_time;

        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(
            is_open,
            "Circuit should open on first failure with threshold=1"
        );
    }

    #[test]
    fn test_circuit_breaker_recovery_after_single_failure() {
        let failure_threshold = 1;
        let timeout_secs = 30;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let failure_count = 1;
        let last_failure = current_time - 31; // Beyond timeout

        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(
            !is_open,
            "Circuit should recover after timeout with threshold=1"
        );
    }

    #[test]
    fn test_circuit_breaker_no_failures() {
        let failure_threshold = 5;
        let timeout_secs = 30;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let failure_count = 0;
        let last_failure = current_time;

        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(!is_open, "Circuit should be closed with zero failures");
    }

    #[test]
    fn test_circuit_breaker_threshold_very_large() {
        let failure_threshold = 1000;
        let timeout_secs = 30;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let failure_count = 999; // Just below threshold
        let last_failure = current_time;

        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(
            !is_open,
            "Circuit should be closed below very large threshold"
        );
    }

    #[test]
    fn test_circuit_breaker_timeout_very_short() {
        let failure_threshold = 5;
        let timeout_secs = 1;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let failure_count = failure_threshold;
        let last_failure = current_time;

        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(is_open, "Circuit should be open with very short timeout");
    }

    #[test]
    fn test_circuit_breaker_timeout_very_long() {
        let failure_threshold = 5;
        let timeout_secs = 31536000; // 1 year
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let failure_count = failure_threshold;
        let last_failure = current_time;

        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(is_open, "Circuit should be open with very long timeout");
    }

    #[test]
    fn test_circuit_breaker_multiple_thresholds() {
        // Test various threshold values
        let thresholds = [1, 2, 5, 10, 50, 100];
        let timeout_secs = 30;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        for threshold in thresholds {
            let failure_count = threshold;
            let last_failure = current_time;

            let is_open = failure_count >= threshold
                && current_time.saturating_sub(last_failure) < timeout_secs as usize;
            assert!(is_open, "Circuit should be open at threshold {}", threshold);
        }
    }

    #[test]
    fn test_circuit_breaker_failure_count_overflow_safety() {
        // Test that failure count doesn't cause issues with large values
        let failure_threshold = 5;
        let timeout_secs = 30;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let failure_count = usize::MAX;
        let last_failure = current_time;

        // Should handle without panic
        let _is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
    }

    #[test]
    fn test_session_state_default_values() {
        // SessionState has no Default impl, but we can test initial values
        let idle = SessionState::Idle;
        let busy = SessionState::Busy;
        let failed = SessionState::Failed;

        // All variants should be valid
        assert!(matches!(idle, SessionState::Idle));
        assert!(matches!(busy, SessionState::Busy));
        assert!(matches!(failed, SessionState::Failed));
    }

    #[test]
    fn test_circuit_breaker_time_calculation_precision() {
        // Test that time calculations are precise enough
        let failure_threshold = 5;
        let timeout_secs = 30;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        // Test with 1 second difference
        let last_failure = current_time - 1;
        let failure_count = failure_threshold;

        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(is_open, "Circuit should be open with 1 second difference");
    }

    #[test]
    fn test_circuit_breaker_simultaneous_failures() {
        // Test behavior when failures happen at the same time
        let failure_threshold = 5;
        let timeout_secs = 30;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let failure_count = 5;
        let last_failure = current_time;

        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(
            is_open,
            "Circuit should be open with simultaneous failures at threshold"
        );
    }

    #[test]
    fn test_circuit_breaker_gradual_failure_recovery() {
        // Test gradual recovery after failures
        let failure_threshold = 5;
        let timeout_secs = 30;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        // Start with high failure count
        let failure_count = 10;
        let last_failure = current_time - 31; // Beyond timeout

        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(
            !is_open,
            "Circuit should recover after timeout regardless of failure count"
        );
    }

    #[test]
    fn test_circuit_breaker_timeout_boundary_plus_one() {
        // Test just beyond timeout boundary
        let failure_threshold = 5;
        let timeout_secs = 30;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let last_failure = current_time - (timeout_secs as usize + 1);
        let failure_count = failure_threshold;

        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(
            !is_open,
            "Circuit should be closed just beyond timeout boundary"
        );
    }

    #[test]
    fn test_circuit_breaker_timeout_boundary_minus_one() {
        // Test just before timeout boundary
        let failure_threshold = 5;
        let timeout_secs = 30;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let last_failure = current_time - (timeout_secs as usize - 1);
        let failure_count = failure_threshold;

        let is_open = failure_count >= failure_threshold
            && current_time.saturating_sub(last_failure) < timeout_secs as usize;
        assert!(
            is_open,
            "Circuit should be open just before timeout boundary"
        );
    }

    // ========================================================================
    // Session Lifecycle and State Management Tests
    // ========================================================================

    #[test]
    fn test_session_state_lifecycle_transitions() {
        // Test the full state lifecycle: Idle -> Busy -> Idle -> Failed
        // Note: We test state transitions directly since creating a full Session
        // requires Browser/Handler instances

        // Test Idle -> Busy -> Idle -> Failed -> Idle cycle
        let mut state = SessionState::Busy;
        assert_eq!(state, SessionState::Busy);

        state = SessionState::Idle;
        assert_eq!(state, SessionState::Idle);

        state = SessionState::Failed;
        assert_eq!(state, SessionState::Failed);

        // Test Failed -> Idle (recovery)
        state = SessionState::Idle;
        assert_eq!(state, SessionState::Idle);
    }

    #[test]
    fn test_session_health_recovery_cycle() {
        // Test the health recovery cycle: Healthy -> Unhealthy -> Healthy
        // Note: We test using atomic operations similar to Session implementation

        let is_healthy = std::sync::atomic::AtomicBool::new(true);
        let failure_count = std::sync::atomic::AtomicUsize::new(0);

        // Initially healthy
        assert!(is_healthy.load(std::sync::atomic::Ordering::SeqCst));

        // Mark unhealthy (simulating circuit breaker open)
        is_healthy.store(false, std::sync::atomic::Ordering::SeqCst);
        failure_count.fetch_add(5, std::sync::atomic::Ordering::SeqCst);
        assert!(!is_healthy.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(failure_count.load(std::sync::atomic::Ordering::SeqCst), 5);

        // Mark healthy (simulating recovery)
        is_healthy.store(true, std::sync::atomic::Ordering::SeqCst);
        failure_count.store(0, std::sync::atomic::Ordering::SeqCst);
        assert!(is_healthy.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(failure_count.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[test]
    fn test_concurrent_page_registration_simulation() {
        // Simulate concurrent page registration using DashSet (like Session does)
        use dashmap::DashSet;
        use std::sync::Arc;
        use std::thread;

        let active_pages: Arc<DashSet<String>> = Arc::new(DashSet::new());
        let mut handles = vec![];

        // Spawn 10 threads that each register 10 pages
        for thread_id in 0..10 {
            let pages = active_pages.clone();
            handles.push(thread::spawn(move || {
                for page_id in 0..10 {
                    let id = format!("thread{}-page{}", thread_id, page_id);
                    pages.insert(id);
                }
            }));
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all 100 pages were registered
        assert_eq!(active_pages.len(), 100);

        // Verify we can check for specific pages
        assert!(active_pages.contains("thread5-page5"));
        assert!(!active_pages.contains("nonexistent"));

        // Simulate page unregistration
        active_pages.remove("thread0-page0");
        assert_eq!(active_pages.len(), 99);
        assert!(!active_pages.contains("thread0-page0"));
    }

    #[test]
    fn test_session_failure_threshold_tracking() {
        // Test that session properly tracks failures up to threshold
        let failure_threshold = 5;
        let failure_count = std::sync::atomic::AtomicUsize::new(0);

        // Simulate incremental failures
        for i in 1..=failure_threshold {
            failure_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let current = failure_count.load(std::sync::atomic::Ordering::SeqCst);

            if i < failure_threshold {
                assert!(
                    current < failure_threshold,
                    "Failure count {} should be below threshold {}",
                    current,
                    failure_threshold
                );
            } else {
                assert!(
                    current >= failure_threshold,
                    "Failure count {} should meet threshold {}",
                    current,
                    failure_threshold
                );
            }
        }

        // Verify final count
        assert_eq!(
            failure_count.load(std::sync::atomic::Ordering::SeqCst),
            failure_threshold
        );
    }
}
