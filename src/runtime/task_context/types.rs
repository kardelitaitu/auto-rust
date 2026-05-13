//! Shared types for the task_context module.
//!
//! This module contains types used across multiple submodules of task_context,
//! including outcome structs, HTTP response types, and file metadata.

use crate::utils::mouse::CursorMovementConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Interaction Pipeline Types
// ============================================================================
/// The kind of interaction being performed through the shared pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionKind {
    /// Click with human-like cursor movement
    Click,
    /// Native OS-level click
    NativeClick,
    /// Type text with human-like timing
    Type,
    /// Press individual keys
    Keyboard,
    /// Focus an element
    Focus,
    /// Select all text in an element
    SelectAll,
    /// Clear input content
    Clear,
    /// Hover over an element
    Hover,
}

/// Request for an interaction through the shared pipeline.
#[derive(Debug, Clone)]
pub struct InteractionRequest {
    /// The kind of interaction to perform
    pub kind: InteractionKind,
    /// The target selector
    pub selector: String,
    /// Optional text for type/keyboard actions
    pub text: Option<String>,
    /// Whether to verify the interaction succeeded
    pub verify: bool,
    /// Whether to allow fallback behavior on failure
    pub allow_fallback: bool,
    /// Minimum pause after interaction (ms)
    pub post_action_pause_ms: u64,
}

impl InteractionRequest {
    /// Create a new click interaction request
    pub fn click(selector: impl Into<String>) -> Self {
        Self {
            kind: InteractionKind::Click,
            selector: selector.into(),
            text: None,
            verify: true,
            allow_fallback: true,
            post_action_pause_ms: 120,
        }
    }

    /// Create a new type interaction request
    pub fn type_text(selector: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            kind: InteractionKind::Type,
            selector: selector.into(),
            text: Some(text.into()),
            verify: true,
            allow_fallback: true,
            post_action_pause_ms: 120,
        }
    }

    /// Create a new focus interaction request
    pub fn focus(selector: impl Into<String>) -> Self {
        Self {
            kind: InteractionKind::Focus,
            selector: selector.into(),
            text: None,
            verify: true,
            allow_fallback: false,
            post_action_pause_ms: 80,
        }
    }

    /// Create a new clear interaction request
    pub fn clear(selector: impl Into<String>) -> Self {
        Self {
            kind: InteractionKind::Clear,
            selector: selector.into(),
            text: None,
            verify: true,
            allow_fallback: false,
            post_action_pause_ms: 100,
        }
    }

    /// Create a new select_all interaction request
    pub fn select_all(selector: impl Into<String>) -> Self {
        Self {
            kind: InteractionKind::SelectAll,
            selector: selector.into(),
            text: None,
            verify: true,
            allow_fallback: false,
            post_action_pause_ms: 80,
        }
    }

    /// Disable verification for this interaction
    pub fn without_verification(mut self) -> Self {
        self.verify = false;
        self
    }

    /// Disable fallback for this interaction
    pub fn without_fallback(mut self) -> Self {
        self.allow_fallback = false;
        self
    }

    /// Set a custom post-action pause
    pub fn with_pause(mut self, ms: u64) -> Self {
        self.post_action_pause_ms = ms;
        self
    }
}

/// Result of an interaction through the shared pipeline.
#[derive(Debug, Clone)]
pub struct InteractionResult {
    /// Whether the interaction succeeded
    pub success: bool,
    /// Whether fallback was used to achieve success
    pub fallback_used: bool,
    /// Whether verification was performed and passed
    pub verified: bool,
    /// X coordinate of the interaction (if applicable)
    pub x: Option<f64>,
    /// Y coordinate of the interaction (if applicable)
    pub y: Option<f64>,
    /// Error message if interaction failed
    pub error: Option<String>,
}

impl InteractionResult {
    /// Create a successful result
    pub fn success() -> Self {
        Self {
            success: true,
            fallback_used: false,
            verified: true,
            x: None,
            y: None,
            error: None,
        }
    }

