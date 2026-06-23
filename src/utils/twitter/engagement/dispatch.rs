//! Action dispatch logic for tweet engagement.

use super::super::twitteractivity_retry::{retry_with_backoff, RetryConfig};
use super::super::twitteractivity_state::{TaskConfig, TweetActionTracker};
use crate::metrics::{
    RUN_COUNTER_BOOKMARK_FAILURE, RUN_COUNTER_BOOKMARK_SUCCESS, RUN_COUNTER_CLICK_VERIFY_FAILED,
    RUN_COUNTER_FOLLOW_FAILURE, RUN_COUNTER_FOLLOW_SUCCESS, RUN_COUNTER_LIKE_FAILURE,
    RUN_COUNTER_LIKE_SUCCESS, RUN_COUNTER_QUOTE_FAILURE, RUN_COUNTER_QUOTE_SUCCESS,
    RUN_COUNTER_REPLY_FAILURE, RUN_COUNTER_REPLY_SUCCESS, RUN_COUNTER_RETWEET_FAILURE,
    RUN_COUNTER_RETWEET_SUCCESS, RUN_COUNTER_TRANSIENT_ERROR,
};
use crate::prelude::TaskContext;
use crate::utils::twitter::twitteractivity_types::{EngagementOutcome, FollowOutcome, TweetId};
use crate::utils::twitter::{
    sentiment::Sentiment,
    twitteractivity_actions::{
        extract_tweet_button_position, generate_quote_text, generate_reply_text, like_at_position,
    },
    twitteractivity_helpers::validate_tweet_page,
    twitteractivity_humanized::{clustered_engagement_pause, clustered_reply_pause},
    twitteractivity_interact::{
        bookmark_tweet, follow_from_tweet, like_tweet, reply_to_tweet, retweet_tweet,
    },
    twitteractivity_limits::EngagementCounters,
    twitteractivity_llm::{
        extract_tweet_context, generate_quote_commentary, generate_reply, quote_tweet,
    },
};
use anyhow::Result;
use log::{info, warn};
use serde_json::Value;

fn engagement_success(outcome: &EngagementOutcome) -> bool {
    matches!(
        outcome,
        EngagementOutcome::Completed | EngagementOutcome::Unverified
    )
}

fn follow_success(outcome: &FollowOutcome) -> bool {
    matches!(outcome, FollowOutcome::Followed)
}

