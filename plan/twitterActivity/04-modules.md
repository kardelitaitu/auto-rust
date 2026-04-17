# Twitter Activity — Helper Modules Specification

This document specifies all helper modules that support the `twitteractivity` task. Each module is a separate Rust file under `src/task/` or `src/utils/twitter/`.

---

## `twitter_navigation.rs`

**Responsibility**: Entry URL selection and navigation with error handling.

```rust
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

/// Navigate to URL with 20s total timeout (15s nav + 5s wait_for_load)
#[allow(dead_code)]
pub async fn navigate_to(page: &Page, url: &str) -> Result<()> {
    let nav_start = std::time::Instant::now();
    navigation::goto(page, url, 15000).await?;

    let elapsed = nav_start.elapsed().as_millis() as u64;
    let remaining = 20000u64.saturating_sub(elapsed);

    match timeout(
        Duration::from_millis(remaining),
        navigation::wait_for_load(page, remaining)
    ).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => {
            warn!("wait_for_load returned error: {e}, continuing anyway");
            Ok(())
        }
        Err(_) => {
            warn!("wait_for_load timeout after {remaining}ms, continuing");
            Ok(())
        }
    }
}
```

**Functions to implement**:
- `weighted_pick(items: &[(T, u32)]) -> Result<T>` — cumulative weight random
- `pick_explore_url() -> String` — random from `["/explore", "/explore/tabs/for-you", "/explore/tabs/trending", "/i/jf/global-trending/home"]`
- `pick_connect_url() -> String` — random from connect people pages
- `pick_supplementary_url() -> String` — random from news/sports/entertainment topic pages

---

## `twitter_feed.rs`

**Responsibility**: Scroll the feed and locate candidate tweets using resilient selectors.

```rust
/// Representation of a tweet found in the feed.
#[derive(Debug, Clone)]
pub struct TweetMetadata {
    pub id: String,
    pub author_handle: String,
    pub author_name: String,
    pub text_preview: String,
    pub has_media: bool,
    pub element_handle: Option<ElementHandle>,
    pub selector_used: String,
}

/// Scans the current feed viewport and returns a random tweet candidate.
#[allow(dead_code)]
pub async fn find_random_tweet(page: &Page) -> Result<Option<TweetMetadata>> {
    let selectors = [
        selectors::TWEET_ARTICLE,
        selectors::TWEET_CELL_INNER,
        selectors::TWEET_ARTICLE_FALLBACK,
        selectors::TWEET_XPATH,
    ];

    let mut candidates = Vec::new();

    for selector in &selectors {
        let elements = page.querySelectorAll(selector).await?;
        if !elements.is_empty() {
            for el in &elements {
                if let Some(meta) = extract_metadata_from_element(el, selector).await? {
                    candidates.push(meta);
                }
            }
        }
        if !candidates.is_empty() {
            break;
        }
    }

    if candidates.is_empty() {
        return Ok(None);
    }

    let idx = rand::thread_rng().gen_range(0..candidates.len());
    Ok(Some(candidates[idx].clone()))
}
```

---

## `twitter_dive.rs`

**Responsibility**: Click into tweet, expand content, extract full context.

```rust
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

/// Clicks tweet to expand it (if collapsed), then extracts full context.
#[allow(dead_code)]
pub async fn load_tweet_context(
    page: &Page,
    tweet: &TweetMetadata
) -> Result<TweetContext> {
    // Scroll tweet into view
    let viewport = page_size::get_viewport(page).await?;
    let target_y = viewport.height / 2.0;
    page.evaluate(format!(
        "window.scrollTo({{top: {}, behavior: 'smooth'}});",
        target_y as i32
    )).await?;
    human_pause(500, 30).await;

    // Click tweet to expand (caller ensures tweet is present)
    // ... (use mouse::cursor_move_to + click_at on tweet.element_handle center)

    // Wait for expansion
    human_pause(1000, 50).await;

    // Scrape full context via evaluate (see full spec in monolith for JS code)
    // Returns TweetContext
    unimplemented!()
}

/// Click a tweet to expand it.
#[allow(dead_code)]
pub async fn click_tweet(page: &Page, tweet: &TweetMetadata) -> Result<()> {
    // Use element handle to get center coordinates, then mouse click
    // Ensure element is in viewport first
    unimplemented!()
}
```

---

## `twitter_interact.rs`

**Responsibility**: UI automation for engagement actions.

