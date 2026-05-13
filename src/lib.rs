/*
last audited 08-05-25 by RSA-Agent
crate: auto-rust | status: SAFE | lint: CLEAN
findings: Zero unsafe blocks, concurrency patterns appropriate, 3 minor dependency concerns | next: clean test imports / verify notify+enigo platform compat | perf: Arc/RwLock for metrics is good; static Mutexes in native.rs are low-risk
*/

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
pub mod bacon_agent_codex;
pub mod bacon_agent_gemini;
pub mod bacon_agent_kilocode;
pub mod bacon_agent_nvidia;
pub mod bacon_agent_ollama;
pub mod bacon_agent_opencode;
pub mod bacon_agent_pi;
pub mod bacon_core;
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
pub mod plugin;
pub mod result;
pub mod runtime;
pub mod session;
pub mod state;
pub mod tests;
pub mod tracing;
pub mod utils;
pub mod validation;

pub mod task;

pub use llm::{ChatMessage, Llm, LlmClient, LlmProvider};
pub use runtime::task_context::TaskContext;
pub use state::ClipboardState;

/// Convenience imports for task authors.
pub mod prelude {
    pub use crate::capabilities::{clipboard, keyboard, mouse, navigation, scroll, timing};
    pub use crate::{ClipboardState, TaskContext};
}
