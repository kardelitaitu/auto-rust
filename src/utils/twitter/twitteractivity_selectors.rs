//! Centralized JavaScript selector snippets for Twitter/X DOM queries.
//!
//! All selectors are designed for Twitter/X's dynamic class structure.
//! Functions return JS code as &'static str that can be passed to `api.page().evaluate()`.

/// Returns the best selector to detect that the main timeline/feed is visible.
/// Used to verify successful navigation to the home timeline.
#[must_use]
pub fn selector_feed_visible() -> &'static str {
    include_str!("js/selector_feed_visible.js")
}

/// Returns JS to find the center coordinates of the first element matching a selector.
/// Returns `{x, y}` or `null` if not found.
#[must_use]
pub fn selector_element_center(selector: &str) -> String {
    include_str!("js/selector_element_center.js")
        .replace("{SELECTOR}", &selector.replace('"', "\\\""))
}

/// Returns JS to query all tweet/article elements currently in the DOM.
/// Returns an array of objects with tweetId (from data-item-id or similar) and bounding rect.
#[must_use]
pub fn selector_all_tweets() -> &'static str {
    include_str!("js/selector_all_tweets.js")
}

/// Returns JS to find visible follow buttons within a tweet/article element.
#[must_use]
pub fn selector_follow_button() -> &'static str {
    include_str!("js/selector_follow_button.js")
}

/// Returns JS to find like/retweet/reply buttons for a given tweet element.
#[must_use]
pub fn selector_engagement_buttons() -> &'static str {
    include_str!("js/selector_engagement_buttons.js")
}

/// Returns JS to check if current page shows a login/onboarding flow.
#[must_use]
pub fn selector_login_flow() -> &'static str {
    include_str!("js/selector_login_flow.js")
}

/// Returns JS to detect if a popup/modal is present (e.g., "Follow on Twitter" prompt, cookies, etc.)
#[must_use]
pub fn selector_popup_overlay() -> &'static str {
    include_str!("js/selector_popup_overlay.js")
}

/// Returns JS to check if a "Follow on X" (external site) confirmation modal is open.
#[must_use]
pub fn selector_follow_confirm_modal() -> &'static str {
    include_str!("js/selector_follow_confirm_modal.js")
}

/// Returns JS to find a close button (X) for a modal/dialog overlay.
#[must_use]
pub fn selector_close_button() -> &'static str {
    include_str!("js/selector_close_button.js")
}

/// Returns JS to find the "Following" state indicator on a user profile or tweet.
#[must_use]
pub fn selector_following_indicator() -> &'static str {
    include_str!("js/selector_following_indicator.js")
}

/// Returns JS to get current URL (for verifying navigation).
#[must_use]
pub fn js_get_current_url() -> &'static str {
    include_str!("js/js_get_current_url.js")
}

// --- CSS Selector Constants ---

/// Home logo selector (X logo)
pub const HOME_LOGO_SELECTOR: &str = r#"a[aria-label="X"]"#;

/// Tweet link selector (links to individual tweets)
pub const TWEET_LINK_SELECTOR: &str = r#"a[href*="/status/"]"#;

/// Tweet detail/dialog selector
pub const TWEET_DETAIL_SELECTOR: &str = r#"div[role="dialog"]"#;

/// Tweet detail fallback selectors
pub const TWEET_DETAIL_FALLBACK1: &str = r#"div[data-testid="tweetDetail"]"#;
pub const TWEET_DETAIL_FALLBACK2: &str = r#"div[data-testid="tweetThread"]"#;
pub const TWEET_DETAIL_FALLBACK3: &str = r#"[aria-label="Timeline: Thread"]"#;
pub const TWEET_DETAIL_FALLBACK4: &str = r#"article[data-testid="tweet"]"#;

/// Retweet button selector
pub const RETWEET_BUTTON_SELECTOR: &str = r#"button[data-testid="retweet"]"#;

