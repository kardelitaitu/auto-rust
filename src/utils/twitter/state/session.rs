//! Per-session engagement state and rate-limit backoff pacing.

use crate::utils::twitter::{
    twitteractivity_limits::{EngagementCounters, EngagementLimits},
    twitteractivity_types::TweetId,
};
use std::time::{Duration, Instant};

use super::tracking::TweetActionTracker;

/// Consolidated session state for Twitter activity task.
/// Groups engagement counters, limits, action tracking, and deadline into a single unit.
#[derive(Debug)]
pub struct SessionState {
    /// Engagement action counters (likes, retweets, follows, etc.)
    pub counters: EngagementCounters,
    /// Maximum allowed actions per session
    pub limits: EngagementLimits,
    /// Tracks per-tweet action timing to prevent rapid chains
    pub action_tracker: TweetActionTracker,
    /// Session deadline for timeout checking
    pub deadline: Instant,
}

impl SessionState {
    #[must_use]
    pub fn new(limits: EngagementLimits, duration_ms: u64, min_action_delay_ms: u64) -> Self {
        Self {
            counters: EngagementCounters::new(),
            limits,
            action_tracker: TweetActionTracker::new(min_action_delay_ms),
            deadline: Instant::now() + Duration::from_millis(duration_ms),
        }
    }

    #[must_use]
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.deadline
    }

    #[must_use]
    pub fn remaining_time(&self) -> Duration {
        let now = Instant::now();
        if now >= self.deadline {
            Duration::from_millis(0)
        } else {
            self.deadline.duration_since(now)
        }
    }

    #[must_use]
    pub fn is_action_allowed(&self, action: &str) -> bool {
        match action {
            "like" => self.limits.can_like(&self.counters),
            "retweet" => self.limits.can_retweet(&self.counters),
            "follow" => self.limits.can_follow(&self.counters),
            "reply" => self.limits.can_reply(&self.counters),
            "bookmark" => self.limits.can_bookmark(&self.counters),
            "quote" => self.limits.can_quote_tweet(&self.counters),
            "dive" => self.limits.can_dive(&self.counters),
            _ => false,
        }
    }

    #[must_use]
    pub fn action_summary(&self) -> (u32, u32) {
        (self.counters.total_actions(), self.limits.max_total_actions)
    }

    #[must_use]
    pub fn is_total_limit_reached(&self) -> bool {
        self.counters.total_actions() >= self.limits.max_total_actions
    }

    pub fn record_action(&mut self, tweet_id: &TweetId, action_type: &'static str) {
        self.counters.increment(action_type);
        self.action_tracker
            .record_action(tweet_id.clone(), action_type);
    }

    #[must_use]
    pub fn progress_summary(&self) -> String {
        format!(
            "Session: {}/{} actions | L:{}/{} R:{}/{} F:{}/{} Re:{}/{} | Time left: {:?}",
            self.counters.total_actions(),
            self.limits.max_total_actions,
            self.counters.likes,
            self.limits.max_likes,
            self.counters.retweets,
            self.limits.max_retweets,
            self.counters.follows,
            self.limits.max_follows,
            self.counters.replies,
            self.limits.max_replies,
            self.remaining_time()
        )
    }

    /// Build summary lines for final engagement logging.
    ///
    /// Returns `(summary_line, remaining_limits_line)` suitable for `info!()` output.
    /// Pure formatting — does not mutate session state.
    #[must_use]
    pub fn build_summary_lines(&self, duration_ms: u64) -> (String, String) {
        let last_remaining = self.remaining_time();
        let duration_secs = Duration::from_millis(duration_ms)
            .saturating_sub(last_remaining)
            .as_secs_f64();
        let summary_line = format!(
            "[twitter] Engagement summary | likes={} retweets={} follows={} replies={} thread_dives={} bookmarks={} quote_tweets={} total_actions={} duration={:.1}s",
            self.counters.likes,
            self.counters.retweets,
            self.counters.follows,
            self.counters.replies,
            self.counters.thread_dives,
            self.counters.bookmarks,
            self.counters.quote_tweets,
            self.counters.total_actions(),
            duration_secs
        );

        let c = &self.counters;
        let l = &self.limits;
        let remaining_limits_line = format!(
            "[twitter] Remaining limits | likes={} retweets={} follows={} replies={} thread_dives={} bookmarks={} quote_tweets={} total_actions={}",
            l.max_likes.saturating_sub(c.likes),
            l.max_retweets.saturating_sub(c.retweets),
            l.max_follows.saturating_sub(c.follows),
            l.max_replies.saturating_sub(c.replies),
            l.max_thread_dives.saturating_sub(c.thread_dives),
            l.max_bookmarks.saturating_sub(c.bookmarks),
            l.max_quote_tweets.saturating_sub(c.quote_tweets),
            l.max_total_actions.saturating_sub(c.total_actions()),
        );

        (summary_line, remaining_limits_line)
    }
}

