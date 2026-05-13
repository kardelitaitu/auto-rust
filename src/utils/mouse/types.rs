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
        format!("hover:{} ({:.1},{:.1})", status, self.x, self.y)
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
        format!("nativecursor {} ({:.1},{:.1})", self.target, self.x, self.y)
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // ClickStatus Tests
    // ========================================================================

    #[test]
    fn test_click_status_variants() {
        assert_eq!(ClickStatus::Success as u8, 0);
        assert_eq!(ClickStatus::Failed as u8, 1);
    }

    #[test]
    fn test_click_status_partial_eq() {
        assert_eq!(ClickStatus::Success, ClickStatus::Success);
        assert_ne!(ClickStatus::Success, ClickStatus::Failed);
    }

    // ========================================================================
    // ClickOutcome Tests
    // ========================================================================

    #[test]
    fn test_click_outcome_summary_success() {
        let outcome = ClickOutcome {
            click: ClickStatus::Success,
            x: 100.5,
            y: 200.3,
            screen_x: None,
            screen_y: None,
        };
        let summary = outcome.summary();
        assert_eq!(summary, "Clicked (100.5,200.3)");
    }

    #[test]
    fn test_click_outcome_summary_failed() {
        let outcome = ClickOutcome {
            click: ClickStatus::Failed,
            x: 0.0,
            y: 0.0,
            screen_x: None,
            screen_y: None,
        };
        let summary = outcome.summary();
        assert_eq!(summary, "Click failed (0.0,0.0)");
    }

    #[test]
    fn test_click_outcome_copy() {
        let outcome = ClickOutcome {
            click: ClickStatus::Success,
            x: 1.0,
            y: 2.0,
            screen_x: Some(100),
            screen_y: Some(200),
        };
        let copied = outcome;
        assert_eq!(copied.screen_x, Some(100));
    }

    // ========================================================================
    // HoverStatus Tests
    // ========================================================================

    #[test]
    fn test_hover_status_variants() {
        assert_eq!(HoverStatus::Success as u8, 0);
        assert_eq!(HoverStatus::Failed as u8, 1);
    }

    #[test]
    fn test_hover_status_partial_eq() {
        assert_eq!(HoverStatus::Success, HoverStatus::Success);
        assert_ne!(HoverStatus::Success, HoverStatus::Failed);
    }

    // ========================================================================
    // HoverOutcome Tests
    // ========================================================================

    #[test]
    fn test_hover_outcome_summary_success() {
        let outcome = HoverOutcome {
            hover: HoverStatus::Success,
            x: 50.0,
            y: 75.5,
        };
        let summary = outcome.summary();
        assert_eq!(summary, "hover:success (50.0,75.5)");
    }

    #[test]
    fn test_hover_outcome_summary_failed() {
        let outcome = HoverOutcome {
            hover: HoverStatus::Failed,
            x: -1.0,
            y: -1.0,
        };
        let summary = outcome.summary();
        assert_eq!(summary, "hover:failed (-1.0,-1.0)");
    }

    #[test]
    fn test_hover_outcome_copy() {
        let outcome = HoverOutcome {
            hover: HoverStatus::Success,
            x: 10.0,
            y: 20.0,
        };
        let copied = outcome;
        assert_eq!(copied.hover, HoverStatus::Success);
    }

    // ========================================================================
    // NativeCursorOutcome Tests
    // ========================================================================

    #[test]
    fn test_native_cursor_outcome_summary() {
        let outcome = NativeCursorOutcome {
            target: "#button".to_string(),
            x: 300.0,
            y: 400.0,
            screen_x: Some(1920),
            screen_y: Some(1080),
        };
        let summary = outcome.summary();
        assert_eq!(summary, "nativecursor #button (300.0,400.0)");
    }

    #[test]
    fn test_native_cursor_outcome_no_screen_coords() {
        let outcome = NativeCursorOutcome {
            target: ".link".to_string(),
            x: 150.0,
            y: 250.0,
            screen_x: None,
            screen_y: None,
        };
        let summary = outcome.summary();
        assert_eq!(summary, "nativecursor .link (150.0,250.0)");
    }

    #[test]
    fn test_native_cursor_outcome_empty_target() {
        let outcome = NativeCursorOutcome {
            target: String::new(),
            x: 0.0,
            y: 0.0,
            screen_x: None,
            screen_y: None,
        };
        assert!(outcome.summary().contains("nativecursor"));
    }

    // ========================================================================
    // MouseButton Tests
    // ========================================================================

    #[test]
    fn test_mouse_button_default() {
        let button = MouseButton::default();
        assert_eq!(button, MouseButton::Left);
    }

    #[test]
    fn test_mouse_button_variants() {
        assert_eq!(MouseButton::Left as u8, 0);
        assert_eq!(MouseButton::Right as u8, 1);
        assert_eq!(MouseButton::Middle as u8, 2);
    }

    #[test]
    fn test_mouse_button_partial_eq() {
        assert_eq!(MouseButton::Left, MouseButton::Left);
        assert_ne!(MouseButton::Left, MouseButton::Right);
    }
}
