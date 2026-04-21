//! Public browser automation framework surface.
//!
//! Task-api verbs live on `TaskContext` and follow the short `api.*`
//! style:
//! - `click`, `double_click`, `right_click`, `hover`
//! - `focus`, `keyboard`, `randomcursor`
//! - `clear`, `select_all`
//! - `exists`, `visible`, `text`, `wait_for`, `wait_for_visible`
//! - `scroll_to`, `url`, `title`
//!
//! Tasks should depend on `TaskContext` and the capability/state modules,
//! not on the lower-level utilities directly.

pub mod api;
pub mod browser;
pub mod capabilities;
pub mod cli;
pub mod config;
pub mod health_logger;
pub mod health_monitor;
pub mod internal;
pub mod llm;
pub mod logger;
pub mod metrics;
pub mod orchestrator;
pub mod result;
pub mod runtime;
pub mod session;
pub mod state;
pub mod tests;
pub mod utils;
pub mod validation;

#[path = "../task/mod.rs"]
pub mod task;

pub use llm::{ChatMessage, Llm, LlmClient, LlmProvider};
pub use runtime::task_context::TaskContext;
pub use state::ClipboardState;

/// Convenience imports for task authors.
pub mod prelude {
    pub use crate::capabilities::{clipboard, keyboard, mouse, navigation, scroll, timing};
    pub use crate::{ClipboardState, TaskContext};
}
