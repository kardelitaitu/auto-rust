//! Compatibility module for `dsl_executor`.
//!
//! This module re-exports items from `dsl::dsl_executor` for backward compatibility
//! with code that expects `dsl_executor` as a direct submodule of `task`.

pub use super::dsl::dsl_executor::*;