/// Dispatch a single engagement action with full retry, validation, and metrics tracking.
#[allow(clippy::too_many_arguments)]
pub async fn dispatch_action(
    api: &TaskContext,
    action: &'static str,
    tweet: &Value,
    tweet_id: &TweetId,
    did_dive: bool,
    sentiment: Sentiment,
    task_config: &TaskConfig,
    counters: &mut EngagementCounters,
    action_tracker: &mut TweetActionTracker,
    actions_this_scan: &mut u32,
) -> Result<bool> {
    if task_config.dry_run_actions {
        if action != "like" && !did_dive {
            info!(
                "Dry-run: would skip {action} on tweet {tweet_id} because thread detail did not open"
            );
            return Ok(false);
        }
        info!("Dry-run: would perform {action} on tweet {tweet_id} (did_dive={did_dive})");
        counters.increment(action);
        *actions_this_scan += 1;
        action_tracker.record_action(tweet_id.clone(), action);
        return Ok(true);
    }

    let success = match action {
        "like" => {
            // Like can be done on feed or in detail
            if did_dive {
                // In detail view, use general like function with retry
                match retry_with_backoff(
                    || like_tweet(api),
                    &RetryConfig::aggressive(),
                    api,
                    "like_tweet",
                )
                .await
                {
                    Ok(outcome) => engagement_success(&outcome),
                    Err(e) => {
                        warn!("Like failed after retries: {e}");
                        api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                        api.increment_run_counter(RUN_COUNTER_LIKE_FAILURE, 1);
                        false
                    }
                }
            } else {
                // On feed, use position from tweet data with retry
                if let Some(btn_pos) = extract_tweet_button_position(tweet, "like") {
                    match retry_with_backoff(
                        || like_at_position(api, btn_pos.0, btn_pos.1),
                        &RetryConfig::aggressive(),
                        api,
                        "like_at_position",
                    )
                    .await
                    {
                        Ok(outcome) => engagement_success(&outcome),
                        Err(e) => {
                            warn!("Like at position failed after retries: {e}");
                            api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                            api.increment_run_counter(RUN_COUNTER_LIKE_FAILURE, 1);
                            false
                        }
                    }
                } else {
                    warn!("Like button position not found in tweet payload for {tweet_id}, falling back to selector-based like");
                    // Fallback to selector-based like (works on feed too)
                    match retry_with_backoff(
                        || like_tweet(api),
                        &RetryConfig::aggressive(),
                        api,
                        "like_tweet",
                    )
                    .await
                    {
                        Ok(outcome) => engagement_success(&outcome),
                        Err(e) => {
                            warn!("Selector-based like failed after retries: {e}");
                            api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                            api.increment_run_counter(RUN_COUNTER_LIKE_FAILURE, 1);
                            false
                        }
                    }
                }
            }
        }
        "retweet" => {
            if !validate_tweet_page(api, did_dive, "retweet", tweet_id).await {
                false
            } else {
                match retry_with_backoff(
                    || retweet_tweet(api),
                    &RetryConfig::default(),
                    api,
                    "retweet_tweet",
                )
                .await
                {
                    Ok(outcome) => engagement_success(&outcome),
                    Err(e) => {
                        warn!("retweet_tweet failed after retries: {e}");
                        api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                        api.increment_run_counter(RUN_COUNTER_RETWEET_FAILURE, 1);
                        false
                    }
                }
            }
        }
        "quote" => {
            if !validate_tweet_page(api, did_dive, "quote", tweet_id).await {
                false
            } else {
                let template_quote = || {
                    generate_quote_text(
                        sentiment,
                        counters.quote_tweets(),
                        &task_config.sentiment_templates,
                    )
                };
                let quote_text = if task_config.llm_enabled {
                    match extract_tweet_context(api).await {
                        Ok((author, text, replies)) if !text.is_empty() && author != "unknown" => {
                            match generate_quote_commentary(api, &author, &text, replies).await {
                                Ok(commentary) => {
                                    info!("Generated LLM quote: {commentary}");
                                    commentary
                                }
                                Err(e) => {
                                    warn!("LLM quote failed, using template: {e}");
                                    template_quote()
                                }
                            }
                        }
                        Ok((_, ref text, _)) => {
                            warn!(
                                "Skipping LLM quote: extracted context is empty/unknown (text_len={}), using template",
                                text.len()
                            );
                            template_quote()
                        }
                        Err(e) => {
                            warn!("Failed to extract tweet context for quote: {e}, using template");
                            template_quote()
                        }
                    }
                } else {
                    template_quote()
                };
                match quote_tweet(api, &quote_text).await {
                    Ok(outcome) => {
                        let success = engagement_success(&outcome);
                        if success {
                            info!("Quote tweeted with commentary: {quote_text}");
                        }
                        success
                    }
                    Err(e) => {
                        warn!("Quote tweet error: {e}");
                        false
                    }
                }
            }
        }
        "follow" => {
            if !validate_tweet_page(api, did_dive, "follow", tweet_id).await {
                false
            } else {
                match retry_with_backoff(
                    || follow_from_tweet(api),
                    &RetryConfig::default(),
                    api,
                    "follow_from_tweet",
                )
                .await
                {
                    Ok(outcome) => follow_success(&outcome),
                    Err(e) => {
                        warn!("Follow failed after retries: {e}");
                        api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                        api.increment_run_counter(RUN_COUNTER_FOLLOW_FAILURE, 1);
                        false
                    }
                }
            }
        }
        "reply" => {
            if !validate_tweet_page(api, did_dive, "reply", tweet_id).await {
                false
            } else {
                let template_reply = || {
                    generate_reply_text(
                        sentiment,
                        counters.replies(),
                        &task_config.sentiment_templates,
                    )
                };
                let reply_text = if task_config.llm_enabled {
                    match extract_tweet_context(api).await {
                        Ok((author, text, replies)) if !text.is_empty() && author != "unknown" => {
                            match generate_reply(api, &author, &text, replies).await {
                                Ok(reply) => {
                                    info!("Generated LLM reply: {reply}");
                                    reply
                                }
                                Err(e) => {
                                    warn!("LLM reply failed, using template: {e}");
                                    template_reply()
                                }
                            }
                        }
                        Ok((_, ref text, _)) => {
                            warn!(
                                "Skipping LLM reply: extracted context is empty/unknown (text_len={}), using template",
                                text.len()
                            );
                            template_reply()
                        }
                        Err(e) => {
                            warn!("Failed to extract tweet context for reply: {e}, using template");
                            template_reply()
                        }
                    }
                } else {
                    template_reply()
                };
                match retry_with_backoff(
                    || reply_to_tweet(api, &reply_text),
                    &RetryConfig::conservative(),
                    api,
                    "reply_to_tweet",
                )
                .await
                {
                    Ok(outcome) => engagement_success(&outcome),
                    Err(e) => {
                        warn!("Reply failed after retries: {e}");
                        api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                        api.increment_run_counter(RUN_COUNTER_REPLY_FAILURE, 1);
                        false
                    }
                }
            }
        }
        "bookmark" => {
            if !validate_tweet_page(api, did_dive, "bookmark", tweet_id).await {
                false
            } else {
                match retry_with_backoff(
                    || bookmark_tweet(api),
                    &RetryConfig::aggressive(),
                    api,
                    "bookmark_tweet",
                )
                .await
                {
                    Ok(outcome) => engagement_success(&outcome),
                    Err(e) => {
                        warn!("bookmark_tweet failed after retries: {e}");
                        api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                        api.increment_run_counter(RUN_COUNTER_BOOKMARK_FAILURE, 1);
                        false
                    }
                }
            }
        }
        _ => false,
    };

    if success {
        // Update counters and record action
        match action {
            "like" => {
                info!("Liked tweet");
                counters.increment_like();
                api.increment_run_counter(RUN_COUNTER_LIKE_SUCCESS, 1);
            }
            "retweet" => {
                info!("Retweeted");
                counters.increment_retweet();
                api.increment_run_counter(RUN_COUNTER_RETWEET_SUCCESS, 1);
            }
            "quote" => {
                counters.increment_quote_tweet();
                api.increment_run_counter(RUN_COUNTER_QUOTE_SUCCESS, 1);
            }
            "follow" => {
                info!("Followed user");
                counters.increment_follow();
                api.increment_run_counter(RUN_COUNTER_FOLLOW_SUCCESS, 1);
            }
            "reply" => {
                info!("Replied with sentiment {sentiment:?}");
                counters.increment_reply();
                api.increment_run_counter(RUN_COUNTER_REPLY_SUCCESS, 1);
            }
            "bookmark" => {
                info!("Bookmarked tweet");
                counters.increment_bookmark();
                api.increment_run_counter(RUN_COUNTER_BOOKMARK_SUCCESS, 1);
            }
            _ => {}
        }
        *actions_this_scan += 1;
        action_tracker.record_action(tweet_id.clone(), action);

        // Use appropriate pause
        if action == "reply" || action == "quote" {
            clustered_reply_pause(api).await;
        } else {
            clustered_engagement_pause(api).await;
        }
    } else {
        match action {
            "like" => {
                api.increment_run_counter(RUN_COUNTER_CLICK_VERIFY_FAILED, 1);
                api.increment_run_counter(RUN_COUNTER_LIKE_FAILURE, 1);
            }
            "retweet" => api.increment_run_counter(RUN_COUNTER_RETWEET_FAILURE, 1),
            "quote" => api.increment_run_counter(RUN_COUNTER_QUOTE_FAILURE, 1),
            "follow" => api.increment_run_counter(RUN_COUNTER_FOLLOW_FAILURE, 1),
            "reply" => api.increment_run_counter(RUN_COUNTER_REPLY_FAILURE, 1),
            "bookmark" => api.increment_run_counter(RUN_COUNTER_BOOKMARK_FAILURE, 1),
            _ => {}
        }
    }

    Ok(success)
}

