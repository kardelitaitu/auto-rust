//! State structs and context types for Twitter activity task.
//! Re-exports from `state/` submodules per spec 0017.

pub use super::state::{
    read_u32, read_u64, CandidateContext, CandidateResult, RateLimitBackoff, SentimentTemplates,
    SessionState, TaskConfig, TaskValidationError, TweetActionTracker,
};

#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test_support {
    use serde_json::{json, Value};

    pub fn twitter_config() -> crate::config::TwitterActivityConfig {
        crate::config::TwitterActivityConfig::default()
    }

    pub fn duration_payload(value: i64) -> Value {
        json!({"duration_ms": value})
    }

    pub fn candidate_count_payload(value: i64) -> Value {
        json!({"candidate_count": value})
    }

    pub fn empty_payload() -> Value {
        json!({})
    }

    pub fn full_payload() -> Value {
        json!({
            "duration_ms": 120000,
            "candidate_count": 10,
            "thread_depth": 15,
            "max_actions_per_scan": 5
        })
    }
}
