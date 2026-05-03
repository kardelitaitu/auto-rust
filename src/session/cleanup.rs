//! Session cleanup utilities.
//!
//! Provides functionality for graceful session shutdown and
//! cleanup of managed browser tabs/pages.

use async_trait::async_trait;
use log::warn;

use crate::session::Session;

/// Trait for graceful session shutdown (test-only trait).
#[cfg(test)]
#[async_trait(?Send)]
pub trait ShutdownSession {
    /// Perform graceful shutdown of the session.
    async fn shutdown(&mut self) -> anyhow::Result<()>;
}

#[cfg(test)]
#[async_trait(?Send)]
impl ShutdownSession for Session {
    async fn shutdown(&mut self) -> anyhow::Result<()> {
        Session::graceful_shutdown(self).await
    }
}

/// Shutdown multiple sessions gracefully.
#[cfg(test)]
pub async fn shutdown_sessions<T: ShutdownSession>(sessions: &mut [T]) {
    for session in sessions.iter_mut() {
        if let Err(e) = session.shutdown().await {
            warn!("Failed to shut down session: {e}");
        }
    }
}

/// Trait for managed tab cleanup.
///
/// Exposed so cleanup orchestration can be tested with lightweight mocks.
#[async_trait(?Send)]
pub trait ManagedTabCleanup {
    /// Stable session identifier used for diagnostics.
    fn session_id(&self) -> &str;

    /// Clean up managed tabs and return the number of tabs closed.
    async fn cleanup_managed_pages(&self) -> anyhow::Result<usize>;
}

#[async_trait(?Send)]
impl ManagedTabCleanup for Session {
    fn session_id(&self) -> &str {
        &self.id
    }

    async fn cleanup_managed_pages(&self) -> anyhow::Result<usize> {
        Session::cleanup_managed_pages(self).await
    }
}

/// Clean up managed tabs in all sessions.
///
/// Iterates through all sessions and closes any tabs that were
/// opened and managed by the orchestrator during task execution.
pub async fn cleanup_managed_tabs<T: ManagedTabCleanup>(sessions: &[T]) {
    let mut total_closed = 0usize;
    for session in sessions {
        match session.cleanup_managed_pages().await {
            Ok(closed) => total_closed += closed,
            Err(e) => warn!(
                "Failed to clean up managed tabs for {}: {e}",
                session.session_id()
            ),
        }
    }

    if total_closed > 0 {
        log::info!("Closed {} managed tab(s) before exit", total_closed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct MockShutdownSession {
        calls: Arc<AtomicUsize>,
        fail: bool,
    }

    #[async_trait(?Send)]
    impl ShutdownSession for MockShutdownSession {
        async fn shutdown(&mut self) -> anyhow::Result<()> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                anyhow::bail!("shutdown failed");
            }
            Ok(())
        }
    }

    struct MockManagedTabCleanup {
        id: &'static str,
        closed: usize,
        fail: bool,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait(?Send)]
    impl ManagedTabCleanup for MockManagedTabCleanup {
        fn session_id(&self) -> &str {
            self.id
        }

        async fn cleanup_managed_pages(&self) -> anyhow::Result<usize> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                anyhow::bail!("cleanup failed");
            }
            Ok(self.closed)
        }
    }

    #[tokio::test]
    async fn test_shutdown_sessions_calls_all_entries() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut sessions = vec![
            MockShutdownSession {
                calls: calls.clone(),
                fail: false,
            },
            MockShutdownSession {
                calls: calls.clone(),
                fail: true,
            },
            MockShutdownSession {
                calls: calls.clone(),
                fail: false,
            },
        ];

        shutdown_sessions(&mut sessions).await;
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_cleanup_managed_tabs_counts_all_sessions_and_continues_after_error() {
        let calls = Arc::new(AtomicUsize::new(0));
        let sessions = vec![
            MockManagedTabCleanup {
                id: "session-a",
                closed: 2,
                fail: false,
                calls: calls.clone(),
            },
            MockManagedTabCleanup {
                id: "session-b",
                closed: 0,
                fail: true,
                calls: calls.clone(),
            },
            MockManagedTabCleanup {
                id: "session-c",
                closed: 3,
                fail: false,
                calls: calls.clone(),
            },
        ];

        cleanup_managed_tabs(&sessions).await;
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_cleanup_managed_tabs_handles_empty_input() {
        let sessions: Vec<MockManagedTabCleanup> = vec![];
        cleanup_managed_tabs(&sessions).await;
    }
}
