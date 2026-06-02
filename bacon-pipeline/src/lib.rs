//! Portable gated-LLM automation pipeline.
//!
//! The bacon pipeline runs 4 stages: Observer → Strategist → Coder → Auditor.
//! Each stage calls an NVIDIA LLM API, with human-in-the-loop confirmation gates.
//!
//! # Usage
//!
//! ```ignore
//! use bacon_pipeline::config::{init, ProjectConfig};
//!
//! fn main() {
//!     init(ProjectConfig::with_defaults(
//!         std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
//!     ));
//!     // ... use the pipeline
//! }
//! ```

pub mod config;
pub mod core;
pub mod llm;
pub mod agent;

// Re-export key types for convenience
pub use config::{init, ProjectConfig};
pub use core::{
    PipelineConfig, PipelineCtx, Stage, WorkerOutput,
    PipelineAgent, GitSnapshot,
};
pub use llm::Llm;
