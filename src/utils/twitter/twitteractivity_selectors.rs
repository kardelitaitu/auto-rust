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

// --- CSS Selector Constants ---

/// Home logo selector (X logo)
pub const HOME_LOGO_SELECTOR: &str = r#"a[aria-label=\"X\"]"#;

/// Tweet link selector (links to individual tweets)
pub const TWEET_LINK_SELECTOR: &str = r#"a[href*=\"/status/\"]"#;

/// Tweet detail/dialog selector
pub const TWEET_DETAIL_SELECTOR: &str = r#"div[role=\"dialog\"]"#;

/// Tweet detail fallback selectors
pub const TWEET_DETAIL_FALLBACK1: &str = r#"div[data-testid=\"tweetDetail\"]"#;
pub const TWEET_DETAIL_FALLBACK2: &str = r#"div[data-testid=\"tweetThread\"]"#;
pub const TWEET_DETAIL_FALLBACK3: &str = r#"[aria-label=\"Timeline: Thread\"]"#;
pub const TWEET_DETAIL_FALLBACK4: &str = r#"article[data-testid=\"tweet\"]"#;

/// Retweet button selector
pub const RETWEET_BUTTON_SELECTOR: &str = r#"button[data-testid=\"retweet\"]"#;

/// Retweet confirm button selector
pub const RETWEET_CONFIRM_SELECTOR: &str = r#"div[data-testid=\"retweetConfirm\"]"#;

/// Like button selector
pub const LIKE_BUTTON_SELECTOR: &str = r#"button[data-testid=\"like\"]"#;

/// Follow button selector (ending with -follow)
pub const FOLLOW_BUTTON_SELECTOR: &str = r#"button[data-testid$=\"-follow\"]"#;

/// Bookmark button selector
pub const BOOKMARK_BUTTON_SELECTOR: &str = r#"button[data-testid=\"bookmark\"]"#;

// --- Additional Selector Constants (for inline selector cleanup) ---

/// Tweet textarea selector (for reply composition)
pub const TWEET_TEXTAREA_SELECTOR: &str = r#"[data-testid=\"tweetTextarea_0\"]"#;

/// Role textbox selector
pub const ROLE_TEXTBOX_SELECTOR: &str = r#"[role=\"textbox\"]"#;

/// Button with role=button selector
pub const BUTTON_ROLE_BUTTON_SELECTOR: &str = r#"button, [role=\"button\"]"#;

/// Subscribe button selector (for follow checks)
pub const SUBSCRIBE_BUTTON_SELECTOR: &str = r#"button[data-testid*=\"-subscribe\"]"#;

/// Article tweet selector (for feed scanning)
pub const ARTICLE_TWEET_SELECTOR: &str = r#"article[data-testid=\"tweet\"]"#;

/// Tweet text selector
pub const TWEET_TEXT_SELECTOR: &str = r#"[data-testid=\"tweetText\"]"#;

/// Reply button selector
pub const REPLY_BUTTON_SELECTOR: &str = r#"button[data-testid="reply"]"#;

/// Tweet reply selector (for extracting replies)
pub const TWEET_REPLY_SELECTOR: &str = r#"[data-testid=\"tweetReply\"]"#;

// --- Attribute-only selectors (for use with element.querySelector) ---

/// Like data-testid attribute selector (element-agnostic)
pub const LIKE_TESTID_SELECTOR: &str = r#"[data-testid="like"]"#;

/// Retweet data-testid attribute selector (element-agnostic)
pub const RETWEET_TESTID_SELECTOR: &str = r#"[data-testid="retweet"]"#;

/// Reply data-testid attribute selector (element-agnostic)
pub const REPLY_TESTID_SELECTOR: &str = r#"[data-testid="reply"]"#;

/// Dir auto span selector (for reply author extraction)
pub const DIR_AUTO_SPAN_SELECTOR: &str = r#"[dir=\"auto\"] span:first-child"#;

/// Tweet button selector (generic button search)
pub const TWEET_BUTTON_SELECTOR: &str = r#"button[data-testid], a[data-testid]"#;

/// Retweet confirm button selector (in modal/dialog)
pub const RETWEET_CONFIRM_BUTTON_SELECTOR: &str = r#"button[data-testid=\"retweetConfirm\"]"#;

/// Tweet button inline selector (reply submit button in composer)
pub const TWEET_BUTTON_INLINE_SELECTOR: &str = r#"button[data-testid=\"tweetButtonInline\"]"#;

/// Returns JS to find and return center coordinates of the retweet confirm button.
/// Returns `{x, y}` or `null` if not found.
pub fn js_confirm_retweet_click() -> &'static str {
    r#"
        (function() {
            var btn = document.querySelector('button[data-testid="retweetConfirm"]');
            if (!btn) return null;
            var rect = btn.getBoundingClientRect();
            return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
        })()
    "#
}

