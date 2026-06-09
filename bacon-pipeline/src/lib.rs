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

pub mod agent;
pub mod config;
pub mod core;
pub mod llm;

// Re-export key types for convenience
pub use config::{init, ProjectConfig};
pub use core::{GitSnapshot, PipelineAgent, PipelineConfig, PipelineCtx, Stage, WorkerOutput};
pub use llm::Llm;