    /// Create a successful result with coordinates
    pub fn success_at(x: f64, y: f64) -> Self {
        Self {
            success: true,
            fallback_used: false,
            verified: true,
            x: Some(x),
            y: Some(y),
            error: None,
        }
    }

    /// Create a successful result with fallback
    pub fn fallback_success() -> Self {
        Self {
            success: true,
            fallback_used: true,
            verified: true,
            x: None,
            y: None,
            error: None,
        }
    }

    /// Create a failed result
    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            success: false,
            fallback_used: false,
            verified: false,
            x: None,
            y: None,
            error: Some(error.into()),
        }
    }

    /// Check if result is success
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// Check if fallback was used
    pub fn is_fallback(&self) -> bool {
        self.fallback_used
    }
}

/// HTTP response structure for network operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    /// HTTP status code
    pub status: u16,
    /// Response body as string
    pub body: String,
    /// Response headers
    pub headers: HashMap<String, String>,
}

/// Rectangle for element position and size.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rect {
    /// X coordinate (left edge)
    pub x: f64,
    /// Y coordinate (top edge)
    pub y: f64,
    /// Width in pixels
    pub width: f64,
    /// Height in pixels
    pub height: f64,
}

/// Metadata for a data file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File size in bytes
    pub size: u64,
    /// Last modification time
    pub modified: std::time::SystemTime,
    /// Creation time
    pub created: std::time::SystemTime,
}

/// Status of a focus operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusStatus {
    Success,
    Failed,
}

/// Outcome of a focus operation.
#[derive(Debug, Clone, Copy)]
pub struct FocusOutcome {
    pub focus: FocusStatus,
    pub x: f64,
    pub y: f64,
}

impl FocusOutcome {
    pub fn summary(&self) -> String {
        let status = match self.focus {
            FocusStatus::Success => "success",
            FocusStatus::Failed => "failed",
        };
        format!("focus:{status} ({:.1},{:.1})", self.x, self.y)
    }
}

/// Outcome of a random cursor movement operation.
#[derive(Debug, Clone, Copy)]
pub struct RandomCursorOutcome {
    pub x: f64,
    pub y: f64,
    pub movement: CursorMovementConfig,
}

impl RandomCursorOutcome {
    pub fn summary(&self) -> String {
        format!(
            "randomcursor ({:.1},{:.1}) delay:{}..{}",
            self.x,
            self.y,
            self.movement.min_step_delay_ms,
            self.movement
                .min_step_delay_ms
                .saturating_add(self.movement.max_step_delay_variance_ms)
        )
    }
}

/// Outcome of a click-and-wait operation.
#[derive(Debug, Clone)]
pub struct ClickAndWaitOutcome {
    pub click: crate::utils::mouse::ClickOutcome,
    pub next_selector: String,
    pub next_visible: WaitForVisibleStatus,
    pub timeout_ms: u64,
}

impl ClickAndWaitOutcome {
    pub fn summary(&self) -> String {
        let next_visible = match self.next_visible {
            WaitForVisibleStatus::Visible => "visible",
            WaitForVisibleStatus::Timeout => "timeout",
        };
        format!(
            "{} wait_for:{} visible:{} timeout:{}ms",
            self.click.summary(),
            self.next_selector,
            next_visible,
            self.timeout_ms
        )
    }
}

