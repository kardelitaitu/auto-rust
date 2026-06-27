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
        bookmark_at_position, extract_tweet_button_position, follow_at_position,
        generate_quote_text, generate_reply_text, like_at_position, retweet_at_position,
    },
    twitteractivity_helpers::validate_tweet_page,
    twitteractivity_humanized::{clustered_engagement_pause, clustered_reply_pause},
    twitteractivity_interact::{bookmark_tweet, follow_from_tweet, like_tweet, reply_to_tweet},
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

fn log_engagement_failure(outcome: &EngagementOutcome, action: &str, tweet_id: &TweetId) {
    match outcome {
        EngagementOutcome::AlreadyDone => {
            info!("[{action}] Skipping {action} for {tweet_id}: already performed");
        }
        EngagementOutcome::ElementNotFound => {
            warn!("[{action}] Failed {action} for {tweet_id}: required UI element not found");
        }
        EngagementOutcome::Failed => {
            warn!("[{action}] Failed {action} for {tweet_id}: action execution failed");
        }
        _ => {}
    }
}

fn log_follow_failure(outcome: &FollowOutcome, tweet_id: &TweetId) {
    match outcome {
        FollowOutcome::AlreadyFollowing => {
            info!("[follow] Skipping follow for {tweet_id}: already following");
        }
        FollowOutcome::ButtonNotFound => {
            warn!("[follow] Failed follow for {tweet_id}: follow button not found");
        }
        FollowOutcome::Failed => {
            warn!("[follow] Failed follow for {tweet_id}: follow action failed");
        }
        _ => {}
    }
}