```rust
/// Like a tweet — clicks like button and verifies state flips to "liked".
#[allow(dead_code)]
pub async fn like_tweet(page: &Page, tweet: &TweetMetadata) -> Result<()> {
    let like_btn = page.querySelector(selectors::BTN_LIKE).await?;

    if like_btn.is_none() {
        // Already liked? Check for unlike button
        let unlike_btn = page.querySelector(selectors::BTN_UNLIKE).await?;
        if unlike_btn.is_some() {
            info!("[twitter] Tweet already liked, skipping");
            return Ok(());
        }
        bail!("Like button not found");
    }

    let (x, y) = page_size::get_element_center(page, selectors::BTN_LIKE).await?;
    mouse::cursor_move_to(page, x, y).await?;
    human_pause(200, 40).await;  // hover pause
    mouse::click_at(page, x, y).await?;

    // Verify state change: like → unlike (or color change)
    let start = Instant::now();
    loop {
        let new_btn = page.querySelector(selectors::BTN_UNLIKE).await?;
        if new_btn.is_some() || start.elapsed() > Duration::from_secs(2) {
            break;
        }
        human_pause(100, 20).await;
    }

    Ok(())
}

/// Retweet (native RT, no comment).
#[allow(dead_code)]
pub async fn retweet_tweet(page: &Page, tweet: &TweetMetadata) -> Result<()> {
    let rt_btn = page.querySelector(selectors::BTN_RETWEET).await?
        .ok_or_else(|| anyhow!("Retweet button not found"))?;

    let (x, y) = page_size::get_element_center(page, selectors::BTN_RETWEET).await?;
    mouse::click_at(page, x, y).await?;
    human_pause(300, 40).await;

    // Modal appears — click "Retweet" confirm button (not "Quote")
    let confirm_selector = selectors::MODAL_RETWEET_CONFIRM;
    let confirm_btn = page.querySelector(confirm_selector).await?
        .ok_or_else(|| anyhow!("Retweet confirm button not found"))?;

    let (cx, cy) = page_size::get_element_center(page, confirm_selector).await?;
    mouse::click_at(page, cx, cy).await?;

    // Verify modal closes (best effort)
    human_pause(1000, 50).await;

    Ok(())
}

/// Follow user.
#[allow(dead_code)]
pub async fn follow_user(page: &Page, tweet: &TweetMetadata) -> Result<()> {
    let follow_btn = page.querySelector(selectors::BTN_FOLLOW).await?;

    if follow_btn.is_none() {
        // Possibly already following? Check "Following" state
        let following_btn = page.querySelector(selectors::BTN_FOLLOWING).await?;
        if following_btn.is_some() {
            info!("[twitter] Already following @{}", tweet.author_handle);
            return Ok(());
        }
        bail!("Follow button not found for @{}", tweet.author_handle);
    }

    let (x, y) = page_size::get_element_center(page, selectors::BTN_FOLLOW).await?;
    mouse::click_at(page, x, y).await?;

    // Verify state: "Follow" → "Following" or "Pending"
    human_pause(800, 50).await;

    Ok(())
}

/// Bookmark tweet (disabled V1, stub for future).
#[allow(dead_code)]
pub async fn bookmark_tweet(page: &Page, tweet: &TweetMetadata) -> Result<()> {
    bail!("Bookmark action disabled in V1");
}
```

---

## `twitter_popup.rs`

**Responsibility**: Dismiss known modal/popup types after every navigation.

```rust
/// Known modal/popup types with dismissal selectors.
#[derive(Debug, Clone, Copy)]
pub enum ModalType {
    SignUpOverlay,
    NotificationPrompt,
    CookieBanner,
    EmailUpdate,
    AppInstallPrompt,
    Unknown,
}

/// Attempts to close all known modal types. Idempotent — safe to call repeatedly.
#[allow(dead_code)]
pub async fn close_all_modals(page: &Page) -> Result<()> {
    let dismiss_selectors = [
        selectors::MODAL_CLOSE_X,
        selectors::MODAL_CLOSE_BUTTON,
        selectors::MODAL_SIGNUP_CLOSE,
        selectors::MODAL_NOTIFICATION_DISMISS,
        selectors::MODAL_COOKIE_ACCEPT,
        selectors::MODAL_COOKIE_REJECT,
        selectors::MODAL_EMAIL_NAG_CLOSE,
    ];

    for selector in &dismiss_selectors {
        if let Some(el) = page.querySelector(selector).await? {
            info!("[twitter_popup] Dismissing modal (selector: {selector})");
            let (x, y) = page_size::get_element_center(page, selector).await?;
            mouse::click_at(page, x, y).await?;
            human_pause(500, 30).await;
        }
    }

    // Also attempt ESC key as universal close
    page.keyboard().press_key("Escape").await?;
    human_pause(300, 20).await;

    Ok(())
}

/// Checks if a known modal is currently visible.
#[allow(dead_code)]
pub async fn is_modal_visible(page: &Page) -> bool {
    for selector in &MODAL_SELECTORS {
        if let Ok(Some(el)) = page.querySelector(selector).await {
            if el.is_visible().await.unwrap_or(false) {
                return true;
            }
        }
    }
    false
}
```

