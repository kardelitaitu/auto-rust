//! Modular Twitter activity state submodules.
//! Extracted from `twitteractivity_state.rs` per spec 0017.

mod config;
mod session;
mod tracking;
mod types;

pub use config::{read_u32, read_u64, TaskConfig};
pub use session::{RateLimitBackoff, SessionState};
pub use tracking::TweetActionTracker;
pub use types::{CandidateContext, CandidateResult, SentimentTemplates, TaskValidationError};
