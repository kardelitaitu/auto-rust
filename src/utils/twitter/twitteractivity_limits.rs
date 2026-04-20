//! Engagement limits and counters for Twitter automation.
//!
//! Prevents rate limits and bans by tracking actions taken during a session
//! and enforcing configurable limits per action type.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tracks the number of actions taken during a Twitter session.
/// Used to enforce rate limits and prevent account restrictions.
#[derive(Debug, Clone, Default)]
pub struct EngagementCounters {
    /// Number of likes performed in current session
    pub likes: u32,
    /// Number of retweets performed in current session
    pub retweets: u32,
    /// Number of follows performed in current session
    pub follows: u32,
    /// Number of replies performed in current session
    pub replies: u32,
    /// Number of thread dives performed in current session
    pub thread_dives: u32,
    /// Number of bookmarks performed in current session (V2)
    pub bookmarks: u32,
    /// Number of quote tweets performed in current session (V2)
    pub quote_tweets: u32,
}

impl EngagementCounters {
    /// Creates a new counters instance with all counts at zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns total number of engagement actions taken.
    pub fn total_actions(&self) -> u32 {
        self.likes
            + self.retweets
            + self.follows
            + self.replies
            + self.thread_dives
            + self.bookmarks
            + self.quote_tweets
    }

    /// Increments the like counter.
    pub fn increment_like(&mut self) {
        self.likes += 1;
    }

    /// Increments the retweet counter.
    pub fn increment_retweet(&mut self) {
        self.retweets += 1;
    }

    /// Increments the follow counter.
    pub fn increment_follow(&mut self) {
        self.follows += 1;
    }

    /// Increments the reply counter.
    pub fn increment_reply(&mut self) {
        self.replies += 1;
    }

    /// Increments the thread dive counter.
    pub fn increment_thread_dive(&mut self) {
        self.thread_dives += 1;
    }

    /// Increments the bookmark counter.
    pub fn increment_bookmark(&mut self) {
        self.bookmarks += 1;
    }

    /// Increments the quote tweet counter.
    pub fn increment_quote_tweet(&mut self) {
        self.quote_tweets += 1;
    }

    /// Returns a summary of all counters as a HashMap.
    pub fn to_summary(&self) -> HashMap<String, u32> {
        let mut summary = HashMap::new();
        summary.insert("likes".to_string(), self.likes);
        summary.insert("retweets".to_string(), self.retweets);
        summary.insert("follows".to_string(), self.follows);
        summary.insert("replies".to_string(), self.replies);
        summary.insert("thread_dives".to_string(), self.thread_dives);
        summary.insert("bookmarks".to_string(), self.bookmarks);
        summary
    }
}

/// Configuration for engagement limits per session.
/// Defines maximum allowed actions for each engagement type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngagementLimits {
    /// Maximum likes per session (default: 5)
    #[serde(default = "default_max_likes")]
    pub max_likes: u32,

    /// Maximum retweets per session (default: 3)
    #[serde(default = "default_max_retweets")]
    pub max_retweets: u32,

    /// Maximum follows per session (default: 2)
    #[serde(default = "default_max_follows")]
    pub max_follows: u32,

    /// Maximum replies per session (default: 1)
    #[serde(default = "default_max_replies")]
    pub max_replies: u32,

    /// Maximum thread dives per session (default: 3)
    #[serde(default = "default_max_thread_dives")]
    pub max_thread_dives: u32,

    /// Maximum bookmarks per session (default: 0, disabled in V1)
    #[serde(default = "default_max_bookmarks")]
    pub max_bookmarks: u32,

    /// Maximum quote tweets per session (default: 2, V2 feature)
    #[serde(default = "default_max_quote_tweets")]
    pub max_quote_tweets: u32,

    /// Maximum total engagement actions per session (default: 10)
    #[serde(default = "default_max_total_actions")]
    pub max_total_actions: u32,
}

fn default_max_likes() -> u32 {
    5
}

fn default_max_retweets() -> u32 {
    3
}

fn default_max_follows() -> u32 {
    2
}

fn default_max_replies() -> u32 {
    1
}

fn default_max_thread_dives() -> u32 {
    3
}

fn default_max_bookmarks() -> u32 {
    0
}

fn default_max_quote_tweets() -> u32 {
    2
}

fn default_max_total_actions() -> u32 {
    10
}

impl Default for EngagementLimits {
    fn default() -> Self {
        Self {
            max_likes: default_max_likes(),
            max_retweets: default_max_retweets(),
            max_follows: default_max_follows(),
            max_replies: default_max_replies(),
            max_thread_dives: default_max_thread_dives(),
            max_bookmarks: default_max_bookmarks(),
            max_quote_tweets: default_max_quote_tweets(),
            max_total_actions: default_max_total_actions(),
        }
    }
}