async fn scroll_and_get_button_pos(
    api: &TaskContext,
    tweet_id: &TweetId,
    button_name: &str,
) -> Option<(f64, f64)> {
    let js = crate::utils::twitter::twitteractivity_selectors::js_scroll_and_get_tweet_button()
        .replace("{TWEET_ID}", tweet_id.as_str())
        .replace("{BUTTON_NAME}", button_name);
    match api.page().evaluate(js).await {
        Ok(res) => res.value().and_then(|v| {
            let obj = v.as_object()?;
            let x = obj.get("x")?.as_f64()?;
            let y = obj.get("y")?.as_f64()?;
            Some((x, y))
        }),
        Err(e) => {
            warn!("[dispatch] Failed to resolve button position after scroll: {e}");
            None
        }
    }
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
                "[{action}] Dry-run: would skip {action} on tweet {tweet_id} because thread detail did not open"
            );
            return Ok(false);
        }
        info!(
            "[{action}] Dry-run: would perform {action} on tweet {tweet_id} (did_dive={did_dive})"
        );
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
                    Ok(outcome) => {
                        let ok = engagement_success(&outcome);
                        if !ok {
                            log_engagement_failure(&outcome, "like", tweet_id);
                        }
                        ok
                    }
                    Err(e) => {
                        warn!("[like] Like failed after retries: {e}");
                        api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                        api.increment_run_counter(RUN_COUNTER_LIKE_FAILURE, 1);
                        false
                    }
                }
            } else {
                // On feed, dynamically scroll and resolve position, with fallback to candidate data
                let action_fn = || {
                    let tweet_id = tweet_id.clone();
                    async move {
                        let pos = scroll_and_get_button_pos(api, &tweet_id, "like")
                            .await
                            .or_else(|| extract_tweet_button_position(tweet, "like"))
                            .ok_or_else(|| anyhow::anyhow!("Like button not found"))?;
                        like_at_position(api, pos.0, pos.1).await
                    }
                };
                match retry_with_backoff(
                    action_fn,
                    &RetryConfig::aggressive(),
                    api,
                    "like_at_position",
                )
                .await
                {
                    Ok(outcome) => {
                        let ok = engagement_success(&outcome);
                        if !ok {
                            log_engagement_failure(&outcome, "like_at_position", tweet_id);
                        }
                        ok
                    }
                    Err(e) => {
                        warn!("[like] Like at position failed after retries: {e}");
                        api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                        api.increment_run_counter(RUN_COUNTER_LIKE_FAILURE, 1);
                        false
                    }
                }
            }
        }
        "retweet" => {
            if !did_dive {
                // Position-based retweet from feed (no thread dive needed)
                let action_fn = || {
                    let tweet_id = tweet_id.clone();
                    async move {
                        let pos = scroll_and_get_button_pos(api, &tweet_id, "retweet")
                            .await
                            .or_else(|| extract_tweet_button_position(tweet, "retweet"))
                            .ok_or_else(|| anyhow::anyhow!("Retweet button not found"))?;
                        retweet_at_position(api, pos.0, pos.1).await
                    }
                };
                match retry_with_backoff(
                    action_fn,
                    &RetryConfig::default(),
                    api,
                    "retweet_at_position",
                )
                .await
                {
                    Ok(outcome) => {
                        let ok = engagement_success(&outcome);
                        if !ok {
                            log_engagement_failure(&outcome, "retweet", tweet_id);
                        }
                        ok
                    }
                    Err(e) => {
                        warn!("[retweet] Feed retweet failed after retries: {e}");
                        api.increment_run_counter(RUN_COUNTER_RETWEET_FAILURE, 1);
                        false
                    }
                }
            } else {
                // Retweets only from home feed during scrolling — skip on detail view
                info!("[retweet] Skipping retweet for {tweet_id}: retweets only from home feed");
                false
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
                if did_dive && task_config.llm_enabled {
                    read_replies_for_context(api, did_dive).await;
                }
                let quote_text = if task_config.llm_enabled {
                    match extract_tweet_context(api).await {
                        Ok((author, text, replies)) if !text.is_empty() && author != "unknown" => {
                            match generate_quote_commentary(api, &author, &text, replies, sentiment)
                                .await
                            {
                                Ok(commentary) => {
                                    info!("[quote] Generated LLM quote: {commentary}");
                                    commentary
                                }
                                Err(e) => {
                                    warn!("[quote] LLM quote failed, using template: {e}");
                                    template_quote()
                                }
                            }
                        }
                        Ok((_, ref text, _)) => {
                            warn!(
                                "[quote] Skipping LLM quote: extracted context is empty/unknown (text_len={}), using template",
                                text.len()
                            );
                            template_quote()
                        }
                        Err(e) => {
                            warn!("[quote] Failed to extract tweet context for quote: {e}, using template");
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
                            info!(
                                "💬 [SUCCESS] Quote tweeted tweet {} with commentary: {}",
                                tweet_id, quote_text
                            );
                        } else {
                            log_engagement_failure(&outcome, "quote", tweet_id);
                        }
                        success
                    }
                    Err(e) => {
                        warn!("[quote] Quote tweet error: {e}");
                        false
                    }
                }
            }
        }
        "follow" => {
            if !did_dive {
                // Position-based follow from feed
                let action_fn = || {
                    let tweet_id = tweet_id.clone();
                    async move {
                        let pos = scroll_and_get_button_pos(api, &tweet_id, "follow")
                            .await
                            .or_else(|| extract_tweet_button_position(tweet, "follow"))
                            .ok_or_else(|| anyhow::anyhow!("Follow button not found"))?;
                        follow_at_position(api, pos.0, pos.1).await
                    }
                };
                match retry_with_backoff(
                    action_fn,
                    &RetryConfig::default(),
                    api,
                    "follow_at_position",
                )
                .await
                {
                    Ok(outcome) => {
                        let ok = engagement_success(&outcome);
                        if !ok {
                            log_engagement_failure(&outcome, "follow", tweet_id);
                        }
                        ok
                    }
                    Err(e) => {
                        warn!("[follow] Feed follow failed after retries: {e}");
                        api.increment_run_counter(RUN_COUNTER_FOLLOW_FAILURE, 1);
                        false
                    }
                }
            } else {
                if !validate_tweet_page(api, did_dive, "follow", tweet_id).await {
                    false
                } else {
                    retry_follow(api, tweet_id).await
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
                if did_dive && task_config.llm_enabled {
                    read_replies_for_context(api, did_dive).await;
                }
                let reply_text = if task_config.llm_enabled {
                    match extract_tweet_context(api).await {
                        Ok((author, text, replies)) if !text.is_empty() && author != "unknown" => {
                            match generate_reply(api, &author, &text, replies, sentiment).await {
                                Ok(reply) => {
                                    info!("[reply] Generated LLM reply: {reply}");
                                    reply
                                }
                                Err(e) => {
                                    warn!("[reply] LLM reply failed, using template: {e}");
                                    template_reply()
                                }
                            }
                        }
                        Ok((_, ref text, _)) => {
                            warn!(
                                "[reply] Skipping LLM reply: extracted context is empty/unknown (text_len={}), using template",
                                text.len()
                            );
                            template_reply()
                        }
                        Err(e) => {
                            warn!("[reply] Failed to extract tweet context for reply: {e}, using template");
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
                    Ok(outcome) => {
                        let ok = engagement_success(&outcome);
                        if ok {
                            info!(
                                "📝 [SUCCESS] Replied to tweet {} with sentiment {:?}: {}",
                                tweet_id, sentiment, reply_text
                            );
                        } else {
                            log_engagement_failure(&outcome, "reply", tweet_id);
                        }
                        ok
                    }
                    Err(e) => {
                        warn!("[reply] Reply failed after retries: {e}");
                        api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                        api.increment_run_counter(RUN_COUNTER_REPLY_FAILURE, 1);
                        false
                    }
                }
            }
        }
        "bookmark" => {
            if !did_dive {
                // Position-based bookmark from feed
                let action_fn = || {
                    let tweet_id = tweet_id.clone();
                    async move {
                        let pos = scroll_and_get_button_pos(api, &tweet_id, "bookmark")
                            .await
                            .or_else(|| extract_tweet_button_position(tweet, "bookmark"))
                            .ok_or_else(|| anyhow::anyhow!("Bookmark button not found"))?;
                        bookmark_at_position(api, pos.0, pos.1).await
                    }
                };
                match retry_with_backoff(
                    action_fn,
                    &RetryConfig::aggressive(),
                    api,
                    "bookmark_at_position",
                )
                .await
                {
                    Ok(outcome) => {
                        let ok = engagement_success(&outcome);
                        if !ok {
                            log_engagement_failure(&outcome, "bookmark", tweet_id);
                        }
                        ok
                    }
                    Err(e) => {
                        warn!("[bookmark] Feed bookmark failed after retries: {e}");
                        api.increment_run_counter(RUN_COUNTER_BOOKMARK_FAILURE, 1);
                        false
                    }
                }
            } else {
                if !validate_tweet_page(api, did_dive, "bookmark", tweet_id).await {
                    false
                } else {
                    selector_bookmark(api, tweet_id).await
                }
            }
        }
        _ => false,
    };

    if success {
        // Update counters and record action
        match action {
            "like" => {
                info!("💖 [SUCCESS] Liked tweet {}", tweet_id);
                counters.increment_like();
                api.increment_run_counter(RUN_COUNTER_LIKE_SUCCESS, 1);
            }
            "retweet" => {
                info!("🔁 [SUCCESS] Retweeted tweet {}", tweet_id);
                counters.increment_retweet();
                api.increment_run_counter(RUN_COUNTER_RETWEET_SUCCESS, 1);
            }
            "quote" => {
                // Logged in detail with commentary text above
                counters.increment_quote_tweet();
                api.increment_run_counter(RUN_COUNTER_QUOTE_SUCCESS, 1);
            }
            "follow" => {
                info!("👤 [SUCCESS] Followed user from tweet {}", tweet_id);
                counters.increment_follow();
                api.increment_run_counter(RUN_COUNTER_FOLLOW_SUCCESS, 1);
            }
            "reply" => {
                // Logged in detail with text above
                counters.increment_reply();
                api.increment_run_counter(RUN_COUNTER_REPLY_SUCCESS, 1);
            }
            "bookmark" => {
                info!("🔖 [SUCCESS] Bookmarked tweet {}", tweet_id);
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

/// Selector-based follow logic (used from detail view or as position fallback).
async fn retry_follow(api: &TaskContext, tweet_id: &TweetId) -> bool {
    match retry_with_backoff(
        || follow_from_tweet(api),
        &RetryConfig::default(),
        api,
        "follow_from_tweet",
    )
    .await
    {
        Ok(outcome) => {
            let ok = follow_success(&outcome);
            if !ok {
                log_follow_failure(&outcome, tweet_id);
            }
            ok
        }
        Err(e) => {
            warn!("[follow] Follow failed after retries: {e}");
            api.increment_run_counter(RUN_COUNTER_FOLLOW_FAILURE, 1);
            false
        }
    }
}

/// Selector-based bookmark logic (used from detail view or as position fallback).
async fn selector_bookmark(api: &TaskContext, tweet_id: &TweetId) -> bool {
    match retry_with_backoff(
        || bookmark_tweet(api),
        &RetryConfig::aggressive(),
        api,
        "bookmark_tweet",
    )
    .await
    {
        Ok(outcome) => {
            let ok = engagement_success(&outcome);
            if !ok {
                log_engagement_failure(&outcome, "bookmark", tweet_id);
            }
            ok
        }
        Err(e) => {
            warn!("[bookmark] bookmark_tweet failed after retries: {e}");
            api.increment_run_counter(RUN_COUNTER_BOOKMARK_FAILURE, 1);
            false
        }
    }
}

async fn read_replies_for_context(api: &TaskContext, did_dive: bool) {
    if did_dive {
        info!("[context] Reading replies to build conversation context...");
        // Scroll down to load and read replies (pauses=2, scroll=500px, variable_speed=true, back_scroll=true)
        if let Err(e) = api.scroll_read(2, 500, true, true).await {
            warn!("[context] Failed to scroll read replies: {e}");
        }
        // Make sure we are at the top to access the tweet/buttons
        if let Err(e) = api.scroll_to_top().await {
            warn!("[context] Failed to scroll back to top: {e}");
        }
    }
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