/// Retweet confirm button selector
pub const RETWEET_CONFIRM_SELECTOR: &str = r#"div[data-testid="retweetConfirm"]"#;

/// Like button selector
pub const LIKE_BUTTON_SELECTOR: &str = r#"button[data-testid="like"]"#;

/// Follow button selector (ending with -follow)
pub const FOLLOW_BUTTON_SELECTOR: &str = r#"button[data-testid$="-follow"]"#;

/// Bookmark button selector
pub const BOOKMARK_BUTTON_SELECTOR: &str = r#"button[data-testid="bookmark"]"#;

// --- Additional Selector Constants (for inline selector cleanup) ---

/// Tweet textarea selector (for reply composition)
pub const TWEET_TEXTAREA_SELECTOR: &str = r#"[data-testid="tweetTextarea_0"]"#;

/// Role textbox selector
pub const ROLE_TEXTBOX_SELECTOR: &str = r#"[role="textbox"]"#;

/// Button with role=button selector
pub const BUTTON_ROLE_BUTTON_SELECTOR: &str = r#"button, [role="button"]"#;

/// Subscribe button selector (for follow checks)
pub const SUBSCRIBE_BUTTON_SELECTOR: &str = r#"button[data-testid*="-subscribe"]"#;

/// Article tweet selector (for feed scanning)
pub const ARTICLE_TWEET_SELECTOR: &str = r#"article[data-testid="tweet"]"#;

/// Tweet text selector
pub const TWEET_TEXT_SELECTOR: &str = r#"[data-testid="tweetText"]"#;

/// Reply button selector
pub const REPLY_BUTTON_SELECTOR: &str = r#"button[data-testid="reply"]"#;

/// Tweet reply selector (for extracting replies)
pub const TWEET_REPLY_SELECTOR: &str = r#"[data-testid="tweetReply"]"#;

// --- Attribute-only selectors (for use with element.querySelector) ---

/// Like data-testid attribute selector (element-agnostic)
pub const LIKE_TESTID_SELECTOR: &str = r#"[data-testid="like"]"#;

/// Retweet data-testid attribute selector (element-agnostic)
pub const RETWEET_TESTID_SELECTOR: &str = r#"[data-testid="retweet"]"#;

/// Reply data-testid attribute selector (element-agnostic)
pub const REPLY_TESTID_SELECTOR: &str = r#"[data-testid="reply"]"#;

/// Dir auto span selector (for reply author extraction)
pub const DIR_AUTO_SPAN_SELECTOR: &str = r#"[dir="auto"] span:first-child"#;

/// Tweet button selector (generic button search)
pub const TWEET_BUTTON_SELECTOR: &str = r"button[data-testid], a[data-testid]";

/// Retweet confirm button selector (in modal/dialog)
pub const RETWEET_CONFIRM_BUTTON_SELECTOR: &str = "button[data-testid=\"retweetConfirm\"]";

/// Tweet button inline selector (reply submit button in composer)
pub const TWEET_BUTTON_INLINE_SELECTOR: &str = "button[data-testid=\"tweetButtonInline\"]";

/// Returns JS to find and return center coordinates of the retweet confirm button.
/// Returns `{x, y}` or `null` if not found.
#[must_use]
pub fn js_confirm_retweet_click() -> &'static str {
    include_str!("js/js_confirm_retweet_click.js")
}

/// Returns JS to find and focus the reply textarea.
/// Returns `{found: true}` if found and focused, `{found: false}` otherwise.
#[must_use]
pub fn js_find_reply_textarea() -> &'static str {
    include_str!("js/js_find_reply_textarea.js")
}

/// Returns JS to find the reply submit button and return its center coordinates.
/// Returns `{x, y}` or `null` if not found.
#[must_use]
pub fn js_find_reply_submit_button() -> &'static str {
    include_str!("js/js_find_reply_submit_button.js")
}

