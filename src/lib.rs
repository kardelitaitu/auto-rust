//! Public browser automation framework surface.
//!
//! Tasks should depend on `TaskContext` and the capability/state modules,
//! not on the lower-level utilities directly.

pub mod api;
pub mod browser;
pub mod capabilities;
pub mod cli;
pub mod config;
pub mod health_monitor;
pub mod internal;
pub mod logger;
pub mod metrics;
pub mod orchestrator;
pub mod page_manager;
pub mod result;
pub mod runtime;
pub mod session;
pub mod state;
pub mod tests;
pub mod utils;
pub mod validation;
pub mod worker_pool;

#[path = "../task/mod.rs"]
pub mod task;

pub use runtime::task_context::TaskContext;
pub use state::ClipboardState;

/// Convenience imports for task authors.
pub mod prelude {
    pub use crate::capabilities::{clipboard, keyboard, mouse, navigation, scroll, timing};
    pub use crate::{ClipboardState, TaskContext};
}
