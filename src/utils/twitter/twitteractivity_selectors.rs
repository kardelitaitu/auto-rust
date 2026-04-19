//! Centralized JavaScript selector snippets for Twitter/X DOM queries.
//!
//! All selectors are designed for Twitter/X's dynamic class structure.
//! Functions return JS code as &'static str that can be passed to `ctx.page().evaluate()`.

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
            if (document.querySelector('h1:contains("Sign in to X")') ||
                document.querySelector('h1:contains("Log in to X")')) return 'login';
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
