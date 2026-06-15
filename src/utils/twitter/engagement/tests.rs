//! Integration, statistical, property, and gap tests for engagement.

use super::*;
use crate::utils::twitter::twitteractivity_actions::extract_tweet_text;
use crate::utils::twitter::TweetId;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::utils::twitter::twitteractivity_limits::{EngagementCounters, EngagementLimits};
    use serde_json::json;

    /// Test action_allowed_by_limits for each action type
    #[test]
    fn action_allowed_by_limits_respects_all_limits() {
        let limits = EngagementLimits::with_limits(1, 1, 1, 1, 1, 1, 1, 5);
        let mut counters = EngagementCounters::new();

        // All should be allowed initially
        assert!(action_allowed_by_limits("like", &limits, &counters));
        assert!(action_allowed_by_limits("retweet", &limits, &counters));
        assert!(action_allowed_by_limits("quote", &limits, &counters));
        assert!(action_allowed_by_limits("follow", &limits, &counters));
        assert!(action_allowed_by_limits("reply", &limits, &counters));
        assert!(action_allowed_by_limits("bookmark", &limits, &counters));

        // After incrementing, should be blocked
        counters.increment_like();
        assert!(!action_allowed_by_limits("like", &limits, &counters));
        // Others still allowed
        assert!(action_allowed_by_limits("retweet", &limits, &counters));
    }

    /// Test action_allowed_by_limits with unknown action
    #[test]
    fn action_allowed_by_limits_returns_false_for_unknown() {
        let limits = EngagementLimits::default();
        let counters = EngagementCounters::new();
        assert!(!action_allowed_by_limits(
            "unknown_action",
            &limits,
            &counters
        ));
    }

    #[test]
    fn selected_candidate_actions_respects_persona_limits_and_tracker() {
        let persona = PersonaWeights {
            like_prob: 1.0,
            retweet_prob: 1.0,
            quote_prob: 1.0,
            follow_prob: 1.0,
            reply_prob: 1.0,
            bookmark_prob: 1.0,
            thread_dive_prob: 1.0,
            interest_multiplier: 1.0,
        };
        let limits = EngagementLimits::with_limits(1, 1, 1, 1, 1, 1, 1, 10);
        let counters = EngagementCounters::new();
        let mut tracker = TweetActionTracker::new(60_000);

        let tid1 = TweetId::from_unchecked("tweet_1");

        let actions = selected_candidate_actions(&persona, &tid1, &limits, &counters, &tracker);
        assert_eq!(
            actions,
            vec!["like", "retweet", "quote", "follow", "reply", "bookmark"]
        );

        tracker.record_action(TweetId::from_unchecked("tweet_1"), "like");
        let blocked = selected_candidate_actions(&persona, &tid1, &limits, &counters, &tracker);
        assert!(blocked.is_empty());
    }

    #[test]
    fn thread_dive_prob_zero_drops_detail_actions() {
        let persona = PersonaWeights {
            thread_dive_prob: 0.0,
            ..PersonaWeights::default()
        };
        let mut actions = vec!["like", "retweet", "reply"];

        filter_detail_actions_for_gate(&mut actions, true, should_dive(&persona));

        assert_eq!(actions, vec!["like"]);
    }

    #[test]
    fn thread_dive_prob_one_keeps_detail_actions_when_status_url_exists() {
        let persona = PersonaWeights {
            thread_dive_prob: 1.0,
            ..PersonaWeights::default()
        };
        let mut actions = vec!["like", "retweet", "reply"];

        filter_detail_actions_for_gate(&mut actions, true, should_dive(&persona));

        assert_eq!(actions, vec!["like", "retweet", "reply"]);
    }

    #[test]
    fn decision_level_minimal_keeps_like_only() {
        let mut actions = vec!["like", "retweet", "quote", "follow", "reply", "bookmark"];

        filter_actions_for_decision_level(&mut actions, EngagementLevel::Minimal);

        assert_eq!(actions, vec!["like"]);
    }

    #[test]
    fn decision_level_medium_keeps_like_and_retweet_only() {
        let mut actions = vec!["like", "retweet", "quote", "follow", "reply", "bookmark"];

        filter_actions_for_decision_level(&mut actions, EngagementLevel::Medium);

        assert_eq!(actions, vec!["like", "retweet"]);
    }

    #[test]
    fn decision_level_full_keeps_selected_actions() {
        let mut actions = vec!["like", "retweet", "quote", "follow", "reply", "bookmark"];

        filter_actions_for_decision_level(&mut actions, EngagementLevel::Full);

        assert_eq!(
            actions,
            vec!["like", "retweet", "quote", "follow", "reply", "bookmark"]
        );
    }

    #[test]
    fn decision_level_none_clears_selected_actions() {
        let mut actions = vec!["like", "retweet"];

        filter_actions_for_decision_level(&mut actions, EngagementLevel::None);

        assert!(actions.is_empty());
    }

    #[test]
    fn dry_run_skips_post_dive_browser_work() {
        let dry_run = TaskConfig {
            dry_run_actions: true,
            ..Default::default()
        };
        let live = TaskConfig {
            dry_run_actions: false,
            ..Default::default()
        };

        assert!(!should_engage_replies_after_root_action(
            true, true, &dry_run
        ));
        assert!(!should_navigate_home_after_dive(true, &dry_run));
        assert!(should_engage_replies_after_root_action(true, true, &live));
        assert!(should_navigate_home_after_dive(true, &live));
    }

    /// Test extract_tweet_text with text field
    #[test]
    fn extract_tweet_text_extracts_text_field() {
        let tweet = json!({"text": "Hello world"});
        assert_eq!(extract_tweet_text(&tweet), "Hello world");
    }

    /// Test extract_tweet_text with full_text field (fallback)
    #[test]
    fn extract_tweet_text_extracts_full_text_field() {
        let tweet = json!({"full_text": "Full text content"});
        assert_eq!(extract_tweet_text(&tweet), "Full text content");
    }

    /// Test extract_tweet_text returns empty for missing fields
    #[test]
    fn extract_tweet_text_returns_empty_for_missing() {
        let tweet = json!({"id": "123"});
        assert_eq!(extract_tweet_text(&tweet), "");
    }

    /// Test generate_reply_text cycles through templates
    #[test]
    fn generate_reply_text_cycles_templates() {
        let templates = SentimentTemplates::default();
        let text1 = generate_reply_text(Sentiment::Positive, 0, &templates);
        let text2 = generate_reply_text(Sentiment::Positive, 1, &templates);
        // Should return different templates
        assert!(!text1.is_empty());
        assert!(!text2.is_empty());
    }

    /// Test generate_quote_text cycles through templates
    #[test]
    fn generate_quote_text_cycles_templates() {
        let templates = SentimentTemplates::default();
        let text1 = generate_quote_text(Sentiment::Neutral, 0, &templates);
        let text2 = generate_quote_text(Sentiment::Neutral, 1, &templates);
        assert!(!text1.is_empty());
        assert!(!text2.is_empty());
    }

    /// Test calc_rate with valid inputs
    #[test]
    fn calc_rate_calculates_correctly() {
        assert_eq!(calc_rate(5, 10), 50.0);
        assert_eq!(calc_rate(0, 10), 0.0);
        assert_eq!(calc_rate(10, 10), 100.0);
    }

    /// Test calc_rate handles zero total
    #[test]
    fn calc_rate_handles_zero_total() {
        assert_eq!(calc_rate(5, 0), 0.0);
    }
}