/// Tracks rate-limit backoff state for session-level pacing.
///
/// When a rate-limit error is detected from Twitter/X, this records the event
/// and enforces a cooldown period before any further engagement actions
/// are attempted. The cooldown increases exponentially with consecutive
/// rate-limit hits and resets on successful actions.
#[derive(Debug, Clone)]
pub struct RateLimitBackoff {
    /// Number of consecutive rate-limit hits
    consecutive_hits: u32,
    /// System time until which backoff is active
    cooldown_until: Instant,
    /// Base delay in ms for the first backoff
    base_delay_ms: u64,
    /// Maximum delay cap to prevent unbounded waiting
    max_delay_ms: u64,
}

impl RateLimitBackoff {
    #[must_use]
    pub fn new(base_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            consecutive_hits: 0,
            cooldown_until: Instant::now(),
            base_delay_ms,
            max_delay_ms,
        }
    }

    pub fn record_rate_limit(&mut self) {
        self.consecutive_hits = self.consecutive_hits.saturating_add(1);
        let delay = self.calculate_delay();
        self.cooldown_until = Instant::now() + Duration::from_millis(delay);
    }

    pub fn record_success(&mut self) {
        self.consecutive_hits = 0;
        self.cooldown_until = Instant::now();
    }

    #[must_use]
    pub fn is_in_cooldown(&self) -> bool {
        Instant::now() < self.cooldown_until
    }

    #[must_use]
    pub fn remaining_cooldown_ms(&self) -> u64 {
        let now = Instant::now();
        if now < self.cooldown_until {
            self.cooldown_until.duration_since(now).as_millis() as u64
        } else {
            0
        }
    }

    pub fn reset(&mut self) {
        self.consecutive_hits = 0;
        self.cooldown_until = Instant::now();
    }

    /// Calculate the delay for the current number of consecutive hits
    /// using exponential backoff: base * 2^(hits-1), capped at `max_delay`.
    fn calculate_delay(&self) -> u64 {
        if self.consecutive_hits == 0 {
            return 0;
        }
        let exponent = (self.consecutive_hits - 1).min(63);
        let multiplier = 2u64.saturating_pow(exponent);
        let delay = self.base_delay_ms.saturating_mul(multiplier);
        delay.min(self.max_delay_ms)
    }
}

#[cfg(test)]
mod tdd_tests {
    use super::{RateLimitBackoff, SessionState};
    use crate::tests::twitter_helpers::*;
    use crate::utils::twitter::twitteractivity_limits::EngagementLimits;
    use crate::utils::twitter::TweetId;

    // ====================================================================
    // SessionState tests
    // ====================================================================

    #[test]
    fn tdd_red_session_expiry_reports_zero_remaining() {
        let limits = EngagementLimits::default();
        let session = SessionState::new(limits, 0, 100);
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(
            session.is_expired(),
            "Session with 0ms duration should be expired"
        );
        assert_eq!(
            session.remaining_time().as_millis(),
            0,
            "Remaining time should be 0 for expired session"
        );
    }

    #[test]
    fn tdd_green_session_progress_summary_format() {
        let mut session = test_session_state();
        session.record_action(&TweetId::from_unchecked("tweet_1"), "like");
        let summary = session.progress_summary();
        assert!(summary.contains("1/10"), "Summary should show 1/10 actions");
        assert!(summary.contains("L:1"), "Summary should show L:1");
        assert!(
            summary.contains("Time left:"),
            "Summary should show Time left"
        );
    }

