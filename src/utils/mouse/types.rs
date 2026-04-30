//! Mouse types and outcome structures.
//!
//! This module provides types used across mouse interactions,
//! including click outcomes, hover outcomes, and mouse buttons.

/// Status of a click operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickStatus {
    Success,
    Failed,
}

/// Outcome of a click operation.
#[derive(Debug, Clone, Copy)]
pub struct ClickOutcome {
    pub click: ClickStatus,
    pub x: f64,
    pub y: f64,
    pub screen_x: Option<i32>,
    pub screen_y: Option<i32>,
}

impl ClickOutcome {
    /// Returns a summary string for logging.
    pub fn summary(&self) -> String {
        match self.click {
            ClickStatus::Success => format!("Clicked ({:.1},{:.1})", self.x, self.y),
            ClickStatus::Failed => format!("Click failed ({:.1},{:.1})", self.x, self.y),
        }
    }
}

/// Status of a hover operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverStatus {
    Success,
    Failed,
}

/// Outcome of a hover operation.
#[derive(Debug, Clone, Copy)]
pub struct HoverOutcome {
    pub hover: HoverStatus,
    pub x: f64,
    pub y: f64,
}

impl HoverOutcome {
    /// Returns a summary string for logging.
    pub fn summary(&self) -> String {
        let status = match self.hover {
            HoverStatus::Success => "success",
            HoverStatus::Failed => "failed",
        };
        format!(
            "hover:{} ({:.1},{:.1})",
            status, self.x, self.y
        )
    }
}

/// Outcome of a native cursor operation.
#[derive(Debug, Clone)]
pub struct NativeCursorOutcome {
    pub target: String,
    pub x: f64,
    pub y: f64,
    pub screen_x: Option<i32>,
    pub screen_y: Option<i32>,
}

impl NativeCursorOutcome {
    /// Returns a summary string for logging.
    pub fn summary(&self) -> String {
        format!(
            "nativecursor {} ({:.1},{:.1})",
            self.target, self.x, self.y
        )
    }
}

/// Mouse button types.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum MouseButton {
    #[default]
    Left,
    Right,
    Middle,
}