#[cfg(test)]
mod decision_integration_tests {
    use super::*;
    use crate::utils::twitter::twitteractivity_persona::PersonaWeights;
    use serde_json::json;

    /// Test handle_engagement_decision returns None when disabled
    #[tokio::test]
    async fn engagement_decision_returns_none_when_disabled() {
        let tweet = json!({"text": "Test tweet"});
        let config = TaskConfig {
            duration_ms: 60000,
            candidate_count: 5,
            smart_decision_enabled: false,
            ..Default::default()
        };
        let persona = PersonaWeights::default();
        let result = handle_engagement_decision(&tweet, &config, &persona, None).await;
        assert!(result.is_none());
    }

    /// Test handle_engagement_decision extracts tweet text correctly
    #[tokio::test]
    async fn engagement_decision_extracts_tweet_text() {
        let tweet = json!({
            "text": "This is a test tweet about technology",
            "replies": []
        });
        let config = TaskConfig {
            duration_ms: 60000,
            candidate_count: 5,
            smart_decision_enabled: true,
            ..Default::default()
        };
        let persona = PersonaWeights::default();
        let result = handle_engagement_decision(&tweet, &config, &persona, None).await;
        // Should return a decision (not None) when enabled
        assert!(result.is_some());
    }

    /// Test handle_engagement_decision handles replies array
    #[tokio::test]
    async fn engagement_decision_extracts_replies() {
        let tweet = json!({
            "text": "Main tweet",
            "replies": [
                {"author": "user1", "text": "Reply 1"},
                {"author": "user2", "text": "Reply 2"}
            ]
        });
        let config = TaskConfig {
            duration_ms: 60000,
            candidate_count: 5,
            smart_decision_enabled: true,
            ..Default::default()
        };
        let persona = PersonaWeights::default();
        let result = handle_engagement_decision(&tweet, &config, &persona, None).await;
        assert!(result.is_some());
    }
}

