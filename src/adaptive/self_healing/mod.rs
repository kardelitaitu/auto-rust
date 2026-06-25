//! Self-healing subsystem for automatic recovery and resilience.

// last audited 26-06-26 by Buffy

pub mod health;
pub mod history;
pub mod state;
pub mod strategy;
pub mod system;

pub use health::*;
pub use history::*;
pub use state::*;
pub use strategy::*;
pub use system::*;
