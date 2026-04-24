//! Centralized JavaScript selector snippets for Twitter/X DOM queries.
//!
//! All selectors are designed for Twitter/X's dynamic class structure.
//! Functions return JS code as &'static str that can be passed to `api.page().evaluate()`.

/// Returns the best selector to detect that the main timeline/feed is visible.
/// Used to verify successful navigation to the home timeline.
pub fn selector_feed_visible() -> &'static str {
    r#"
        (function() {
            // Prefer data-testid attributes (most stable)
            if (document.querySelector('[data-testid="primaryColumn"]')) return true;
            if (document.querySelector('main[role="main"]')) return true;
            // Fallback to article detection
            if (document.querySelector('article[data-testid="tweet"]')) return true;
            if (document.querySelector('article')) return true;
            return false;
        })()
    "#
}

/// Returns JS to find the center coordinates of the first element matching a selector.
/// Returns `{x, y}` or `null` if not found.
pub fn selector_element_center(selector: &str) -> String {
    format!(
        r#"
        (function() {{
            var el = document.querySelector("{}");
            if (!el) return null;
            var rect = el.getBoundingClientRect();
            return {{ x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 }};
        }})()
        "#,
        selector.replace('"', "\\\"")
    )
}

/// Returns JS to query all tweet/article elements currently in the DOM.
/// Returns an array of objects with tweetId (from data-item-id or similar) and bounding rect.
pub fn selector_all_tweets() -> &'static str {
    r#"
        (function() {
            var tweets = [];
            var articles = document.querySelectorAll('article[data-testid="tweet"]');
            if (articles.length === 0) {
                articles = document.querySelectorAll('article');
            }
            for (var i = 0; i < articles.length; i++) {
                var el = articles[i];
                var rect = el.getBoundingClientRect();
                var tweetId = el.getAttribute('data-item-id') ||
                              el.getAttribute('data-tweet-id') ||
                              el.getAttribute('data-testid')?.includes('tweet-') ? el.getAttribute('data-testid').replace('tweet-', '') : null;
                tweets.push({
                    id: tweetId,
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height
                });
            }
            return tweets;
        })()
    "#
}

/// Returns JS to find visible follow buttons within a tweet/article element.
pub fn selector_follow_button() -> &'static str {
    r#"
        (function() {
            var scope =
                document.querySelector('main header') ||
                document.querySelector('main [data-testid="UserProfileHeader_Items"]') ||
                document.querySelector('main') ||
                document.body;

            var buttons = scope.querySelectorAll('button, [role="button"]');
            for (var i = 0; i < buttons.length; i++) {
                var btn = buttons[i];
                var text = (btn.textContent || btn.innerText || '').trim();
                var label = (btn.getAttribute('aria-label') || '').trim();
                var dataTestId = btn.getAttribute('data-testid') || '';
                if (label.toLowerCase().includes('follow @') ||
                    label.toLowerCase() === 'follow' ||
                    text.toLowerCase() === 'follow' ||
                    dataTestId.toLowerCase().includes('follow')) {
                    var rect = btn.getBoundingClientRect();
                    if (rect.width > 0 && rect.height > 0) {
                        return {
                            x: rect.x + rect.width / 2,
                            y: rect.y + rect.height / 2,
                            text: text,
                            label: label
                        };
                    }
                }
            }
            return null;
        })()
    "#
}

/// Returns JS to find like/retweet/reply buttons for a given tweet element.
pub fn selector_engagement_buttons() -> &'static str {
    r#"
        (function() {
            var result = {
                like: null,
                retweet: null,
                reply: null
            };
            var buttons = document.querySelectorAll('button[data-testid], a[data-testid]');
            for (var i = 0; i < buttons.length; i++) {
                var el = buttons[i];
                var testId = (el.getAttribute('data-testid') || '').toLowerCase();
                var rect = el.getBoundingClientRect();
                if (rect.width === 0 || rect.height === 0) continue;
                var pos = { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
                if (testId.includes('like') && !testId.includes('unlike')) {
                    result.like = pos;
                } else if (testId.includes('retweet') && !testId.includes('unretweet')) {
                    result.retweet = pos;
                } else if (testId.includes('reply') || testId.includes('comment')) {
                    result.reply = pos;
                }
            }
            return result;
        })()
    "#
}

/// Returns JS to check if current page shows a login/onboarding flow.
pub fn selector_login_flow() -> &'static str {
    r#"
        (function() {
            // Login forms
            if (document.querySelector('form[action*="/session"]')) return 'login';
            if (document.querySelector('input[name="session[username_or_email]"]')) return 'login';
            // Phone/email input
            if (document.querySelector('input[type="email"][name*="identifier"]')) return 'login';
            // Onboarding
            if (document.querySelector('form[action*="/i/flow/login"]')) return 'onboarding';
            if (document.querySelector('input[autocomplete="username"]')) return 'onboarding';
            // "Sign in to X" heading/signals
            var h1Elements = document.querySelectorAll('h1');
            for (var i = 0; i < h1Elements.length; i++) {
                var text = (h1Elements[i].textContent || '').toLowerCase();
                if (text.includes('sign in to x') || text.includes('log in to x')) return 'login';
            }
            return null;
        })()
    "#
}

