//! Twitter activity task utilities.
//! Provides helper functions for browser automation on Twitter/X.
//!
//! All helpers operate on `TaskContext` and use JavaScript evaluation
//! for DOM queries and interactions.

pub mod decision;
pub mod sentiment;
pub mod twitteractivity_constants;
pub mod twitteractivity_dive;
pub mod twitteractivity_engagement;
pub mod twitteractivity_errors;
pub mod twitteractivity_feed;
pub mod twitteractivity_humanized;
pub mod twitteractivity_interact;
pub mod twitteractivity_limits;
pub mod twitteractivity_llm;
pub mod twitteractivity_navigation;
pub mod twitteractivity_persona;
pub mod twitteractivity_popup;
pub mod twitteractivity_retry;
pub mod twitteractivity_selectors;
pub mod twitteractivity_sentiment;
pub mod twitteractivity_sentiment_context;
pub mod twitteractivity_sentiment_domains;
pub mod twitteractivity_sentiment_emoji;
pub mod twitteractivity_sentiment_enhanced;
pub mod twitteractivity_sentiment_llm;
pub mod twitteractivity_state;

#[allow(unused_imports)]
pub use decision::*;
#[allow(unused_imports)]
pub use sentiment::*;
#[allow(unused_imports)]
pub use twitteractivity_constants::*;
#[allow(unused_imports)]
pub use twitteractivity_dive::*;
#[allow(unused_imports)]
pub use twitteractivity_engagement::*;
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