    #[test]
    fn tdd_green_session_records_multiple_action_types() {
        let mut session = test_session_state_with_limits(5, 3, 2, 1, 3, 2, 2, 20, 60000);
        session.record_action(&TweetId::from_unchecked("t1"), "like");
        session.record_action(&TweetId::from_unchecked("t2"), "retweet");
        session.record_action(&TweetId::from_unchecked("t3"), "follow");
        session.record_action(&TweetId::from_unchecked("t4"), "reply");
        session.record_action(&TweetId::from_unchecked("t5"), "bookmark");
        session.record_action(&TweetId::from_unchecked("t6"), "quote");
        session.record_action(&TweetId::from_unchecked("t7"), "dive");
        assert_eq!(session.counters.likes, 1);
        assert_eq!(session.counters.retweets, 1);
        assert_eq!(session.counters.follows, 1);
        assert_eq!(session.counters.replies, 1);
        assert_eq!(session.counters.bookmarks, 1);
        assert_eq!(session.counters.quote_tweets, 1);
        assert_eq!(session.counters.thread_dives, 1);
        assert_eq!(session.counters.total_actions(), 7);
    }

    #[test]
    fn tdd_green_session_is_total_limit_reached_detection() {
        let mut session = test_session_state_with_limits(5, 3, 2, 1, 3, 2, 2, 3, 60000);
        assert!(!session.is_total_limit_reached());
        session.record_action(&TweetId::from_unchecked("t1"), "like");
        session.record_action(&TweetId::from_unchecked("t2"), "like");
        session.record_action(&TweetId::from_unchecked("t3"), "like");
        assert!(session.is_total_limit_reached());
    }

    #[test]
    fn tdd_edge_session_is_action_allowed_for_unknown_action() {
        let session = test_session_state();
        assert!(!session.is_action_allowed("unknown_action"));
    }

    #[test]
    fn tdd_edge_session_action_summary_empty() {
        let session = test_session_state();
        assert_eq!(session.action_summary(), (0, 10));
    }

    // ====================================================================
    // RateLimitBackoff tests
    // ====================================================================

    #[test]
    fn tdd_red_rate_limit_backoff_blocks_after_record() {
        let mut backoff = RateLimitBackoff::new(100, 5000);
        assert!(
            !backoff.is_in_cooldown(),
            "Fresh backoff should not be in cooldown"
        );
        backoff.record_rate_limit();
        assert!(
            backoff.is_in_cooldown(),
            "Backoff should block after rate-limit hit"
        );
        assert!(
            backoff.remaining_cooldown_ms() > 0,
            "Remaining cooldown should be positive"
        );
    }

    #[test]
    fn tdd_red_rate_limit_backoff_increases_with_consecutive_hits() {
        let mut backoff = RateLimitBackoff::new(100, 5000);
        backoff.record_rate_limit();
        let cooldown_1 = backoff.remaining_cooldown_ms();
        backoff.record_rate_limit();
        let cooldown_2 = backoff.remaining_cooldown_ms();
        backoff.record_rate_limit();
        let cooldown_3 = backoff.remaining_cooldown_ms();
        assert!(
            cooldown_1 >= 90,
            "First cooldown {} should be >= 90ms (base=100ms)",
            cooldown_1
        );
        assert!(
            cooldown_2 > cooldown_1,
            "Second cooldown {} should be longer than first {}",
            cooldown_2,
            cooldown_1
        );
        assert!(
            cooldown_3 > cooldown_2,
            "Third cooldown {} should be longer than second {}",
            cooldown_3,
            cooldown_2
        );
    }

    #[test]
    fn tdd_red_rate_limit_backoff_success_clears_state() {
        let mut backoff = RateLimitBackoff::new(100, 5000);
        backoff.record_rate_limit();
        assert!(backoff.is_in_cooldown(), "Should be in cooldown after hit");
        backoff.record_success();
        assert!(!backoff.is_in_cooldown(), "Success should clear cooldown");
        assert_eq!(
            backoff.remaining_cooldown_ms(),
            0,
            "Remaining cooldown should be 0 after success"
        );
    }

    #[test]
    fn tdd_green_rate_limit_backoff_base_delay_default() {
        let backoff = RateLimitBackoff::new(1000, 30000);
        assert!(!backoff.is_in_cooldown());
        assert_eq!(backoff.remaining_cooldown_ms(), 0);
    }

