/*
last audited 08-05-25 by RSA-Agent
crate: auto-rust | status: SAFE | lint: CLEAN
findings: Zero unsafe blocks, concurrency patterns appropriate, 3 minor dependency concerns | next: clean test imports / verify notify+enigo platform compat | perf: Arc/RwLock for metrics is good; static Mutexes in native.rs are low-risk
*/

#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![cfg_attr(test, allow(clippy::unwrap_used))]
#![deny(clippy::expect_used)]
#![cfg_attr(test, allow(clippy::expect_used))]
#![deny(unsafe_op_in_unsafe_fn)]

//! Public browser automation framework surface.
//!
//! Task-api verbs live on `TaskContext` and follow the short `api.*`
//! style:
//! - `click`, `double_click`, `right_click`, `hover`
//! - `nativeclick` for OS-level mouse input
//! - `focus`, `keyboard`, `randomcursor`
//! - `clear`, `select_all`
//! - `exists`, `visible`, `text`, `wait_for`, `wait_for_visible`
//! - `scroll_to`, `url`, `title`
//!
//! Tasks should depend on `TaskContext` and the capability/state modules,
//! not on the lower-level utilities directly.

pub mod adaptive;
pub mod api;

/// Re-exported from the standalone `bacon-pipeline` crate.
///
/// To use the pipeline directly (without the auto-rust re-exports),
/// depend on `bacon-pipeline` and call `bacon_pipeline::config::init()`.
pub mod bacon_core {
    pub use bacon_pipeline::core::*;
    // Re-export modules explicitly for binary access
    pub use bacon_pipeline::core::cli_types;
    pub use bacon_pipeline::core::spec_io;
}
pub mod bacon_agent_nvidia {
    // Re-export agent submodules explicitly (glob doesn't bring in submodules)
    pub use bacon_pipeline::agent::auditor;
    pub use bacon_pipeline::agent::cli;
    pub use bacon_pipeline::agent::coder;
    pub use bacon_pipeline::agent::observer;
    pub use bacon_pipeline::agent::pipeline;
    pub use bacon_pipeline::agent::spec_io;
    pub use bacon_pipeline::agent::strategist;
    pub use bacon_pipeline::agent::types;
}
pub mod browser;
pub mod capabilities;
pub mod cli;
pub mod config;
pub mod error;
pub mod health_logger;
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

pub mod task;

/// Re-export Key types from bacon-pipeline.
pub use bacon_pipeline::ProjectConfig;
pub use llm::{ChatMessage, Llm, LlmClient, LlmProvider};
pub use runtime::task_context::TaskContext;
pub use state::ClipboardState;

/// Convenience imports for task authors.
pub mod prelude {
    pub use crate::capabilities::{clipboard, keyboard, mouse, navigation, scroll, timing};
    pub use crate::{ClipboardState, TaskContext};
}
