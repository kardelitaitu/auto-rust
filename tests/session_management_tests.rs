//! Session management integration tests.
//!
//! Tests session lifecycle, concurrent access, and recovery scenarios.
//! Requires live browser via TASK_API_TEST_WS environment variable.

use auto::session::{Session, SessionState};
use chromiumoxide::Browser;

async fn connect_test_session() -> anyhow::Result<Option<Session>> {
    let ws_url = match std::env::var("TASK_API_TEST_WS") {
        Ok(url) => url,
        Err(_) => return Ok(None),
    };
    let (browser, handler) = Browser::connect(&ws_url).await?;
    let session = Session::new(
        "test-session-mgmt".to_string(),
        "Session Management Test".to_string(),
        "brave".to_string(),
        browser,
        handler,
        5, // max_workers
        0,  // cursor_overlay_ms
        None, // circuit_breaker_config
    );
    Ok(Some(session))
}

// ============================================================================
// Lifecycle Tests
// ============================================================================

#[tokio::test]
async fn test_session_lifecycle_initial_state() -> anyhow::Result<()> {
    let Some(mut session) = connect_test_session().await? else {
        return Ok(());
    };
    assert_eq!(session.state(), SessionState::Idle);
    assert!(session.is_healthy());
    assert_eq!(session.get_failure_count(), 0);
    assert_eq!(session.active_page_count(), 0);
    session.graceful_shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_session_lifecycle_state_transitions() -> anyhow::Result<()> {
    let Some(mut session) = connect_test_session().await? else {
        return Ok(());
    };
    // Idle -> Busy
    session.set_state(SessionState::Busy);
    assert!(session.is_busy());
    assert!(!session.is_idle());

    // Busy -> Failed
    session.set_state(SessionState::Failed);
    assert!(!session.is_idle());
    assert!(!session.is_busy());

    // Failed -> Idle (recovery)
    session.set_state(SessionState::Idle);
    assert!(session.is_idle());
    assert!(!session.is_busy());

    session.graceful_shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_session_lifecycle_shutdown_marks_failed() -> anyhow::Result<()> {
    let Some(mut session) = connect_test_session().await? else {
        return Ok(());
    };
    session.graceful_shutdown().await?;
    assert_eq!(session.state(), SessionState::Failed);
    Ok(())
}

// ============================================================================
// Recovery Scenarios Tests
// ============================================================================

#[tokio::test]
async fn test_circuit_breaker_recovery_flow() -> anyhow::Result<()> {
    let Some(mut session) = connect_test_session().await? else {
        return Ok(());
    };
    let threshold = session.get_circuit_breaker_threshold();
    let timeout_secs = session.get_circuit_breaker_timeout_secs();

    session.reset_circuit_breaker();
    assert!(!session.is_circuit_breaker_open());

    // Trigger threshold failures
    session.set_circuit_breaker_failure_count(threshold);
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    session.set_circuit_breaker_last_failure_time(current_time);

    assert!(session.is_circuit_breaker_open());
    assert!(!session.is_healthy());

    // Expire timeout
    session.set_circuit_breaker_last_failure_time(
        current_time - (timeout_secs as usize + 10)
    );
    assert!(!session.is_circuit_breaker_open());

    // Reset and recover
    session.reset_circuit_breaker();
    assert!(session.is_healthy());
    assert!(!session.is_circuit_breaker_open());

    session.graceful_shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_failure_count_tracking() -> anyhow::Result<()> {
    let Some(mut session) = connect_test_session().await? else {
        return Ok(());
    };
    assert_eq!(session.get_failure_count(), 0);

    session.increment_failure();
    assert_eq!(session.get_failure_count(), 1);

    session.increment_failure();
    assert_eq!(session.get_failure_count(), 2);

    session.graceful_shutdown().await?;
    Ok(())
}

// ============================================================================
// Active Page Tracking Test
// ============================================================================

#[tokio::test]
async fn test_session_active_page_tracking() -> anyhow::Result<()> {
    let Some(mut session) = connect_test_session().await? else {
        return Ok(());
    };
    assert_eq!(session.active_page_count(), 0);

    let page = session.acquire_page().await?;
    assert_eq!(session.active_page_count(), 1);

    session.release_page(page).await;
    assert_eq!(session.active_page_count(), 0);

    session.graceful_shutdown().await?;
    Ok(())
}
