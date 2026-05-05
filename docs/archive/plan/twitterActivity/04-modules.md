# Twitter Activity — Helper Modules Specification

This document specifies all helper modules that support the `twitteractivity` task. All helper modules reside in `src/utils/twitter/` as a cohesive utility layer. The main task (`task/twitteractivity.rs`) imports and uses these helpers via `crate::utils::twitter::...`.

---

## `src/utils/twitter/twitteractivity_navigation.rs`

**Responsibility**: Entry URL selection. Navigation is performed via `TaskContext::navigate()`.

```rust
use crate::prelude::TaskContext;
use crate::config::TwitterConfig;
use anyhow::Result;

/// Pre-defined entry point URLs with weights for weighted random selection.
/// Categories (weight total = 100):
/// - Home (59%):       https://x.com/
/// - Explore (32%):    [for-you (8), trending (8), tabs/gaming/tabs (8), explore (8)]
/// - Connect (4%):     [connect_people?show_topics=false (2), connect_people?is_creator_only=true (2)]
/// - Supplementary (5%): [notifications (1), bookmarks (1), i/chat (1), news/sports/entertainment verticals (2)]
const ENTRY_POINTS: &[( &str, u32 )] = &[
    ("home", 59),
    ("explore", 32),
    ("connect", 4),
    ("supplementary", 5),
];

/// Returns a concrete URL based on weighted category selection, then sub-URL random.
pub async fn select_entry_point(config: &TwitterConfig) -> Result<String> {
    let category = weighted_pick(ENTRY_POINTS)?;
    match category {
        "home" => Ok("https://x.com/".to_string()),
        "explore" => Ok(pick_explore_url()),
        "connect" => Ok(pick_connect_url()),
        "supplementary" => Ok(pick_supplementary_url()),
        _ => Err(anyhow!("unknown entry category: {category}")),
    }
}
```

**Functions to implement**:
- `weighted_pick(items: &[(T, u32)]) -> Result<T>` — cumulative weight random
- `pick_explore_url() -> String` — random from `["/explore", "/explore/tabs/for-you", "/explore/tabs/trending", "/i/jf/global-trending/home"]`
- `pick_connect_url() -> String` — random from connect people pages
- `pick_supplementary_url() -> String` — random from news/sports/entertainment topic pages

---

## `src/utils/twitter/twitteractivity_feed.rs`

**Responsibility**: Scan the feed and locate candidate tweets using selector families.

```rust
use crate::prelude::TaskContext;
use anyhow::Result;
use serde_json::Value;
use crate::utils::twitter::twitteractivity_selectors::*;

/// Representation of a tweet found in the feed.
#[derive(Debug, Clone)]
pub struct TweetMetadata {
    pub id: String,
    pub author_handle: String,
    pub author_name: String,
    pub text_preview: String,
    pub has_media: bool,
    pub selector_used: String,
}

/// Scans the current feed viewport and returns a random tweet candidate.
pub async fn find_random_tweet(ctx: &TaskContext) -> Result<Option<TweetMetadata>> {
    let selectors = [
        TWEET_ARTICLE,
        TWEET_CELL_INNER,
        TWEET_ARTICLE_FALLBACK,
    ];

    // Use JS evaluation to collect tweet metadata in one shot
    for &selector in &selectors {
        let js = format!(r#"
            (() => {{
                const els = document.querySelectorAll({});
                if (els.length === 0) return [];
                return Array.from(els).slice(0, 20).map(el => {{
                    const id = el.getAttribute("data-tweet-id") || "";
                    const authorHandle = el.getAttribute("data-author-handle") || "";
                    const authorName = el.querySelector('[data-testid="User-Names"]')?.innerText?.trim() || "";
                    const text = el.querySelector('[data-testid="tweetText"]')?.innerText?.trim() || "";
                    const hasMedia = el.querySelector('img, video') !== null;
                    return {{ id, author_handle: authorHandle, author_name: authorName, text_preview: text, has_media: hasMedia }};
                }});
            }})()
        "#, serde_json::to_string(selector)?);

        let result = ctx.page().evaluate(js).await?;
        let value = result.value().ok_or_else(|| anyhow::anyhow!("No value returned from feed scan"))?;
        let tweets: Vec<TweetMetadata> = serde_json::from_value(value.clone())?;
        if !tweets.is_empty() {
            let idx = rand::thread_rng().gen_range(0..tweets.len());
            let mut chosen = tweets[idx].clone();
            chosen.selector_used = selector.to_string();
            return Ok(Some(chosen));
        }
    }

    Ok(None)
}
```