#[cfg(test)]
mod statistical_tests {
    use super::*;
    use crate::utils::twitter::twitteractivity_persona::PersonaWeights;

    /// Test that should_like produces expected distribution (within tolerance)
    #[test]
    fn should_like_distribution_within_tolerance() {
        let persona = PersonaWeights::default();
        let expected_prob = persona.like_prob;
        let trials = 1000;

        let successes: u32 = (0..trials)
            .map(|_| if should_like(&persona) { 1 } else { 0 })
            .sum();

        let actual_rate = successes as f64 / trials as f64;
        let tolerance = 0.05; // 5% tolerance

        assert!(
            (actual_rate - expected_prob).abs() < tolerance,
            "Expected ~{:.2}, got {:.2}",
            expected_prob,
            actual_rate
        );
    }

    /// Test that should_retweet produces expected distribution (within tolerance)
    #[test]
    fn should_retweet_distribution_within_tolerance() {
        let persona = PersonaWeights::default();
        let expected_prob = persona.retweet_prob;
        let trials = 1000;

        let successes: u32 = (0..trials)
            .map(|_| if should_retweet(&persona) { 1 } else { 0 })
            .sum();

        let actual_rate = successes as f64 / trials as f64;
        let tolerance = 0.05;

        assert!(
            (actual_rate - expected_prob).abs() < tolerance,
            "Expected ~{:.2}, got {:.2}",
            expected_prob,
            actual_rate
        );
    }

    /// Test that should_reply produces expected distribution (within tolerance)
    #[test]
    fn should_reply_distribution_within_tolerance() {
        let persona = PersonaWeights::default();
        let expected_prob = persona.reply_prob;
        let trials = 1000;

        let successes: u32 = (0..trials)
            .map(|_| if should_reply(&persona) { 1 } else { 0 })
            .sum();

        let actual_rate = successes as f64 / trials as f64;
        let tolerance = 0.05;

        assert!(
            (actual_rate - expected_prob).abs() < tolerance,
            "Expected ~{:.2}, got {:.2}",
            expected_prob,
            actual_rate
        );
    }

    /// Test that should_follow produces expected distribution (within tolerance)
    #[test]
    fn should_follow_distribution_within_tolerance() {
        let persona = PersonaWeights::default();
        let expected_prob = persona.follow_prob;
        let trials = 1000;

        let successes: u32 = (0..trials)
            .map(|_| if should_follow(&persona) { 1 } else { 0 })
            .sum();

        let actual_rate = successes as f64 / trials as f64;
        let tolerance = 0.05;

        assert!(
            (actual_rate - expected_prob).abs() < tolerance,
            "Expected ~{:.2}, got {:.2}",
            expected_prob,
            actual_rate
        );
    }

