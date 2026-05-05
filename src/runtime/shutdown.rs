//! Runtime shutdown coordination.
//!
//! Keeps shutdown signaling in the runtime layer so entry points do not need
//! to own signal wiring details.

use log::{info, warn};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

/// Coordinates cooperative shutdown requests across runtime components.
#[derive(Clone)]
pub struct ShutdownManager {
    tx: Arc<broadcast::Sender<()>>,
}

impl ShutdownManager {
    /// Create a shutdown manager with the default channel capacity.
    pub fn new() -> Self {
        Self::with_capacity(1)
    }

    /// Create a shutdown manager with an explicit channel capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity.max(1));
        Self { tx: Arc::new(tx) }
    }

    /// Subscribe to shutdown requests.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.tx.subscribe()
    }

    /// Request shutdown and notify active subscribers.
    pub fn request_shutdown(&self) -> bool {
        self.tx.send(()).is_ok()
    }

    /// Wait for the next shutdown request.
    pub async fn wait(&self) {
        let mut rx = self.subscribe();
        let _ = rx.recv().await;
    }

    /// Spawn the Ctrl+C listener that requests shutdown.
    pub fn spawn_ctrl_c_listener(&self) -> JoinHandle<()> {
        let manager = self.clone();
        tokio::spawn(async move {
            match tokio::signal::ctrl_c().await {
                Ok(()) => {
                    info!("\nReceived shutdown signal (Ctrl+C)");
                    let _ = manager.request_shutdown();
                }
                Err(e) => warn!("Failed to listen for Ctrl+C: {e}"),
            }
        })
    }
}

impl Default for ShutdownManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_request_shutdown_notifies_subscriber() {
        let manager = ShutdownManager::new();
        let mut rx = manager.subscribe();

        assert!(manager.request_shutdown());

        timeout(Duration::from_millis(50), rx.recv())
            .await
            .expect("shutdown signal should arrive")
            .expect("shutdown channel should be open");
    }

    #[tokio::test]
    async fn test_wait_returns_after_shutdown_request() {
        let manager = ShutdownManager::new();
        let waiter = {
            let manager = manager.clone();
            tokio::spawn(async move {
                manager.wait().await;
            })
        };

        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(manager.request_shutdown());

        timeout(Duration::from_millis(50), waiter)
            .await
            .expect("waiter should complete")
            .expect("waiter task should join");
    }

    #[tokio::test]
    async fn test_cloned_manager_shares_shutdown_channel() {
        let manager = ShutdownManager::new();
        let cloned = manager.clone();
        let mut rx = manager.subscribe();

        assert!(cloned.request_shutdown());

        timeout(Duration::from_millis(50), rx.recv())
            .await
            .expect("shutdown signal should arrive")
            .expect("shutdown channel should be open");
    }
}
