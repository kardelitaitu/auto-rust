//! Per-tweet action deduplication tracker — prevents rapid action chains on the same tweet.

use crate::utils::twitter::twitteractivity_types::TweetId;
use log::info;
use std::collections::HashMap;
use std::time::Instant;

/// Tracks the last action type and timestamp for each tweet to prevent unrealistic action chains.
#[derive(Debug, Clone)]
pub struct TweetActionTracker {
    /// Maps tweet ID to (`last_action_type`, timestamp)
    pub last_action: HashMap<TweetId, (&'static str, Instant)>,
    /// Minimum delay between actions on the same tweet in milliseconds
    pub min_delay_ms: u64,
}

impl TweetActionTracker {
    #[must_use]
    pub fn new(min_delay_ms: u64) -> Self {
        Self {
            last_action: HashMap::new(),
            min_delay_ms,
        }
    }

    /// Check if an action is allowed on this tweet (prevents rapid action chains).
    /// Cooldown is per-tweet, not per-action-type.
    #[must_use]
    pub fn can_perform_action(&self, tweet_id: &TweetId) -> bool {
        if let Some((_, last_time)) = self.last_action.get(tweet_id) {
            let elapsed = last_time.elapsed();
            if elapsed.as_millis() < u128::from(self.min_delay_ms) {
                return false;
            }
        }
        true
    }

    /// Record that an action was performed on a tweet.
    pub fn record_action(&mut self, tweet_id: TweetId, action_type: &'static str) {
        info!(
            "Recorded {} action on tweet {} (cooldown: {}ms)",
            action_type, tweet_id, self.min_delay_ms
        );
        self.last_action
            .insert(tweet_id, (action_type, Instant::now()));
    }
}

#[cfg(test)]
mod tdd_tests {
    use crate::tests::twitter_helpers::test_action_tracker;
    use super::TweetId;

    #[test]
    fn tdd_green_action_tracker_cooldown_expires() {
        let mut tracker = test_action_tracker(50);

        tracker.record_action(TweetId::from_unchecked("tweet_1"), "like");
        assert!(!tracker.can_perform_action(&TweetId::from_unchecked("tweet_1")));

        std::thread::sleep(std::time::Duration::from_millis(60));

        assert!(tracker.can_perform_action(&TweetId::from_unchecked("tweet_1")));
    }

    #[test]
    fn tdd_edge_action_tracker_unknown_tweet_allowed() {
        let tracker = test_action_tracker(1000);
        assert!(tracker.can_perform_action(&TweetId::from_unchecked("unknown_tweet")));
    }
}

#[cfg(test)]
mod gap_tests {
    use super::TweetActionTracker;
    use super::TweetId;

    #[test]
    fn tracker_record_action_updates_last_action_map() {
        let mut tracker = TweetActionTracker::new(5000);
        assert!(tracker.last_action.is_empty());

        tracker.record_action(TweetId::from_unchecked("tweet_1"), "like");
        assert_eq!(tracker.last_action.len(), 1);
        assert!(tracker.last_action.contains_key(&TweetId::from_unchecked("tweet_1")));

        let (action_type, _) = tracker.last_action.get(&TweetId::from_unchecked("tweet_1")).unwrap();
        assert_eq!(*action_type, "like");
    }

    #[test]
    fn tracker_record_action_overwrites_previous() {
        let mut tracker = TweetActionTracker::new(5000);

        tracker.record_action(TweetId::from_unchecked("tweet_1"), "like");
        tracker.record_action(TweetId::from_unchecked("tweet_1"), "retweet");

        assert_eq!(tracker.last_action.len(), 1);
        let (action_type, _) = tracker.last_action.get(&TweetId::from_unchecked("tweet_1")).unwrap();
        assert_eq!(*action_type, "retweet");
    }

    #[test]
    fn tracker_blocks_same_tweet_within_cooldown() {
        let mut tracker = TweetActionTracker::new(60_000);

        tracker.record_action(TweetId::from_unchecked("tweet_1"), "like");
        assert!(!tracker.can_perform_action(&TweetId::from_unchecked("tweet_1")));
        assert!(tracker.can_perform_action(&TweetId::from_unchecked("tweet_2")));
    }
}
