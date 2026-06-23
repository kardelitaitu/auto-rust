/*
last audited 09-06-26 by RSA-Agent
crate: utils::twitter | status: SAFE | lint: CLEAN
findings: 0 unsafe, all unwrap/expect in test code, clean clippy, 4 files >1kLoC | next: none | perf: OnceLock lazy static, no red flags
*/

//! Twitter activity task utilities.
//! Provides helper functions for browser automation on Twitter/X.
//!
//! All helpers operate on `TaskContext` and use JavaScript evaluation
//! for DOM queries and interactions.

pub mod decision;
pub mod engagement;
pub mod reply_engine;
pub mod sentiment;
pub mod state;
pub mod twitteractivity_actions;
pub mod twitteractivity_constants;
pub mod twitteractivity_dive;
pub mod twitteractivity_errors;
pub mod twitteractivity_feed;
pub mod twitteractivity_helpers;
pub mod twitteractivity_humanized;
pub mod twitteractivity_interact;
pub mod twitteractivity_limits;
pub mod twitteractivity_llm;
pub mod twitteractivity_llm_execute;
pub mod twitteractivity_llm_validation;
pub mod twitteractivity_navigation;
pub mod twitteractivity_persona;
pub mod twitteractivity_popup;
pub mod twitteractivity_retry;
pub mod twitteractivity_selectors;
pub mod twitteractivity_simulation;
pub mod twitteractivity_state;
pub mod twitteractivity_types;
pub use twitteractivity_types::{
    ComposerFlow, EngagementOutcome, FlowError, FollowOutcome, PostOutcome, ReplyFlowState,
    StatusUrl, TweetId,
};

#[allow(unused_imports, ambiguous_glob_reexports)]
pub use decision::*;
#[allow(unused_imports)]
pub use engagement::*;
pub use reply_engine::*;
#[allow(unused_imports)]
pub use sentiment::*;
#[allow(unused_imports)]
pub use twitteractivity_constants::*;
#[allow(unused_imports)]
pub use twitteractivity_dive::*;
#[allow(unused_imports)]
pub use twitteractivity_errors::*;
#[allow(unused_imports)]
pub use twitteractivity_feed::*;
#[allow(unused_imports)]
pub use twitteractivity_humanized::*;
#[allow(unused_imports)]
pub use twitteractivity_interact::*;
#[allow(unused_imports)]
pub use twitteractivity_limits::*;
#[allow(unused_imports)]
pub use twitteractivity_llm::*;
#[allow(unused_imports)]
pub use twitteractivity_navigation::*;
#[allow(unused_imports)]
pub use twitteractivity_persona::*;
#[allow(unused_imports)]
pub use twitteractivity_popup::*;
#[allow(unused_imports)]
pub use twitteractivity_retry::*;
#[allow(unused_imports)]
pub use twitteractivity_selectors::*;
#[allow(unused_imports)]
pub use twitteractivity_simulation::*;

#[allow(unused_imports)]
pub use twitteractivity_state::*;

#[cfg(test)]
mod tests {
    /// Smoke test to verify twitter utils module compiles.
    #[test]
    fn test_twitter_utils_module_compiles() {
        // All re-exports are just aliases - verify structure
        // Module tests placeholder
    }
}