/// Returns JS to detect if a popup/modal is present (e.g., "Follow on Twitter" prompt, cookies, etc.)
pub fn selector_popup_overlay() -> &'static str {
    r#"
        (function() {
            var selectors = [
                'div[role="dialog"]',
                'div[aria-modal="true"]',
                'div[data-testid="sidebarColumn"]',
                'div[data-testid="app-bar-ads"]',
                'div[data-testid="placementTracking"]',
                'div[aria-label=" cookie"]',
                'div[aria-label="Privacy"]'
            ];
            for (var i = 0; i < selectors.length; i++) {
                var el = document.querySelector(selectors[i]);
                if (el) {
                    var rect = el.getBoundingClientRect();
                    if (rect.width > 100 && rect.height > 100) {
                        return el;
                    }
                }
            }
            return null;
        })()
    "#
}

/// Returns JS to check if a "Follow on X" (external site) confirmation modal is open.
pub fn selector_follow_confirm_modal() -> &'static str {
    r#"
        (function() {
            var dialog = document.querySelector('div[role="dialog"]');
            if (!dialog) return null;
            var text = (dialog.textContent || '').toLowerCase();
            if (text.includes('follow') || text.includes('confirm')) {
                return dialog;
            }
            return null;
        })()
    "#
}

/// Returns JS to find a close button (X) for a modal/dialog overlay.
pub fn selector_close_button() -> &'static str {
    r#"
        (function() {
            var closeButtons = document.querySelectorAll('button[aria-label*="Close"], button[data-testid*="close"], div[role="button"][aria-label*="Close"]');
            for (var i = 0; i < closeButtons.length; i++) {
                var btn = closeButtons[i];
                var rect = btn.getBoundingClientRect();
                if (rect.width > 0 && rect.height > 0) {
                    return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
                }
            }
            return null;
        })()
    "#
}

/// Returns JS to find the "Following" state indicator on a user profile or tweet.
pub fn selector_following_indicator() -> &'static str {
    r#"
        (function() {
            var buttons = document.querySelectorAll('button');
            for (var i = 0; i < buttons.length; i++) {
                var btn = buttons[i];
                var text = (btn.textContent || btn.innerText || '').trim().toLowerCase();
                var label = (btn.getAttribute('aria-label') || '').toLowerCase();
                var dataTestId = (btn.getAttribute('data-testid') || '').toLowerCase();
                if (text === 'following' ||
                    label.includes('following @') ||
                    dataTestId.includes('unfollow')) {
                    return true;
                }
            }
            return false;
        })()
    "#
}

/// Returns JS to get current URL (for verifying navigation).
pub fn js_get_current_url() -> &'static str {
    r#"window.location.href"#
}

/// Returns JS to extract username from a profile page (if navigated to /username).
pub fn js_extract_username_from_url() -> &'static str {
    r#"
        (function() {
            var path = window.location.pathname;
            if (path.startsWith('/')) path = path.substring(1);
            if (path.includes('/')) path = path.split('/')[0];
            return path || null;
        })()
    "#
}

/// Returns JS to find and click the user avatar in a tweet to navigate to their profile.
/// Returns coordinates or null if not found.
pub fn selector_tweet_user_avatar() -> &'static str {
    r#"
        (function() {
            // Try multiple selector strategies for tweet user avatar
            var avatar = document.querySelector('[data-testid="Tweet-User-Avatar"]') ||
                        document.querySelector('article img[src*="/profile_images"]') ||
                        document.querySelector('[role="article"] img');
            if (avatar) {
                var rect = avatar.getBoundingClientRect();
                return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
            }
            return null;
        })()
    "#
}

/// Returns JS to perform a quick health check on critical selectors.
/// Returns an object with health status of each selector type.
pub fn selector_health_check() -> &'static str {
    r#"
        (function() {
            var results = {
                feed_visible: false,
                tweets_found: false,
                engagement_buttons: false,
                follow_button: false
            };

            // Check feed visibility
            if (document.querySelector('[data-testid="primaryColumn"]') ||
                document.querySelector('main[role="main"]') ||
                document.querySelector('article[data-testid="tweet"]')) {
                results.feed_visible = true;
            }

            // Check for tweets
            var articles = document.querySelectorAll('article[data-testid="tweet"]');
            if (articles.length > 0 || document.querySelectorAll('article').length > 0) {
                results.tweets_found = true;
            }

            // Check for engagement buttons
            var buttons = document.querySelectorAll('button[data-testid], a[data-testid]');
            for (var i = 0; i < buttons.length; i++) {
                var testId = (buttons[i].getAttribute('data-testid') || '').toLowerCase();
                if (testId.includes('like') || testId.includes('retweet') || testId.includes('reply')) {
                    results.engagement_buttons = true;
                    break;
                }
            }

            // Check for follow button
            var allButtons = document.querySelectorAll('[role="button"]');
            for (var i = 0; i < allButtons.length; i++) {
                var label = (allButtons[i].getAttribute('aria-label') || '').toLowerCase();
                if (label.includes('follow')) {
                    results.follow_button = true;
                    break;
                }
            }

            return results;
        })()
    "#
}

#[cfg(test)]
mod tests {
    use super::*;

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