---

## `src/utils/twitter/twitteractivity_dive.rs`

**Responsibility**: Click into tweet, expand content, extract full context.

```rust
use crate::prelude::TaskContext;
use crate::utils::page_size::get_element_center;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::time::Instant;

/// Full tweet context after expansion.
#[derive(Debug, Clone)]
pub struct TweetContext {
    pub tweet_id: String,
    pub author_handle: String,
    pub full_text: String,
    pub reply_count: u32,
    pub retweet_count: u32,
    pub like_count: u32,
    pub quote_count: u32,
    pub timestamp: Option<String>,
    pub is_from_followed_account: bool,
}

pub async fn load_tweet_context(
    ctx: &TaskContext,
    tweet: &super::TweetMetadata
) -> Result<TweetContext> {
    // Scroll tweet into view using smooth JS
    let selector_js = serde_json::to_string(&tweet.selector_used)?;
    let js = format!(r#"
        (() => {{
            const el = document.querySelector({});
            if (!el) return null;
            el.scrollIntoView({{behavior: 'smooth', block: 'center'}});
            return true;
        }})()
    "#, selector_js);
    ctx.page().evaluate(js).await?;
    ctx.pause(500, 30).await;

    // Click at the element center
    click_tweet(ctx, tweet).await?;

    // Wait for expansion
    ctx.pause(1000, 50).await;

    // Scrape full context via JS evaluation
    let js = r#"
        (() => {
            const tweetEl = document.querySelector('article[data-testid="tweet"]');
            if (!tweetEl) return null;
            const text = tweetEl.querySelector('[data-testid="tweetText"]')?.innerText?.trim() || "";
            return {
                tweet_id: tweetEl.getAttribute("data-tweet-id") || "",
                author_handle: tweetEl.getAttribute("data-author-handle") || "",
                full_text: text,
                reply_count: 0,
                retweet_count: 0,
                like_count: 0,
                quote_count: 0,
                timestamp: null,
                is_from_followed_account: false
            };
        })()
    "#;
    let result = ctx.page().evaluate(js).await?;
    let value = result.value().ok_or_else(|| anyhow::anyhow!("Failed to extract tweet context"))?;
    let ctx_val: TweetContext = serde_json::from_value(value)?;
    Ok(ctx_val)
}

/// Click a tweet to expand it.
pub async fn click_tweet(ctx: &TaskContext, tweet: &super::TweetMetadata) -> Result<()> {
    let selector = &tweet.selector_used;
    let (x, y) = get_element_center(ctx.page(), selector).await?;
    ctx.move_mouse_to(x, y).await?;
    ctx.pause(200, 40).await;  // hover pause
    ctx.click(x, y).await?;
    Ok(())
}
```

---

## `src/utils/twitter/twitteractivity_interact.rs`

**Responsibility**: UI automation for engagement actions.

