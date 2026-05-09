//! Unified decision engine module.
//!
//! This module consolidates all decision engine functionality previously
//! spread across 5 separate files into a unified architecture.
//!
//! # Architecture
//!
//! - `types`: Shared types (TweetContext, EngagementDecision, etc.)
//! - `engine`: UnifiedEngine implementation
//! - `strategies`: Individual strategy implementations
//!
//! # Usage
//!
//! ```rust
//! use auto::utils::twitter::decision::{DecisionEngineFactory, DecisionStrategy};
//!
//! let api_key = "test_key".to_string();
//! let engine = DecisionEngineFactory::create(
//!     DecisionStrategy::Auto,
//!     Some(api_key),
//! );
//! ```

pub mod engine;
pub mod types;

pub(crate) mod strategies;

// Public exports
pub use engine::DecisionEngineFactory;
pub use engine::UnifiedEngine;
pub use types::DecisionEngine;
pub use types::DecisionStrategy;
pub use types::EngagementDecision;
pub use types::EngagementLevel;
pub use types::TweetContext;