#[cfg(test)]
mod pure_function_tests {
    use super::*;

    // ====================================================================
    // engagement_success
    // ====================================================================

    #[test]
    fn engagement_success_completed() {
        assert!(engagement_success(&EngagementOutcome::Completed));
    }

    #[test]
    fn engagement_success_unverified() {
        assert!(engagement_success(&EngagementOutcome::Unverified));
    }

    #[test]
    fn engagement_success_failed() {
        assert!(!engagement_success(&EngagementOutcome::Failed));
    }

    #[test]
    fn engagement_success_element_not_found() {
        assert!(!engagement_success(&EngagementOutcome::ElementNotFound));
    }

    #[test]
    fn engagement_success_skipped() {
        assert!(!engagement_success(&EngagementOutcome::AlreadyDone));
    }

    // ====================================================================
    // follow_success
    // ====================================================================

    #[test]
    fn follow_success_followed() {
        assert!(follow_success(&FollowOutcome::Followed));
    }

    #[test]
    fn follow_success_already_following() {
        assert!(!follow_success(&FollowOutcome::AlreadyFollowing));
    }

    #[test]
    fn follow_success_button_not_found() {
        assert!(!follow_success(&FollowOutcome::ButtonNotFound));
    }

    #[test]
    fn follow_success_failed() {
        assert!(!follow_success(&FollowOutcome::Failed));
    }
}