```rust
use crate::prelude::TaskContext;
use crate::utils::page_size::get_element_center;
use crate::utils::twitter::twitteractivity_selectors::*;

/// Like a tweet — clicks like button and verifies state flips to "liked".
pub async fn like_tweet(ctx: &TaskContext, tweet: &super::TweetMetadata) -> Result<()> {
    // Use JS to check if already liked
    let already_liked = ctx.page().evaluate(r#"
        (() => document.querySelector('[data-testid="unlike"]') !== null)
    "#).await?.value().and_then(|v| v.as_bool()).unwrap_or(false);

    if already_liked {
        info!("[twitter] Tweet already liked, skipping");
        return Ok(());
    }

    let (x, y) = get_element_center(ctx.page(), BTN_LIKE).await?;
    ctx.move_mouse_to(x, y).await?;
    ctx.pause(200, 40).await;
    ctx.click(x, y).await?;

    // Verify state change: wait up to 2s for unlike to appear
    let start = std::time::Instant::now();
    loop {
        let found = ctx.page().evaluate(r#"document.querySelector('[data-testid="unlike"]') !== null"#)
            .await?.value().and_then(|v| v.as_bool()).unwrap_or(false);
        if found || start.elapsed() > std::time::Duration::from_secs(2) {
            break;
        }
        ctx.pause(100, 20).await;
    }

    Ok(())
}

/// Retweet (native RT, no comment).
pub async fn retweet_tweet(ctx: &TaskContext, tweet: &super::TweetMetadata) -> Result<()> {
    let (x, y) = get_element_center(ctx.page(), BTN_RETWEET).await?;
    ctx.move_mouse_to(x, y).await?;
    ctx.click(x, y).await?;
    ctx.pause(300, 40).await;

    // Modal appears — click "Retweet" confirm button
    let (cx, cy) = get_element_center(ctx.page(), BTN_RETWEET_CONFIRM).await?;
    ctx.move_mouse_to(cx, cy).await?;
    ctx.click(cx, cy).await?;

    // Verify modal closes (best-effort)
    ctx.pause(1000, 50).await;

    Ok(())
}

/// Follow user.
pub async fn follow_user(ctx: &TaskContext, tweet: &super::TweetMetadata) -> Result<()> {
    // Check already following?
    let already_following = ctx.page().evaluate(r#"
        (() => document.querySelector('[data-testid="following"]') !== null)
    "#).await?.value().and_then(|v| v.as_bool()).unwrap_or(false);

    if already_following {
        info!("[twitter] Already following @{}", tweet.author_handle);
        return Ok(());
    }

    let (x, y) = get_element_center(ctx.page(), BTN_FOLLOW).await?;
    ctx.move_mouse_to(x, y).await?;
    ctx.click(x, y).await?;

    // Verify state change (best-effort)
    ctx.pause(800, 50).await;

    Ok(())
}

/// Bookmark tweet (disabled V1, stub for future).
pub async fn bookmark_tweet(_ctx: &TaskContext, _tweet: &super::TweetMetadata) -> Result<()> {
    bail!("Bookmark action disabled in V1");
}
```

---

## `src/utils/twitter/twitteractivity_popup.rs`

**Responsibility**: Dismiss known modal/popup types after every navigation.

```rust
use crate::prelude::TaskContext;
use crate::utils::page_size::get_element_center;
use crate::utils::twitter::twitteractivity_selectors::*;

pub async fn close_all_modals(ctx: &TaskContext) -> Result<()> {
    let dismiss_selectors = [
        MODAL_CLOSE_X,
        MODAL_CLOSE_BUTTON,
        MODAL_SIGNUP_CLOSE,
        MODAL_NOTIFICATION_DISMISS,
        MODAL_COOKIE_ACCEPT,
        MODAL_COOKIE_REJECT,
        MODAL_EMAIL_NAG_CLOSE,
    ];

    for selector in &dismiss_selectors {
        let js = format!(r#"
            (() => {{
                const el = document.querySelector({});
                if (!el) return false;
                el.click();
                return true;
            }})()
        "#, serde_json::to_string(selector)?);
        let clicked = ctx.page().evaluate(js).await?.value()
            .and_then(|v| v.as_bool()).unwrap_or(false);
        if clicked {
            info!("[twitter_popup] Dismissed modal: {}", selector);
            ctx.pause(500, 30).await;
        }
    }

    // Also attempt ESC key as universal close
    ctx.press("Escape").await?;
    ctx.pause(300, 20).await;

    Ok(())
}
```

---

## `src/utils/twitter/twitteractivity_sentiment.rs`

**Responsibility**: Lightweight sentiment detection using keyword blocklist.

```rust
use log::info;

/// Check if text contains negative sentiment keywords.
pub fn contains_negative_sentiment(text: &str, blocklist: &[String]) -> bool {
    let lower = text.to_lowercase();
    blocklist.iter().any(|word| lower.contains(word))
}
```

---

## `src/utils/twitter/twitteractivity_persona.rs`

**Responsibility**: Map `BrowserProfile` → `TwitterPersona` and adjust behavior overlays.