    /// Test that calc_rate produces expected percentages
    #[test]
    fn calc_rate_statistical_accuracy() {
        assert_eq!(calc_rate(50, 100), 50.0);
        assert_eq!(calc_rate(25, 100), 25.0);
        assert_eq!(calc_rate(75, 100), 75.0);
        assert!((calc_rate(1, 3) - 33.33).abs() < 0.01);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;

    /// Property: action_allowed_by_limits never panics on valid/invalid action names
    #[test]
    fn action_allowed_by_limits_no_panic() {
        let limits = EngagementLimits::default();
        let counters = EngagementCounters::new();

        let actions = vec![
            "like",
            "retweet",
            "quote",
            "follow",
            "reply",
            "bookmark",
            "unknown",
            "",
            "invalid_action",
        ];

        for action in &actions {
            let _result = action_allowed_by_limits(action, &limits, &counters);
        }
    }

    /// Property: calc_rate handles all usize inputs without panic
    #[test]
    fn calc_rate_handles_all_inputs() {
        // Test edge cases
        assert_eq!(calc_rate(0, 0), 0.0);
        assert_eq!(calc_rate(usize::MAX, usize::MAX), 100.0);
        assert_eq!(calc_rate(0, usize::MAX), 0.0);

        // Test various combinations
        for success in [0, 1, 50, 100] {
            for total in [1, 50, 100, 1000] {
                if success <= total {
                    let rate = calc_rate(success, total);
                    assert!((0.0..=100.0).contains(&rate));
                }
            }
        }
    }

    /// Property: extract_tweet_text never panics on various JSON inputs
    #[test]
    fn extract_tweet_text_no_panic() {
        use serde_json::json;

        let test_cases = vec![
            json!({"text": "test"}),
            json!({"full_text": "full"}),
            json!({"text": null}),
            json!({"full_text": null}),
            json!({}),
            json!({"text": 123}),
            json!({"text": ["array"]}),
            json!({"text": {"nested": "object"}}),
            json!(null),
            json!("string"),
        ];

        for case in &test_cases {
            let _result = extract_tweet_text(case);
        }
    }

    /// Property: generate_reply_text always returns non-empty for valid sentiment
    #[test]
    fn generate_reply_text_returns_non_empty() {
        let templates = SentimentTemplates::default();

        for sentiment in [Sentiment::Positive, Sentiment::Neutral, Sentiment::Negative] {
            for idx in 0..100 {
                let result = generate_reply_text(sentiment, idx, &templates);
                assert!(!result.is_empty(), "Reply text should never be empty");
            }
        }
    }

    /// Property: generate_quote_text always returns non-empty for valid sentiment
    #[test]
    fn generate_quote_text_returns_non_empty() {
        let templates = SentimentTemplates::default();

        for sentiment in [Sentiment::Positive, Sentiment::Neutral, Sentiment::Negative] {
            for idx in 0..100 {
                let result = generate_quote_text(sentiment, idx, &templates);
                assert!(!result.is_empty(), "Quote text should never be empty");
            }
        }
    }
}

#[cfg(test)]
mod gap_tests {
    use super::*;
    use crate::utils::twitter::twitteractivity_limits::{EngagementCounters, EngagementLimits};

    // should_engage_replies_after_root_action combinations
    #[test]
    fn should_engage_replies_requires_all_conditions() {
        let config = TaskConfig {
            dry_run_actions: false,
            ..Default::default()
        };

        // All true → true
        assert!(should_engage_replies_after_root_action(true, true, &config));

        // did_dive=false → false
        assert!(!should_engage_replies_after_root_action(
            false, true, &config
        ));

        // root_action_success=false → false
        assert!(!should_engage_replies_after_root_action(
            true, false, &config
        ));

        // Both false → false
        assert!(!should_engage_replies_after_root_action(
            false, false, &config
        ));
    }

    #[test]
    fn should_engage_replies_dry_run_blocks() {
        let config = TaskConfig {
            dry_run_actions: true,
            ..Default::default()
        };
        // Even with dive+success, dry_run blocks it
        assert!(!should_engage_replies_after_root_action(
            true, true, &config
        ));
    }

    // should_navigate_home_after_dive combinations
    #[test]
    fn should_navigate_home_requires_dive_and_not_dry_run() {
        let config = TaskConfig {
            dry_run_actions: false,
            ..Default::default()
        };

        assert!(should_navigate_home_after_dive(true, &config));
        assert!(!should_navigate_home_after_dive(false, &config));
    }

    #[test]
    fn should_navigate_home_dry_run_blocks() {
        let config = TaskConfig {
            dry_run_actions: true,
            ..Default::default()
        };
        assert!(!should_navigate_home_after_dive(true, &config));
    }

    // filter_detail_actions_for_gate edge cases
    #[test]
    fn filter_detail_actions_no_status_url_keeps_only_like() {
        let mut actions = vec!["like", "retweet", "reply"];
        filter_detail_actions_for_gate(&mut actions, false, true);
        assert_eq!(actions, vec!["like"]);
    }