impl EngagementLimits {
    /// Creates a new limits instance with conservative defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new limits instance with custom values.
    pub fn with_limits(
        max_likes: u32,
        max_retweets: u32,
        max_follows: u32,
        max_replies: u32,
        max_thread_dives: u32,
        max_bookmarks: u32,
        max_quote_tweets: u32,
        max_total: u32,
    ) -> Self {
        Self {
            max_likes,
            max_retweets,
            max_follows,
            max_replies,
            max_thread_dives,
            max_bookmarks,
            max_quote_tweets,
            max_total_actions: max_total,
        }
    }

    /// Checks if a like action is allowed given current counters.
    pub fn can_like(&self, counters: &EngagementCounters) -> bool {
        counters.likes < self.max_likes && counters.total_actions() < self.max_total_actions
    }

    /// Checks if a retweet action is allowed given current counters.
    pub fn can_retweet(&self, counters: &EngagementCounters) -> bool {
        counters.retweets < self.max_retweets
            && counters.total_actions() < self.max_total_actions
    }

    /// Checks if a follow action is allowed given current counters.
    pub fn can_follow(&self, counters: &EngagementCounters) -> bool {
        counters.follows < self.max_follows && counters.total_actions() < self.max_total_actions
    }

    /// Checks if a reply action is allowed given current counters.
    pub fn can_reply(&self, counters: &EngagementCounters) -> bool {
        counters.replies < self.max_replies && counters.total_actions() < self.max_total_actions
    }

    /// Checks if a thread dive is allowed given current counters.
    pub fn can_dive(&self, counters: &EngagementCounters) -> bool {
        counters.thread_dives < self.max_thread_dives
            && counters.total_actions() < self.max_total_actions
    }

    /// Checks if a bookmark action is allowed given current counters.
    pub fn can_bookmark(&self, counters: &EngagementCounters) -> bool {
        counters.bookmarks < self.max_bookmarks
            && counters.total_actions() < self.max_total_actions
    }

    /// Checks if a quote tweet action is allowed given current counters.
    pub fn can_quote_tweet(&self, counters: &EngagementCounters) -> bool {
        counters.quote_tweets < self.max_quote_tweets
            && counters.total_actions() < self.max_total_actions
    }

    /// Returns which actions are still available given current counters.
    pub fn available_actions(&self, counters: &EngagementCounters) -> Vec<&'static str> {
        let mut actions = Vec::new();

        if self.can_like(counters) {
            actions.push("like");
        }
        if self.can_retweet(counters) {
            actions.push("retweet");
        }
        if self.can_follow(counters) {
            actions.push("follow");
        }
        if self.can_reply(counters) {
            actions.push("reply");
        }
        if self.can_dive(counters) {
            actions.push("thread_dive");
        }
        if self.can_bookmark(counters) {
            actions.push("bookmark");
        }
        if self.can_quote_tweet(counters) {
            actions.push("quote_tweet");
        }

        actions
    }

    /// Returns a summary of remaining actions as a HashMap.
    pub fn remaining(&self, counters: &EngagementCounters) -> HashMap<String, u32> {
        let mut remaining = HashMap::new();
        remaining.insert("likes".to_string(), self.max_likes.saturating_sub(counters.likes));
        remaining.insert(
            "retweets".to_string(),
            self.max_retweets.saturating_sub(counters.retweets),
        );
        remaining.insert(
            "follows".to_string(),
            self.max_follows.saturating_sub(counters.follows),
        );
        remaining.insert(
            "replies".to_string(),
            self.max_replies.saturating_sub(counters.replies),
        );
        remaining.insert(
            "thread_dives".to_string(),
            self.max_thread_dives.saturating_sub(counters.thread_dives),
        );
        remaining.insert(
            "bookmarks".to_string(),
            self.max_bookmarks.saturating_sub(counters.bookmarks),
        );
        remaining.insert(
            "quote_tweets".to_string(),
            self.max_quote_tweets.saturating_sub(counters.quote_tweets),
        );
        remaining.insert(
            "total_actions".to_string(),
            self.max_total_actions
                .saturating_sub(counters.total_actions()),
        );
        remaining
    }
}

/// Result of checking whether an engagement action is allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngagementCheck {
    /// Action is allowed
    Allowed,
    /// Action blocked due to per-action limit
    LimitReached { action: &'static str },
    /// Action blocked due to total session limit
    SessionLimitReached,
}