    #[test]
    fn tdd_green_rate_limit_backoff_capped_at_max() {
        let mut backoff = RateLimitBackoff::new(10, 500);
        for _ in 0..10 {
            backoff.record_rate_limit();
        }
        let cooldown = backoff.remaining_cooldown_ms();
        assert!(
            cooldown <= 500,
            "Cooldown {} should not exceed max_delay of 500",
            cooldown
        );
    }

    #[test]
    fn tdd_green_rate_limit_backoff_reset_clears_hit_counter() {
        let mut backoff = RateLimitBackoff::new(100, 5000);
        backoff.record_rate_limit();
        assert!(backoff.is_in_cooldown());
        backoff.reset();
        assert!(!backoff.is_in_cooldown());
        backoff.record_rate_limit();
        let cooldown_after_reset = backoff.remaining_cooldown_ms();
        assert!(
            cooldown_after_reset >= 90,
            "After reset, cooldown {} should be ≈ base delay (100ms)",
            cooldown_after_reset
        );
    }

    #[test]
    fn tdd_green_rate_limit_backoff_records_success_clears_consecutive_hits() {
        let mut backoff = RateLimitBackoff::new(100, 5000);
        backoff.record_rate_limit();
        backoff.record_success();
        backoff.record_rate_limit();
        let cooldown_after_success = backoff.remaining_cooldown_ms();
        assert!(
            (90..=150).contains(&cooldown_after_success),
            "After success+re-hit, cooldown {} should be ≈ base (100ms), not escalated (200ms)",
            cooldown_after_success
        );
    }
}

#[cfg(test)]
mod gap_tests {
    use super::{RateLimitBackoff, SessionState};
    use crate::utils::twitter::twitteractivity_limits::EngagementLimits;
    use crate::utils::twitter::TweetId;

    #[test]
    fn is_action_allowed_checks_all_seven_types() {
        let limits = EngagementLimits::with_limits(1, 1, 1, 1, 1, 1, 1, 100);
        let mut session = SessionState::new(limits, 60_000, 100);

        for action in &[
            "like", "retweet", "follow", "reply", "bookmark", "quote", "dive",
        ] {
            assert!(
                session.is_action_allowed(action),
                "{action} should be allowed"
            );
        }

        session.record_action(&TweetId::from_unchecked("t1"), "like");
        assert!(!session.is_action_allowed("like"));
        session.record_action(&TweetId::from_unchecked("t2"), "retweet");
        assert!(!session.is_action_allowed("retweet"));
        session.record_action(&TweetId::from_unchecked("t3"), "follow");
        assert!(!session.is_action_allowed("follow"));
        session.record_action(&TweetId::from_unchecked("t4"), "reply");
        assert!(!session.is_action_allowed("reply"));
        session.record_action(&TweetId::from_unchecked("t5"), "bookmark");
        assert!(!session.is_action_allowed("bookmark"));
        session.record_action(&TweetId::from_unchecked("t6"), "quote");
        assert!(!session.is_action_allowed("quote"));
        session.record_action(&TweetId::from_unchecked("t7"), "dive");
        assert!(!session.is_action_allowed("dive"));
    }

    #[test]
    fn rate_limit_backoff_calculate_delay_zero_hits_returns_zero() {
        let backoff = RateLimitBackoff::new(100, 5000);
        assert_eq!(backoff.calculate_delay(), 0);
    }

    #[test]
    fn rate_limit_backoff_calculate_delay_exponential_growth() {
        let mut backoff = RateLimitBackoff::new(100, 10000);
        backoff.consecutive_hits = 1;
        assert_eq!(backoff.calculate_delay(), 100);
        backoff.consecutive_hits = 2;
        assert_eq!(backoff.calculate_delay(), 200);
        backoff.consecutive_hits = 3;
        assert_eq!(backoff.calculate_delay(), 400);
        backoff.consecutive_hits = 4;
        assert_eq!(backoff.calculate_delay(), 800);
    }

    #[test]
    fn rate_limit_backoff_calculate_delay_capped_at_max() {
        let mut backoff = RateLimitBackoff::new(100, 500);
        backoff.consecutive_hits = 10;
        assert_eq!(backoff.calculate_delay(), 500);
    }