/// Returns JS to generate center coordinates for a button within the root tweet.
/// Takes a CSS selector string and searches within the first visible tweet article.
/// Returns `{x, y}` or `null` if not found.
#[must_use]
pub fn js_root_tweet_button_center(selector: &str) -> String {
    include_str!("js/js_root_tweet_button_center.js")
        .replace("{SELECTOR}", &selector.replace('"', "\\\""))
}

/// Returns JS to identify reply candidates within a thread dive.
/// Skips the root tweet and returns engageable replies with text and button positions.
#[must_use]
pub fn js_identify_thread_replies() -> &'static str {
    include_str!("js/js_identify_thread_replies.js")
}

/// Returns JS to identify engagement candidates in the current feed.
/// Returns an array of tweet objects with id, text, button positions, and replies.
#[must_use]
pub fn js_identify_engagement_candidates() -> &'static str {
    include_str!("js/js_identify_engagement_candidates.js")
}

/// Returns JS to extract username from a profile page (if navigated to /username).
#[must_use]
pub fn js_extract_username_from_url() -> &'static str {
    include_str!("js/js_extract_username_from_url.js")
}

/// Returns JS to find and click the user avatar in a tweet to navigate to their profile.
/// Returns coordinates or null if not found.
#[must_use]
pub fn selector_tweet_user_avatar() -> &'static str {
    include_str!("js/selector_tweet_user_avatar.js")
}

/// Returns JS to perform a quick health check on critical selectors.
/// Returns an object with health status of each selector type.
#[must_use]
pub fn selector_health_check() -> &'static str {
    include_str!("js/selector_health_check.js")
}

/// Returns JS to verify a like was registered by checking button state.
/// Replaces {X} and {Y} with click coordinates.
#[must_use]
pub fn js_verify_like(x: f64, y: f64) -> String {
    include_str!("js/js_verify_like.js")
        .replace("{X}", &x.to_string())
        .replace("{Y}", &y.to_string())
}

/// Returns JS to extract tweet context (author, text, replies) for LLM.
#[must_use]
pub fn js_extract_tweet_context() -> &'static str {
    include_str!("js/js_extract_tweet_context.js")
}

/// Returns JS to find the quote tweet button in the retweet menu.
/// Returns `{x, y}` or `null` if not found.
#[must_use]
pub fn js_find_quote_button() -> &'static str {
    include_str!("js/js_find_quote_button.js")
}

/// Returns JS to find and focus the composer textarea for quote tweets.
/// Returns `true` if found and focused, `false` otherwise.
#[must_use]
pub fn js_focus_composer() -> &'static str {
    include_str!("js/js_focus_composer.js")
}

/// Returns JS to find the tweet/post button in the composer.
/// Returns `{x, y}` or `null` if not found.
#[must_use]
pub fn js_find_tweet_button() -> &'static str {
    include_str!("js/js_find_tweet_button.js")
}