```rust
use crate::utils::profile::BrowserProfile;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TwitterPersona {
    Efficient,
    Casual,
    Researcher,
    Hesitant,
    Distracted,
    Focused,
}

impl TwitterPersona {
    /// Derive from BrowserProfile params using heuristic thresholds.
    pub fn from_profile(profile: &BrowserProfile) -> Self {
        let cursor_speed = profile.cursor_speed.base;
        let idle_chance = profile.cursor_micro_pause_chance.base;
        let scroll_smooth = profile.scroll_smoothness.base;

        match (cursor_speed, idle_chance, scroll_smooth) {
            (s, i, _) if s >= 1.7 && i < 5.0 => Self::Efficient,
            (s, i, _) if s <= 0.6 && i >= 20.0 => Self::Researcher,
            (s, i, _) if i >= 25.0 && s < 1.0 => Self::Distracted,
            (s, i, _) if i <= 5.0 && s >= 1.4 => Self::Focused,
            (s, i, _) if s >= 1.5 && i >= 10.0 => Self::Casual,
            _ => Self::Casual,
        }
    }

    pub fn dive_probability_multiplier(&self) -> f64 {
        match self {
            Self::Efficient => 1.2,
            Self::Casual => 1.0,
            Self::Researcher => 1.5,
            Self::Hesitant => 0.7,
            Self::Distracted => 0.6,
            Self::Focused => 1.3,
        }
    }
}
```

---

## `src/utils/twitter/twitteractivity_selectors.rs`

**Centralized selector definitions with fallbacks**.

```rust
/// Tweet container selectors (in priority order)
pub const TWEET_ARTICLE: &str = "article[data-testid=\"tweet\"]";
pub const TWEET_CELL_INNER: &str = "div[data-testid=\"cellInnerDiv\"]";
pub const TWEET_ARTICLE_FALLBACK: &str = "article";

/// Button selectors
pub const BTN_LIKE: &str = "[data-testid=\"like\"]";
pub const BTN_UNLIKE: &str = "[data-testid=\"unlike\"]";
pub const BTN_RETWEET: &str = "[data-testid=\"retweet\"]";
pub const BTN_RETWEET_CONFIRM: &str = "[data-testid=\"retweetConfirm\"]";
pub const BTN_FOLLOW: &str = "[data-testid=\"follow\"]";
pub const BTN_FOLLOWING: &str = "[data-testid=\"following\"]";
pub const BTN_BOOKMARK: &str = "[data-testid=\"bookmark\"]";
pub const BTN_BOOKMARKED: &str = "[data-testid=\"bookmark\"]";

/// Modal / overlay selectors
pub const MODAL_CLOSE_X: &str = "[aria-label=\"Close\"]";
pub const MODAL_CLOSE_BUTTON: &str = "button[data-testid=\"app-presence-close-button\"]";
pub const MODAL_SIGNUP_CLOSE: &str = "[data-testid=\"dismiss\"]";
pub const MODAL_NOTIFICATION_DISMISS: &str = "button[aria-label=\"Dismiss\"]";
pub const MODAL_COOKIE_ACCEPT: &str = "button:has-text('Accept all cookies')";
pub const MODAL_COOKIE_REJECT: &str = "button:has-text('Reject non-essential cookies')";
pub const MODAL_EMAIL_NAG_CLOSE: &str = "[data-testid=\"dismiss\"]";
```

---

## `src/utils/twitter/twitteractivity_humanized.rs`

Humanization helpers specific to Twitter interaction:

```rust
use crate::prelude::TaskContext;

/// Hover briefly before clicking (simulates visual verification).
pub async fn hover_before_click(ctx: &TaskContext, x: f64, y: f64) -> Result<()> {
    ctx.move_mouse_to(x, y).await?;
    ctx.pause(200, 40).await;
    Ok(())
}
```

---

## `src/utils/twitter/mod.rs`

```rust
pub mod twitteractivity_navigation;
pub mod twitteractivity_feed;
pub mod twitteractivity_dive;
pub mod twitteractivity_interact;
pub mod twitteractivity_popup;
pub mod twitteractivity_sentiment;
pub mod twitteractivity_persona;
pub mod twitteractivity_selectors;
pub mod twitteractivity_humanized;

pub use twitteractivity_navigation::select_entry_point;
pub use twitteractivity_feed::TweetMetadata;
pub use twitteractivity_feed::find_random_tweet;
pub use twitteractivity_dive::{click_tweet, load_tweet_context, TweetContext};
pub use twitteractivity_interact::{like_tweet, retweet_tweet, follow_user, bookmark_tweet};
pub use twitteractivity_popup::close_all_modals;
pub use twitteractivity_sentiment::contains_negative_sentiment;
pub use twitteractivity_persona::TwitterPersona;
pub use twitteractivity_selectors::*;
pub use twitteractivity_humanized::*;
```

---

*This document is part of the Twitter Activity task planning suite. See [README.md](README.md) for navigation.*