    #[test]
    fn session_action_summary_reflects_recorded_actions() {
        let mut session = SessionState::new(
            EngagementLimits::with_limits(5, 3, 2, 1, 3, 2, 2, 20),
            60_000,
            100,
        );
        assert_eq!(session.action_summary(), (0, 20));
        session.record_action(&TweetId::from_unchecked("t1"), "like");
        session.record_action(&TweetId::from_unchecked("t2"), "like");
        session.record_action(&TweetId::from_unchecked("t3"), "retweet");
        assert_eq!(session.action_summary(), (3, 20));
    }

    #[test]
    fn session_is_total_limit_reached() {
        let mut session = SessionState::new(
            EngagementLimits::with_limits(5, 5, 5, 5, 5, 5, 5, 3),
            60_000,
            100,
        );
        assert!(!session.is_total_limit_reached());
        session.record_action(&TweetId::from_unchecked("t1"), "like");
        session.record_action(&TweetId::from_unchecked("t2"), "retweet");
        assert!(!session.is_total_limit_reached());
        session.record_action(&TweetId::from_unchecked("t3"), "follow");
        assert!(session.is_total_limit_reached());
    }

    #[test]
    fn build_summary_lines_contains_expected_keys() {
        let limits = EngagementLimits::with_limits(3, 4, 5, 6, 7, 8, 9, 10);
        let session = SessionState::new(limits, 60_000, 100);

        let (summary_line, remaining_limits_line) = session.build_summary_lines(60_000);

        for key in [
            "likes=",
            "retweets=",
            "follows=",
            "replies=",
            "thread_dives=",
            "bookmarks=",
            "quote_tweets=",
            "total_actions=",
            "duration=",
        ] {
            assert!(summary_line.contains(key), "summary line missing {key}");
        }

        for key in [
            "likes=",
            "retweets=",
            "follows=",
            "replies=",
            "thread_dives=",
            "bookmarks=",
            "quote_tweets=",
            "total_actions=",
        ] {
            assert!(
                remaining_limits_line.contains(key),
                "remaining limits line missing {key}"
            );
        }
    }

    #[test]
    fn build_summary_lines_with_non_zero_counters() {
        let limits = EngagementLimits::with_limits(10, 8, 6, 4, 5, 3, 3, 50);
        let mut session = SessionState::new(limits, 60_000, 100);

        session.record_action(&TweetId::from_unchecked("t1"), "like");
        session.record_action(&TweetId::from_unchecked("t2"), "like");
        session.record_action(&TweetId::from_unchecked("t3"), "retweet");
        session.record_action(&TweetId::from_unchecked("t4"), "follow");

        let (summary, remaining) = session.build_summary_lines(60_000);

        assert!(
            summary.contains("likes=2"),
            "Expected likes=2, got: {summary}"
        );
        assert!(
            summary.contains("retweets=1"),
            "Expected retweets=1, got: {summary}"
        );
        assert!(
            summary.contains("follows=1"),
            "Expected follows=1, got: {summary}"
        );
        assert!(summary.contains("total_actions=4"));

        assert!(
            remaining.contains("likes=8"),
            "Expected remaining likes=8, got: {remaining}"
        );
        assert!(
            remaining.contains("retweets=7"),
            "Expected remaining retweets=7, got: {remaining}"
        );
    }

    #[test]
    fn build_summary_lines_zero_duration() {
        let limits = EngagementLimits::default();
        let session = SessionState::new(limits, 60_000, 100);

        let (summary, remaining) = session.build_summary_lines(0);
        assert!(summary.contains("likes=0"));
        assert!(summary.contains("total_actions=0"));
        assert!(remaining.contains("likes=5"));
    }

    #[test]
    fn build_summary_lines_saturating_sub_no_underflow() {
        let limits = EngagementLimits::with_limits(1, 1, 1, 1, 1, 1, 1, 10);
        let mut session = SessionState::new(limits, 60_000, 100);

        session.counters.likes = 5;
        session.counters.retweets = 3;

        let (_, remaining) = session.build_summary_lines(60_000);

        assert!(remaining.contains("likes=0"));
        assert!(remaining.contains("retweets=0"));
    }
}