/// Status of a wait-for-visible operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitForVisibleStatus {
    Visible,
    Timeout,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // InteractionRequest Tests
    // ========================================================================

    #[test]
    fn test_interaction_request_click_defaults() {
        let req = InteractionRequest::click("#btn");
        assert_eq!(req.kind, InteractionKind::Click);
        assert_eq!(req.selector, "#btn");
        assert!(req.text.is_none());
        assert!(req.verify);
        assert!(req.allow_fallback);
        assert_eq!(req.post_action_pause_ms, 120);
    }

    #[test]
    fn test_interaction_request_type_text_defaults() {
        let req = InteractionRequest::type_text("#input", "hello");
        assert_eq!(req.kind, InteractionKind::Type);
        assert_eq!(req.selector, "#input");
        assert_eq!(req.text.as_deref(), Some("hello"));
        assert!(req.verify);
        assert!(req.allow_fallback);
        assert_eq!(req.post_action_pause_ms, 120);
    }

    #[test]
    fn test_interaction_request_focus_defaults() {
        let req = InteractionRequest::focus("#field");
        assert_eq!(req.kind, InteractionKind::Focus);
        assert_eq!(req.selector, "#field");
        assert!(req.text.is_none());
        assert!(req.verify);
        assert!(!req.allow_fallback);
        assert_eq!(req.post_action_pause_ms, 80);
    }

    #[test]
    fn test_interaction_request_clear_defaults() {
        let req = InteractionRequest::clear("#input");
        assert_eq!(req.kind, InteractionKind::Clear);
        assert_eq!(req.selector, "#input");
        assert!(req.verify);
        assert!(!req.allow_fallback);
        assert_eq!(req.post_action_pause_ms, 100);
    }

    #[test]
    fn test_interaction_request_select_all_defaults() {
        let req = InteractionRequest::select_all("#textarea");
        assert_eq!(req.kind, InteractionKind::SelectAll);
        assert!(req.verify);
        assert!(!req.allow_fallback);
        assert_eq!(req.post_action_pause_ms, 80);
    }

    #[test]
    fn test_interaction_request_without_verification() {
        let req = InteractionRequest::click("#btn").without_verification();
        assert!(!req.verify);
        assert!(req.allow_fallback);
    }

    #[test]
    fn test_interaction_request_without_fallback() {
        let req = InteractionRequest::click("#btn").without_fallback();
        assert!(req.verify);
        assert!(!req.allow_fallback);
    }

    #[test]
    fn test_interaction_request_with_pause() {
        let req = InteractionRequest::click("#btn").with_pause(500);
        assert_eq!(req.post_action_pause_ms, 500);
    }

    #[test]
    fn test_interaction_request_chained_modifiers() {
        let req = InteractionRequest::type_text("#input", "text")
            .without_verification()
            .without_fallback()
            .with_pause(250);
        assert!(!req.verify);
        assert!(!req.allow_fallback);
        assert_eq!(req.post_action_pause_ms, 250);
        assert_eq!(req.text.as_deref(), Some("text"));
    }

    #[test]
    fn test_interaction_kind_variants() {
        assert_eq!(InteractionKind::Click as u8, 0);
        assert_eq!(InteractionKind::NativeClick as u8, 1);
        assert_eq!(InteractionKind::Type as u8, 2);
        assert_eq!(InteractionKind::Keyboard as u8, 3);
        assert_eq!(InteractionKind::Focus as u8, 4);
        assert_eq!(InteractionKind::SelectAll as u8, 5);
        assert_eq!(InteractionKind::Clear as u8, 6);
        assert_eq!(InteractionKind::Hover as u8, 7);
    }

    #[test]
    fn test_interaction_kind_debug() {
        let kind = InteractionKind::Click;
        assert_eq!(format!("{:?}", kind), "Click");
        assert_eq!(format!("{:?}", InteractionKind::Hover), "Hover");
    }

    // ========================================================================
    // InteractionResult Tests
    // ========================================================================

    #[test]
    fn test_interaction_result_success() {
        let result = InteractionResult::success();
        assert!(result.success);
        assert!(!result.fallback_used);
        assert!(result.verified);
        assert!(result.x.is_none());
        assert!(result.y.is_none());
        assert!(result.error.is_none());
        assert!(result.is_success());
        assert!(!result.is_fallback());
    }

    #[test]
    fn test_interaction_result_success_at() {
        let result = InteractionResult::success_at(100.5, 200.3);
        assert!(result.success);
        assert!(result.verified);
        assert_eq!(result.x, Some(100.5));
        assert_eq!(result.y, Some(200.3));
    }

    #[test]
    fn test_interaction_result_fallback_success() {
        let result = InteractionResult::fallback_success();
        assert!(result.success);
        assert!(result.fallback_used);
        assert!(result.verified);
        assert!(result.is_fallback());
    }

    #[test]
    fn test_interaction_result_failed() {
        let result = InteractionResult::failed("element not found");
        assert!(!result.success);
        assert!(!result.fallback_used);
        assert!(!result.verified);
        assert_eq!(result.error.as_deref(), Some("element not found"));
        assert!(!result.is_success());
    }

    #[test]
    fn test_interaction_result_failed_empty() {
        let result = InteractionResult::failed("");
        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some(""));
    }

    // ========================================================================
    // HttpResponse Tests
    // ========================================================================

    #[test]
    fn test_http_response_creation() {
        let response = HttpResponse {
            status: 200,
            body: "OK".to_string(),
            headers: {
                let mut m = std::collections::HashMap::new();
                m.insert("content-type".to_string(), "application/json".to_string());
                m
            },
        };
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "OK");
        assert_eq!(
            response.headers.get("content-type").map(|s| s.as_str()),
            Some("application/json")
        );
    }

    #[test]
    fn test_http_response_error_status() {
        let response = HttpResponse {
            status: 404,
            body: "Not Found".to_string(),
            headers: std::collections::HashMap::new(),
        };
        assert_eq!(response.status, 404);
    }

    #[test]
    fn test_http_response_empty_body() {
        let response = HttpResponse {
            status: 204,
            body: String::new(),
            headers: std::collections::HashMap::new(),
        };
        assert!(response.body.is_empty());
    }

    #[test]
    fn test_http_response_serialize_roundtrip() {
        let response = HttpResponse {
            status: 200,
            body: "{\"key\":\"value\"}".to_string(),
            headers: {
                let mut m = std::collections::HashMap::new();
                m.insert("x-request-id".to_string(), "abc".to_string());
                m
            },
        };
        let json = serde_json::to_string(&response).expect("serialize");
        let restored: HttpResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.status, response.status);
        assert_eq!(restored.body, response.body);
        assert_eq!(
            restored.headers.get("x-request-id").map(|s| s.as_str()),
            Some("abc")
        );
    }

    // ========================================================================
    // Rect Tests
    // ========================================================================

    #[test]
    fn test_rect_creation() {
        let rect = Rect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 200.0,
        };
        assert_eq!(rect.x, 10.0);
        assert_eq!(rect.y, 20.0);
        assert_eq!(rect.width, 100.0);
        assert_eq!(rect.height, 200.0);
    }

    #[test]
    fn test_rect_zero_size() {
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        };
        assert_eq!(rect.width, 0.0);
        assert_eq!(rect.height, 0.0);
    }

    #[test]
    fn test_rect_serialize_roundtrip() {
        let rect = Rect {
            x: 1.5,
            y: 2.5,
            width: 3.5,
            height: 4.5,
        };
        let json = serde_json::to_string(&rect).expect("serialize");
        let restored: Rect = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.x, rect.x);
        assert_eq!(restored.y, rect.y);
        assert_eq!(restored.width, rect.width);
        assert_eq!(restored.height, rect.height);
    }

    // ========================================================================
    // FileMetadata Tests
    // ========================================================================

    #[test]
    fn test_file_metadata_creation() {
        let meta = FileMetadata {
            size: 1024,
            modified: std::time::SystemTime::now(),
            created: std::time::SystemTime::now(),
        };
        assert_eq!(meta.size, 1024);
    }

    #[test]
    fn test_file_metadata_zero_size() {
        let meta = FileMetadata {
            size: 0,
            modified: std::time::SystemTime::UNIX_EPOCH,
            created: std::time::SystemTime::UNIX_EPOCH,
        };
        assert_eq!(meta.size, 0);
    }

    // ========================================================================
    // FocusOutcome Tests
    // ========================================================================

    #[test]
    fn test_focus_outcome_summary_success() {
        let outcome = FocusOutcome {
            focus: FocusStatus::Success,
            x: 100.0,
            y: 200.0,
        };
        let summary = outcome.summary();
        assert!(summary.contains("success"));
        assert!(summary.contains("100.0"));
        assert!(summary.contains("200.0"));
    }

    #[test]
    fn test_focus_outcome_summary_failed() {
        let outcome = FocusOutcome {
            focus: FocusStatus::Failed,
            x: 0.0,
            y: 0.0,
        };
        let summary = outcome.summary();
        assert!(summary.contains("failed"));
    }

    #[test]
    fn test_focus_status_variants() {
        assert_eq!(FocusStatus::Success as u8, 0);
        assert_eq!(FocusStatus::Failed as u8, 1);
    }

    #[test]
    fn test_focus_status_partial_eq() {
        assert_eq!(FocusStatus::Success, FocusStatus::Success);
        assert_ne!(FocusStatus::Success, FocusStatus::Failed);
    }

    // ========================================================================
    // RandomCursorOutcome Tests
    // ========================================================================

    #[test]
    fn test_random_cursor_outcome_summary() {
        let movement = CursorMovementConfig {
            min_step_delay_ms: 5,
            max_step_delay_variance_ms: 15,
            ..Default::default()
        };
        let outcome = RandomCursorOutcome {
            x: 50.0,
            y: 75.0,
            movement,
        };
        let summary = outcome.summary();
        assert!(summary.contains("50.0"));
        assert!(summary.contains("75.0"));
        assert!(summary.contains("5..20"));
    }

    // ========================================================================
    // ClickAndWaitOutcome Tests
    // ========================================================================

    #[test]
    fn test_click_and_wait_outcome_summary_visible() {
        use crate::utils::mouse::types::{ClickOutcome, ClickStatus};
        let outcome = ClickAndWaitOutcome {
            click: ClickOutcome {
                click: ClickStatus::Success,
                x: 100.0,
                y: 200.0,
                screen_x: None,
                screen_y: None,
            },
            next_selector: "#next".to_string(),
            next_visible: WaitForVisibleStatus::Visible,
            timeout_ms: 5000,
        };
        let summary = outcome.summary();
        assert!(summary.contains("visible"));
        assert!(summary.contains("#next"));
        assert!(summary.contains("5000ms"));
    }

    #[test]
    fn test_click_and_wait_outcome_summary_timeout() {
        use crate::utils::mouse::types::{ClickOutcome, ClickStatus};
        let outcome = ClickAndWaitOutcome {
            click: ClickOutcome {
                click: ClickStatus::Failed,
                x: 0.0,
                y: 0.0,
                screen_x: None,
                screen_y: None,
            },
            next_selector: "#target".to_string(),
            next_visible: WaitForVisibleStatus::Timeout,
            timeout_ms: 10000,
        };
        let summary = outcome.summary();
        assert!(summary.contains("timeout"));
        assert!(summary.contains("10000ms"));
    }

    // ========================================================================
    // WaitForVisibleStatus Tests
    // ========================================================================

    #[test]
    fn test_wait_for_visible_status_variants() {
        assert_eq!(WaitForVisibleStatus::Visible, WaitForVisibleStatus::Visible);
        assert_eq!(WaitForVisibleStatus::Timeout, WaitForVisibleStatus::Timeout);
        assert_ne!(WaitForVisibleStatus::Visible, WaitForVisibleStatus::Timeout);
    }

    #[test]
    fn test_wait_for_visible_status_debug() {
        assert_eq!(format!("{:?}", WaitForVisibleStatus::Visible), "Visible");
        assert_eq!(format!("{:?}", WaitForVisibleStatus::Timeout), "Timeout");
    }
}