---

## `twitter_sentiment.rs`

**Responsibility**: Lightweight sentiment detection using keyword blocklist.

```rust
/// Check if text contains negative sentiment keywords.
#[allow(dead_code)]
pub fn contains_negative_sentiment(
    text: &str,
    blocklist: &[String]
) -> bool {
    let lower = text.to_lowercase();
    blocklist.iter().any(|word| lower.contains(word))
}
```

---

## `twitter_limits.rs`

**Responsibility**: Track per-session engagement counters with hard caps.

```rust
#[derive(Debug, Clone, Copy)]
pub struct EngagementCounters {
    pub likes: u32,
    pub retweets: u32,
    pub follows: u32,
    pub replies: u32,
    pub quotes: u32,
    pub bookmarks: u32,
}

impl EngagementCounters {
    pub fn new() -> Self {
        Self {
            likes: 0, retweets: 0, follows: 0,
            replies: 0, quotes: 0, bookmarks: 0,
        }
    }

    pub fn can_engage(&self, action: Engagement, limits: &TwitterLimits) -> bool {
        match action {
            Engagement::Like => self.likes < limits.max_likes_per_session,
            Engagement::Retweet => self.retweets < limits.max_retweets_per_session,
            Engagement::Follow => self.follows < limits.max_follows_per_session,
            Engagement::Reply => self.replies < limits.max_replies_per_session,
            Engagement::Quote => self.quotes < limits.max_quotes_per_session,
            Engagement::Bookmark => self.bookmarks < limits.max_bookmarks_per_session,
        }
    }

    pub fn increment(&mut self, action: Engagement) {
        match action {
            Engagement::Like => self.likes += 1,
            Engagement::Retweet => self.retweets += 1,
            Engagement::Follow => self.follows += 1,
            Engagement::Reply => self.replies += 1,
            Engagement::Quote => self.quotes += 1,
            Engagement::Bookmark => self.bookmarks += 1,
        }
    }
}
```

---

## `twitter_persona.rs`

**Responsibility**: Map `BrowserProfile` → `TwitterPersona` and adjust behavior overlays.

```rust
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

    /// Returns (mouse%, keyboard%, wheel%) input method weights.
    pub fn input_method_weights(&self) -> (f64, f64, f64) {
        match self {
            Self::Efficient => (0.80, 0.15, 0.05),
            Self::Casual => (0.72, 0.18, 0.10),
            Self::Researcher => (0.60, 0.30, 0.10),
            Self::Hesitant => (0.50, 0.35, 0.15),
            Self::Distracted => (0.65, 0.20, 0.15),
            Self::Focused => (0.75, 0.20, 0.05),
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

## `src/utils/twitter/selectors.rs`

**Centralized selector definitions with fallbacks**.

```rust
/// Tweet container selectors (in priority order)
pub const TWEET_ARTICLE: &str = "article[data-testid=\"tweet\"]";
pub const TWEET_CELL_INNER: &str = "div[data-testid=\"cellInnerDiv\"]";
pub const TWEET_ARTICLE_FALLBACK: &str = "article";
pub const TWEET_XPATH: &str = "//article[contains(@*, 'tweet')]";

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

/// Text content selectors
pub const TEXT_TWEET: &str = "[data-testid=\"tweetText\"]";
pub const TEXT_USERNAME: &str = "[data-testid=\"User-Names\"]";
```

**Fallback strategy**: Each action tries primary selector, then secondary, then XPath alternative. Log which selector succeeded.

---

## `src/utils/twitter/humanized.rs`

Humanization helpers specific to Twitter interaction:

```rust
/// Hover briefly before clicking (simulates visual verification).
pub async fn hover_pause(page: &Page, x: f64, y: f64) -> Result<()> {
    mouse::cursor_move_to(page, x, y).await?;
    human_pause(200, 40).await;
    Ok(())
}

/// Idle wiggle — tiny cursor movements simulating inattention.
pub async fn idle_wiggle(page: &Page) -> Result<()> {
    if rand::random::<f64>() < 0.15 {
        use crate::utils::math::random_in_range;
        let base_x = random_in_range(100, 800) as f64;
        let base_y = random_in_range(100, 600) as f64;

        mouse::cursor_move_to(page, base_x + 10.0, base_y).await?;
        human_pause(50, 20).await;
        mouse::cursor_move_to(page, base_x - 10.0, base_y).await?;
        human_pause(50, 20).await;
        mouse::cursor_move_to(page, base_x, base_y).await?;
    }
    Ok(())
}
```

---

## `src/utils/twitter/mod.rs`

```rust
pub mod selectors;
pub mod humanized;

pub use selectors::*;
pub use humanized::*;
```

---

*This document is part of the Twitter Activity task planning suite. See [README.md](README.md) for navigation.*
