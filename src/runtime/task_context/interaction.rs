//! Interaction methods for browser automation.
//!
//! This module provides standalone interaction functions that can be used
//! independently or through TaskContext. Complex methods requiring TaskContext
//! state remain in the main TaskContext impl.

use anyhow::Result;
use chromiumoxide::Page;

// Re-export keyboard operations
pub use crate::capabilities::keyboard::{press, press_with_modifiers};

// Re-export clipboard operations
pub use crate::capabilities::clipboard::{copy, cut};

/// Paste clipboard content into focused element.
pub async fn paste(session_id: &str, page: &Page) -> Result<String> {
    crate::capabilities::clipboard::paste_from_clipboard(session_id, page).await
}

// Re-export scroll operations
pub use crate::capabilities::scroll::{back, scroll_into_view};