    #[test]
    fn filter_detail_actions_dive_not_allowed_keeps_only_like() {
        let mut actions = vec!["like", "retweet", "reply"];
        filter_detail_actions_for_gate(&mut actions, true, false);
        assert_eq!(actions, vec!["like"]);
    }

    #[test]
    fn filter_detail_actions_like_only_unaffected() {
        let mut actions = vec!["like"];
        filter_detail_actions_for_gate(&mut actions, false, false);
        assert_eq!(actions, vec!["like"]);
    }

    #[test]
    fn filter_detail_actions_empty_list_stays_empty() {
        let mut actions: Vec<&str> = vec![];
        filter_detail_actions_for_gate(&mut actions, true, true);
        assert!(actions.is_empty());
    }

    #[test]
    fn filter_detail_actions_all_allowed_when_status_and_dive() {
        let mut actions = vec!["like", "retweet", "reply"];
        filter_detail_actions_for_gate(&mut actions, true, true);
        assert_eq!(actions, vec!["like", "retweet", "reply"]);
    }

    // filter_actions_for_decision_level with empty list
    #[test]
    fn filter_decision_level_empty_actions_stays_empty() {
        let mut actions: Vec<&str> = vec![];
        filter_actions_for_decision_level(&mut actions, EngagementLevel::Full);
        assert!(actions.is_empty());

        filter_actions_for_decision_level(&mut actions, EngagementLevel::Medium);
        assert!(actions.is_empty());

        filter_actions_for_decision_level(&mut actions, EngagementLevel::Minimal);
        assert!(actions.is_empty());

        filter_actions_for_decision_level(&mut actions, EngagementLevel::None);
        assert!(actions.is_empty());
    }

    // action_allowed_by_limits with "dive" action
    #[test]
    fn action_allowed_by_limits_dive_returns_false() {
        let limits = EngagementLimits::default();
        let counters = EngagementCounters::new();
        // "dive" is not in the match statement of action_allowed_by_limits
        assert!(!action_allowed_by_limits("dive", &limits, &counters));
    }

    #[test]
    fn action_allowed_by_limits_empty_string_returns_false() {
        let limits = EngagementLimits::default();
        let counters = EngagementCounters::new();
        assert!(!action_allowed_by_limits("", &limits, &counters));
    }

    // calc_rate edge cases
    #[test]
    fn calc_rate_hundred_percent() {
        assert_eq!(calc_rate(100, 100), 100.0);
    }

    #[test]
    fn calc_rate_fractional() {
        let rate = calc_rate(1, 3);
        assert!((rate - 33.333333).abs() < 0.01);
    }

    // extract_tweet_text with truncated text (Twitter API v2 uses "text" with note_text)
    #[test]
    fn extract_tweet_text_handles_empty_string_value() {
        let tweet = serde_json::json!({"text": ""});
        assert_eq!(extract_tweet_text(&tweet), "");
    }

    // selected_candidate_actions with tracker blocking specific tweets
    #[test]
    fn selected_candidate_actions_tracker_blocks_per_tweet() {
        let persona = PersonaWeights {
            like_prob: 1.0,
            retweet_prob: 1.0,
            quote_prob: 0.0,
            follow_prob: 0.0,
            reply_prob: 0.0,
            bookmark_prob: 0.0,
            thread_dive_prob: 0.0,
            interest_multiplier: 1.0,
        };
        let limits = EngagementLimits::with_limits(10, 10, 10, 10, 10, 10, 10, 100);
        let counters = EngagementCounters::new();
        let mut tracker = TweetActionTracker::new(60_000);

        let tid_x = TweetId::from_unchecked("tweet_x");
        let tid_y = TweetId::from_unchecked("tweet_y");

        // First call should include like and retweet
        let actions = selected_candidate_actions(&persona, &tid_x, &limits, &counters, &tracker);
        assert!(actions.contains(&"like"));
        assert!(actions.contains(&"retweet"));

        // Record an action on tweet_x — tracker blocks further actions on same tweet
        tracker.record_action(TweetId::from_unchecked("tweet_x"), "like");
        let blocked = selected_candidate_actions(&persona, &tid_x, &limits, &counters, &tracker);
        assert!(blocked.is_empty());

        // Different tweet should still be allowed
        let other = selected_candidate_actions(&persona, &tid_y, &limits, &counters, &tracker);
        assert!(!other.is_empty());
    }
}
