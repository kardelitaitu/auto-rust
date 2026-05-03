//! Shared types for the task_context module.
//!
//! This module contains types used across multiple submodules of task_context,
//! including outcome structs, HTTP response types, and file metadata.

use crate::utils::mouse::CursorMovementConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