/// Returns JS to verify a quote tweet was posted (composer cleared).
/// Returns `{posted, reason}` object.
#[must_use]
pub fn js_verify_quote_posted() -> &'static str {
    include_str!("js/js_verify_quote_posted.js")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_css_selector_constants_do_not_contain_literal_backslashes() {
        let selectors = [
            HOME_LOGO_SELECTOR,
            TWEET_LINK_SELECTOR,
            TWEET_DETAIL_SELECTOR,
            TWEET_DETAIL_FALLBACK1,
            TWEET_DETAIL_FALLBACK2,
            TWEET_DETAIL_FALLBACK3,
            TWEET_DETAIL_FALLBACK4,
            RETWEET_BUTTON_SELECTOR,
            RETWEET_CONFIRM_SELECTOR,
            LIKE_BUTTON_SELECTOR,
            FOLLOW_BUTTON_SELECTOR,
            BOOKMARK_BUTTON_SELECTOR,
            TWEET_TEXTAREA_SELECTOR,
            ROLE_TEXTBOX_SELECTOR,
            BUTTON_ROLE_BUTTON_SELECTOR,
            SUBSCRIBE_BUTTON_SELECTOR,
            ARTICLE_TWEET_SELECTOR,
            TWEET_TEXT_SELECTOR,
            REPLY_BUTTON_SELECTOR,
            TWEET_REPLY_SELECTOR,
            LIKE_TESTID_SELECTOR,
            RETWEET_TESTID_SELECTOR,
            REPLY_TESTID_SELECTOR,
            DIR_AUTO_SPAN_SELECTOR,
            TWEET_BUTTON_SELECTOR,
            RETWEET_CONFIRM_BUTTON_SELECTOR,
            TWEET_BUTTON_INLINE_SELECTOR,
        ];

        for selector in selectors {
            assert!(
                !selector.contains('\\'),
                "CSS selector should not contain literal backslashes: {selector}"
            );
        }
    }

    #[test]
    fn test_selector_feed_visible_returns_js() {
        let js = selector_feed_visible();
        assert!(js.contains("querySelector"));
        assert!(js.contains("data-testid"));
    }

    #[test]
    fn test_selector_all_tweets_returns_js() {
        let js = selector_all_tweets();
        assert!(js.contains("querySelectorAll"));
        assert!(js.contains("article"));
    }

    #[test]
    fn test_selector_follow_button_returns_js() {
        let js = selector_follow_button();
        assert!(js.contains("querySelector"));
        assert!(js.contains("aria-label"));
    }

    #[test]
    fn test_selector_engagement_buttons_returns_js() {
        let js = selector_engagement_buttons();
        assert!(js.contains("like"));
        assert!(js.contains("retweet"));
        assert!(js.contains("reply"));
    }

    #[test]
    fn test_selector_tweet_user_avatar_returns_js() {
        let js = selector_tweet_user_avatar();
        assert!(js.contains("Tweet-User-Avatar"));
        assert!(js.contains("profile_images"));
    }

    #[test]
    fn test_selector_login_flow_returns_js() {
        let js = selector_login_flow();
        assert!(js.contains("session"));
        assert!(js.contains("Sign in"));
    }

    #[test]
    fn test_selector_element_center_format() {
        let js = selector_element_center("#test-selector");
        assert!(js.contains("querySelector"));
        assert!(js.contains("getBoundingClientRect"));
        assert!(js.contains("x:"));
        assert!(js.contains("y:"));
    }

    #[test]
    fn test_selector_element_center_escapes_quotes() {
        let js = selector_element_center("#test\"quote");
        assert!(js.contains("\\\""));
    }

    #[test]
    fn test_selector_popup_overlay_returns_js() {
        let js = selector_popup_overlay();
        assert!(js.contains("dialog"));
        assert!(js.contains("aria-modal"));
    }

    #[test]
    fn test_selector_follow_confirm_modal_returns_js() {
        let js = selector_follow_confirm_modal();
        assert!(js.contains("dialog"));
        assert!(js.contains("follow"));
    }

    #[test]
    fn test_selector_close_button_returns_js() {
        let js = selector_close_button();
        assert!(js.contains("Close"));
        assert!(js.contains("aria-label"));
    }

    #[test]
    fn test_selector_following_indicator_returns_js() {
        let js = selector_following_indicator();
        assert!(js.contains("following"));
        assert!(js.contains("unfollow"));
    }

    #[test]
    fn test_js_get_current_url_returns_js() {
        let js = js_get_current_url();
        assert!(js.contains("window.location.href"));
    }

    #[test]
    fn test_js_extract_username_from_url_returns_js() {
        let js = js_extract_username_from_url();
        assert!(js.contains("window.location.pathname"));
        assert!(js.contains("split"));
    }

    #[test]
    fn test_selector_health_check_returns_js() {
        let js = selector_health_check();
        assert!(js.contains("feed_visible"));
        assert!(js.contains("tweets_found"));
        assert!(js.contains("engagement_buttons"));
    }
}
