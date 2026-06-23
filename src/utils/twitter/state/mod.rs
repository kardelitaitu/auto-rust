//! Modular Twitter activity state submodules.
//! Extracted from `twitteractivity_state.rs` per spec 0017.

mod config;
mod session;
mod tracking;
mod types;

pub use config::{read_u32, read_u64, TaskConfig};
pub use session::{RateLimitBackoff, SessionState};
pub use tracking::TweetActionTracker;
pub use types::{
    compute_trending_bias, detect_conversation_indicators, parse_button_coordinates,
    parse_coordinates_with_default, parse_following_result, parse_reply_verification,
    CandidateContext, CandidateResult, SentimentTemplates, TaskValidationError,
};
