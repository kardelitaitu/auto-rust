//! Compatibility module for `dsl_executor`.
//!
//! This module re-exports items from `dsl::executor` for backward compatibility
//! with code that expects `dsl_executor` as a separate module.

pub use super::executor::DslExecutionStats;
pub use super::executor::DslExecutor;
pub use super::profiling::ExecutionReport;
