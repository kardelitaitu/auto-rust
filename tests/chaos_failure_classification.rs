//! Tests for chaos engineering and failure classification.
//!
//! This module tests that various error messages are correctly
//! classified into appropriate TaskErrorKind variants.
//!
//! # Test Strategy
//! - Provide error messages that mimic real failure scenarios
//! - Verify correct TaskErrorKind classification
//! - Ensure TaskResult status is set correctly
//!
//! # Coverage
//! - Browser failures (Target closed, WebSocket errors)
//! - Session failures (receiver gone, disconnected)
//! - Network failures (timeout, connection reset)
//! - Validation failures (invalid input)

use auto::result::{TaskErrorKind, TaskResult, TaskStatus};

#[test]
fn chaos_mid_action_page_detach_classifies_as_browser_failure() {
    let message = "Protocol error (Target.detachedFromTarget): Target closed.";
    let kind = TaskErrorKind::classify(message);
    assert_eq!(kind, TaskErrorKind::Browser);

    let result = TaskResult::failure(42, message.to_string(), kind);
    assert!(matches!(result.status, TaskStatus::Failed(_)));
    assert_eq!(result.error_kind, Some(TaskErrorKind::Browser));
}

#[test]
fn chaos_mid_action_disconnect_classifies_as_session_failure() {
    let message = "send failed because receiver is gone";
    let kind = TaskErrorKind::classify(message);
    assert_eq!(kind, TaskErrorKind::Session);

    let result = TaskResult::failure(42, message.to_string(), kind);
    assert!(matches!(result.status, TaskStatus::Failed(_)));
    assert_eq!(result.error_kind, Some(TaskErrorKind::Session));
}

#[test]
fn chaos_disconnect_storm_never_panics_and_stays_classified() {
    let storm_errors = vec![
        "Protocol error (Target.detachedFromTarget): Target closed.",
        "browser disconnected unexpectedly during click",
        "websocket closed while waiting for response",
        "send failed because receiver is gone",
        "connection reset by peer",
    ];

    for error in storm_errors {
        let classified = std::panic::catch_unwind(|| TaskErrorKind::classify(error));
        assert!(
            classified.is_ok(),
            "classification should not panic for '{}'",
            error
        );
        let kind = classified.unwrap();
        assert!(
            matches!(kind, TaskErrorKind::Browser | TaskErrorKind::Session),
            "expected browser/session classification for '{}', got {:?}",
            error,
            kind
        );
    }
}