/// Returns JS to find and focus the reply textarea.
/// Returns `{found: true}` if found and focused, `{found: false}` otherwise.
pub fn js_find_reply_textarea() -> &'static str {
    r#"
        (function() {
            var textboxes = document.querySelectorAll('[data-testid="tweetTextarea_0"][role="textbox"], [data-testid="tweetTextarea_0"]');
            for (var i = 0; i < textboxes.length; i++) {
                var ta = textboxes[i];
                var rect = ta.getBoundingClientRect();
                if (rect.width <= 0 || rect.height <= 0) continue;
                ta.focus();
                ta.click();
                return { found: true };
            }
            return { found: false };
        })()
    "#
}

/// Returns JS to find the reply submit button and return its center coordinates.
/// Returns `{x, y}` or `null` if not found.
pub fn js_find_reply_submit_button() -> &'static str {
    r#"
        (function() {
            var buttons = document.querySelectorAll('button[data-testid="tweetButtonInline"]');
            for (var i = 0; i < buttons.length; i++) {
                var btn = buttons[i];
                var rect = btn.getBoundingClientRect();
                if (rect.width <= 0 || rect.height <= 0) continue;
                if (btn.disabled || btn.getAttribute('aria-disabled') === 'true') continue;
                var text = (btn.textContent || btn.innerText || '').trim().toLowerCase();
                if (text !== 'reply') continue;
                return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
            }
            return null;
        })()
    "#
}

/// Returns JS to generate center coordinates for a button within the root tweet.
/// Takes a CSS selector string and searches within the first visible tweet article.
/// Returns `{x, y}` or `null` if not found.
pub fn js_root_tweet_button_center(selector: &str) -> String {
    format!(
        r#"
        (function() {{
            function visible(el) {{
                if (!el) return false;
                var rect = el.getBoundingClientRect();
                return rect.width > 0 && rect.height > 0;
            }}
            function center(el) {{
                var rect = el.getBoundingClientRect();
                return {{ x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 }};
            }}

            var modal = document.querySelector('div[role="dialog"]');
            var articles = [];
            if (modal && visible(modal)) {{
                articles = Array.prototype.slice.call(
                    modal.querySelectorAll('article[data-testid="tweet"]')
                ).filter(visible);
            }}

            if (articles.length === 0) {{
                articles = Array.prototype.slice.call(
                    document.querySelectorAll('article[data-testid="tweet"]')
                ).filter(visible);
            }}

            var statusMatch = window.location.pathname.match(/\/status\/(\d+)/);
            var targetStatusId = statusMatch ? statusMatch[1] : null;
            var targetArticle = null;
            if (targetStatusId) {{
                for (var i = 0; i < articles.length; i++) {{
                    if (articles[i].querySelector('a[href*="/status/' + targetStatusId + '"]')) {{
                        targetArticle = articles[i];
                        break;
                    }}
                }}
            }}
            var scopes = articles.length > 0
                ? [targetArticle || articles[0]]
                : [document.querySelector('main'), document.body].filter(Boolean);

            for (var i = 0; i < scopes.length; i++) {{
                var button = scopes[i].querySelector("{}");
                if (visible(button)) return center(button);
            }}
            return null;
        }})()
        "#,
        selector.replace('"', "\\\"")
    )
}

/// Returns JS to identify reply candidates within a thread dive.
/// Skips the root tweet and returns engageable replies with text and button positions.
pub fn js_identify_thread_replies() -> &'static str {
    r#"
        (function() {
            var replies = [];
            var articles = document.querySelectorAll('article[data-testid="tweet"]');
            
            // In a thread view, the first tweet is usually the root tweet.
            // We want to skip it and look at the replies.
            for (var i = 1; i < articles.length; i++) {
                var el = articles[i];
                var rect = el.getBoundingClientRect();
                
                // Only consider visible replies
                if (rect.height > 0 && rect.width > 0 && rect.top < window.innerHeight && rect.bottom > 0) {
                    var tweetTextEl = el.querySelector('[data-testid="tweetText"]');
                    var tweetText = tweetTextEl ? tweetTextEl.textContent.trim() : '';
                    
                    // Find Like button
                    var likeBtn = el.querySelector('[data-testid="like"]');
                    var likePos = null;
                    if (likeBtn) {
                        var likeRect = likeBtn.getBoundingClientRect();
                        if (likeRect.width > 0 && likeRect.height > 0) {
                            likePos = { x: likeRect.x + likeRect.width/2, y: likeRect.y + likeRect.height/2 };
                        }
                    }
                    
                    // Extract author
                    var authorEl = el.querySelector('[dir="ltr"] span');
                    var author = authorEl ? authorEl.textContent.trim() : 'unknown';
                    
                    // Extract tweet ID
                    var statusLink = el.querySelector('a[href*="/status/"]');
                    var tweetId = 'unknown';
                    if (statusLink) {
                        var href = statusLink.getAttribute('href');
                        var parts = href.split('/');
                        tweetId = parts[parts.length - 1].split('?')[0];
                    }

                    if (likePos && tweetText.length > 5) {
                        replies.push({
                            id: tweetId,
                            text: tweetText,
                            author: author,
                            like_pos: likePos,
                            y_top: rect.top
                        });
                    }
                }
                
                if (replies.length >= 8) break; // Scan a reasonable number of visible replies
            }
            return replies;
        })()
    "#
}

