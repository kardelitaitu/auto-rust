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