impl EngagementCheck {
    /// Returns true if the action is allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, EngagementCheck::Allowed)
    }

    /// Returns the reason if blocked.
    pub fn reason(&self) -> Option<String> {
        match self {
            EngagementCheck::Allowed => None,
            EngagementCheck::LimitReached { action } => {
                Some(format!("{} limit reached", action))
            }
            EngagementCheck::SessionLimitReached => {
                Some("Session engagement limit reached".to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counters_increment() {
        let mut counters = EngagementCounters::new();
        assert_eq!(counters.likes, 0);

        counters.increment_like();
        assert_eq!(counters.likes, 1);

        counters.increment_retweet();
        counters.increment_follow();
        assert_eq!(counters.total_actions(), 3);
    }

    #[test]
    fn test_limits_default_values() {
        let limits = EngagementLimits::new();
        assert_eq!(limits.max_likes, 5);
        assert_eq!(limits.max_retweets, 3);
        assert_eq!(limits.max_follows, 2);
        assert_eq!(limits.max_replies, 1);
        assert_eq!(limits.max_thread_dives, 3);
        assert_eq!(limits.max_bookmarks, 0);
        assert_eq!(limits.max_total_actions, 10);
    }

    #[test]
    fn test_can_engage_checks_limits() {
        let limits = EngagementLimits::new();
        let mut counters = EngagementCounters::new();

        // Should be allowed initially
        assert!(limits.can_like(&counters));
        assert!(limits.can_retweet(&counters));
        assert!(limits.can_follow(&counters));

        // Increment to limit
        for _ in 0..5 {
            counters.increment_like();
        }

        // Should be blocked now
        assert!(!limits.can_like(&counters));
        assert!(limits.can_retweet(&counters)); // Other actions still OK
    }

    #[test]
    fn test_total_actions_limit() {
        let limits = EngagementLimits::with_limits(5, 3, 2, 1, 3, 0, 2, 5);
        let mut counters = EngagementCounters::new();

        // Fill up total actions
        for _ in 0..5 {
            counters.increment_like();
        }

        // All actions should be blocked due to total limit
        assert!(!limits.can_like(&counters));
        assert!(!limits.can_retweet(&counters));
        assert!(!limits.can_follow(&counters));
    }

    #[test]
    fn test_available_actions() {
        let limits = EngagementLimits::new();
        let mut counters = EngagementCounters::new();

        // All actions available initially
        let available = limits.available_actions(&counters);
        assert!(available.contains(&"like"));
        assert!(available.contains(&"retweet"));
        assert!(available.contains(&"follow"));

        // Block likes
        for _ in 0..5 {
            counters.increment_like();
        }

        let available = limits.available_actions(&counters);
        assert!(!available.contains(&"like"));
        assert!(available.contains(&"retweet"));
    }

    #[test]
    fn test_remaining_calculation() {
        let limits = EngagementLimits::new();
        let mut counters = EngagementCounters::new();

        counters.increment_like();
        counters.increment_like();

        let remaining = limits.remaining(&counters);
        assert_eq!(remaining.get("likes").copied().unwrap_or(0), 3); // 5 - 2 = 3
        assert_eq!(remaining.get("retweets").copied().unwrap_or(0), 3);
    }

    #[test]
    fn test_engagement_check() {
        let check = EngagementCheck::Allowed;
        assert!(check.is_allowed());
        assert_eq!(check.reason(), None);

        let check = EngagementCheck::LimitReached { action: "like" };
        assert!(!check.is_allowed());
        assert_eq!(check.reason(), Some("like limit reached".to_string()));
    }

    #[test]
    fn test_engagement_limits_with_custom_values() {
        let limits = EngagementLimits::with_limits(10, 5, 3, 2, 5, 1, 2, 20);
        let counters = EngagementCounters::new();

        assert_eq!(limits.max_likes, 10);
        assert_eq!(limits.max_retweets, 5);
        assert_eq!(limits.max_follows, 3);
        assert_eq!(limits.max_quote_tweets, 2);
        assert!(limits.can_like(&counters));
        assert!(limits.can_retweet(&counters));
        assert!(limits.can_quote_tweet(&counters));
    }

    #[test]
    fn test_counterv2_to_summary() {
        let mut counters = EngagementCounters::new();
        counters.increment_like();
        counters.increment_retweet();
        counters.increment_follow();
        
        let summary = counters.to_summary();
        assert_eq!(summary.get("likes").copied().unwrap_or(0), 1);
        assert_eq!(summary.get("retweets").copied().unwrap_or(0), 1);
        assert_eq!(summary.get("follows").copied().unwrap_or(0), 1);
    }
}