/// Returns JS to identify engagement candidates in the current feed.
/// Returns an array of tweet objects with id, text, button positions, and replies.
pub fn js_identify_engagement_candidates() -> &'static str {
    r#"
        (function() {
            var tweets = [];
            var elements = document.querySelectorAll('article[data-testid="tweet"]');
            for (var i = 0; i < elements.length; i++) {
                var el = elements[i];
                var rect = el.getBoundingClientRect();
                if (rect.height > 0 && rect.width > 0) {
                    // Extract tweet text content
                    var tweetTextEl = el.querySelector('[data-testid="tweetText"]');
                    var tweetText = tweetTextEl ? tweetTextEl.textContent.trim() : '';

                    // Find engagement buttons within this tweet element
                    var likeBtn = el.querySelector('[data-testid="like"]');
                    var retweetBtn = el.querySelector('[data-testid="retweet"]');
                    var replyBtn = el.querySelector('[data-testid="reply"]');

                    var buttonPositions = {};
                    if (likeBtn) {
                        var likeRect = likeBtn.getBoundingClientRect();
                        if (likeRect.width > 0 && likeRect.height > 0) {
                            buttonPositions.like = { x: likeRect.x + likeRect.width/2, y: likeRect.y + likeRect.height/2 };
                        }
                    }
                    if (retweetBtn) {
                        var retweetRect = retweetBtn.getBoundingClientRect();
                        if (retweetRect.width > 0 && retweetRect.height > 0) {
                            buttonPositions.retweet = { x: retweetRect.x + retweetRect.width/2, y: retweetRect.y + retweetRect.height/2 };
                        }
                    }
                    if (replyBtn) {
                        var replyRect = replyBtn.getBoundingClientRect();
                        if (replyRect.width > 0 && replyRect.height > 0) {
                            buttonPositions.reply = { x: replyRect.x + replyRect.width/2, y: replyRect.y + replyRect.height/2 };
                        }
                    }

                    // Extract the status URL from the time element for reliable diving
                    var links = el.querySelectorAll('a[href*="/status/"]');
                    var statusUrl = null;
                    for (var j = 0; j < links.length; j++) {
                        var href = links[j].getAttribute('href');
                        var parts = href.split('/').filter(function(p) { return p.length > 0; });
                        if (parts.length === 3 && parts[1] === 'status' && !isNaN(parts[2])) {
                            statusUrl = href;
                            break;
                        }
                    }

                    // Prefer stable tweet identity from permalink
                    var statusId = null;
                    if (statusUrl) {
                        var statusParts = statusUrl.split('/').filter(function(p) { return p.length > 0; });
                        statusId = statusParts[statusParts.length - 1].split(/[?#]/)[0];
                    }
                    var tweetId = el.dataset.tweetId ||
                                  el.getAttribute('data-item-id') ||
                                  el.getAttribute('data-tweet-id') ||
                                  statusId ||
                                  'tweet_' + Math.floor(rect.x) + '_' + Math.floor(rect.y);

                    var tweetObj = {
                        id: tweetId,
                        status_url: statusUrl,
                        index: i,
                        text: tweetText,
                        x: rect.x + rect.width/2,
                        y: rect.y + rect.height/2,
                        height: rect.height,
                        width: rect.width,
                        buttons: buttonPositions
                    };

                    // Extract reply information for smart decision
                    var replies = [];
                    var replyElements = el.querySelectorAll('[data-testid="tweetReply"]');
                    for (var j = 0; j < Math.min(replyElements.length, 3); j++) {
                        var replyEl = replyElements[j];
                        var authorEl = replyEl.querySelector('[dir="auto"] span:first-child');
                        var textEl = replyEl.querySelector('[data-testid="tweetText"]');
                        if (authorEl && textEl) {
                            replies.push({
                                author: authorEl.textContent.trim(),
                                text: textEl.textContent.trim()
                            });
                        }
                    }

                    if (replies.length > 0) {
                        tweetObj.replies = replies;
                    }

                    tweets.push(tweetObj);
                }
            }
            return tweets;
        })()
    "#
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
