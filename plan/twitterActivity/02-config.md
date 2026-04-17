# Twitter Activity — Configuration & Profile

This document covers all configuration-related aspects of the Twitter activity task:
- Config schema extensions (`[twitter]` section in `config/default.toml`)
- Environment variable overrides
- `BrowserProfile` extension for Twitter-specific parameters
- Persona mapping

---

## 3. Rust Implementation Plan (Config-Related Parts)

### Directory Structure

```
src/task/
  mod.rs               # Register twitteractivity module
  twitteractivity.rs   # Main entry
  twitter_agent.rs     # TwitterAgent struct
  twitter_navigation.rs
  twitter_feed.rs
  twitter_dive.rs
  twitter_interact.rs
  twitter_popup.rs
  twitter_sentiment.rs
  twitter_limits.rs
  twitter_persona.rs

src/utils/twitter/
  selectors.rs
  humanized.rs
  mod.rs

src/validation/
  task.rs
```

### Public API (task-level)

```rust
// src/task/twitteractivity.rs
pub async fn run(
    session_id: &str,
    page: &Page,
    payload: Value,
    max_retries: u32
) -> Result<TaskResult> {
    let config = load_twitter_config()?;
    let profile = get_session_profile()?;
    let mut agent = TwitterAgent::new(page, profile, config).await?;
    agent.run_session(cycles: 5..10, duration: 540..840).await
}
```

**Task Registration** (in `src/task/mod.rs`):
```rust
pub mod cookiebot;
pub mod pageview;
pub mod twitteractivity;

// In perform_task match arm:
match clean_name {
    "cookiebot" => cookiebot::run(session_id, page, payload.clone()).await,
    "pageview" => pageview::run(session_id, page, payload.clone()).await,
    "twitteractivity" => twitteractivity::run(session_id, page, payload.clone(), max_retries).await,
    _ => Err(anyhow::anyhow!("Unknown task: {name}")),
}
```

---

## 4. Config Schema Extensions (`config/default.toml` + `src/config.rs`)

### 4.1 TOML Configuration

Add the following `[twitter]` section to your `config/default.toml`:

```toml
# ─── Twitter Activity Task Configuration ────────────────────────────────────
[twitter]
enabled = true
min_cycles = 5
max_cycles = 10
min_duration_sec = 540
max_duration_sec = 840
timeout_ms = 600000  # Task-level hard timeout

[twitter.entry_points]
# Weighted selection of entry URLs per cycle (sum to ~100)
home_weight = 59
explore_weight = 32   # Combined: explore tabs (for-you, trending, tabs)
connect_weight = 4    # Connect people pages
supplementary_weight = 5  # News, sports, entertainment supplements

[twitter.engagement]
# Engagement probabilities PER TWEET that is "dived" into (conditional on diving decision)
like_probability = 0.30    # P(like | dive)
retweet_probability = 0.15 # P(retweet | dive)
follow_probability = 0.10  # P(follow author | dive)
reply_probability = 0.00   # V1: disabled (needs LLM)
quote_probability = 0.00   # V1: disabled (complex UI)
bookmark_probability = 0.00 # V1: disabled (low value, bookmark UI changes frequently)
# Feature flags for V2: set true to enable AI-generated text for replies/quotes
reply_with_ai = false      # V2: require LLM for reply generation
quote_with_ai = false      # V2: require LLM for quote generation

[twitter.limits]
# Absolute per-session caps (hard stops)
max_likes_per_session = 5
max_retweets_per_session = 3
max_follows_per_session = 2
max_replies_per_session = 0   # V1: disabled
max_quotes_per_session = 0    # V1: disabled
max_bookmarks_per_session = 0 # V1: disabled

[twitter.sentiment]
# Simple keyword-based sentiment guard (conservative V1 approach)
block_negative_engagement = false   # true → skip engagement on negative tweets
negative_keywords = [
  "hate", "kill", "die", "awful", "terrible",
  "disgusting", "worst", "disaster", "horrible", "stupid",
  "suck", "worse", "useless", "garbage", "trash", "scam"
]

[twitter.styles]
# Theme enforcement (optional; mirrors `theme` from BrowserProfile)
prefer_dark_mode = true   # If true, attempts to switch to dark theme on first load
ignore_theme_mismatch = true  # Don't fail if theme toggle button is missing
```

### 4.2 Rust Struct Definitions (`src/config.rs`)

Add these structs to `src/config.rs`:

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct TwitterConfig {
    pub enabled: bool,
    pub min_cycles: u32,
    pub max_cycles: u32,
    pub min_duration_sec: u64,
    pub max_duration_sec: u64,
    pub timeout_ms: u64,
    pub entry_points: TwitterEntryConfig,
    pub engagement: TwitterEngagementConfig,
    pub limits: TwitterLimits,
    pub sentiment: TwitterSentimentConfig,
    pub styles: TwitterStyleConfig,
    // Optional override: if set, overrides profile's derived dive probability
    #[serde(default)]
    pub global_dive_probability: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TwitterEntryConfig {
    pub home_weight: u32,
    pub explore_weight: u32,
    pub connect_weight: u32,
    pub supplementary_weight: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TwitterEngagementConfig {
    pub like_probability: f64,
    pub retweet_probability: f64,
    pub follow_probability: f64,
    pub reply_probability: f64,
    pub quote_probability: f64,
    pub bookmark_probability: f64,
    // Feature flags: require AI generation for reply/quote (V2)
    #[serde(default)]
    pub reply_with_ai: bool,
    #[serde(default)]
    pub quote_with_ai: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TwitterLimits {
    pub max_likes_per_session: u32,
    pub max_retweets_per_session: u32,
    pub max_follows_per_session: u32,
    pub max_replies_per_session: u32,
    pub max_quotes_per_session: u32,
    pub max_bookmarks_per_session: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TwitterSentimentConfig {
    pub block_negative_engagement: bool,
    pub negative_keywords: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TwitterStyleConfig {
    pub prefer_dark_mode: bool,
    pub ignore_theme_mismatch: bool,
}

// Extend top-level Config:
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub browser: BrowserConfig,
    pub orchestrator: OrchestratorConfig,
    #[serde(default)]
    pub twitter: TwitterConfig,  // ← add this
}
```

**Note on `BrowserProfile` extension** (in `src/utils/profile.rs`):
To support per-profile dive probability, add one field:

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct BrowserProfile {
    // ... existing fields ...
    /// Probability (0–1) that the agent decides to "dive" into a tweet during a cycle.
    /// Higher = more engagements per session.
    #[serde(default)]
    pub dive_probability: ProfileParam,
}
```

Then, for each preset constructor (`BrowserProfile::average()`, etc.), set a sensible default:

```rust
dive_probability: p(0.35, 20.0), // base 35% ±20% variation
```

If not set (e.g., loading older profiles), default to 0.35. This field is Twitter-specific but harmless for other tasks.

### 4.3 Default Fallback (`default_twitter_config()`)

In `config.rs`, provide a code-based default for when TOML section is missing:

```rust
fn default_twitter_config() -> TwitterConfig {
    TwitterConfig {
        enabled: true,
        min_cycles: 5,
        max_cycles: 10,
        min_duration_sec: 540,
        max_duration_sec: 840,
        timeout_ms: 600_000,
        entry_points: TwitterEntryConfig {
            home_weight: 59,
            explore_weight: 32,
            connect_weight: 4,
            supplementary_weight: 5,
        },
        engagement: TwitterEngagementConfig {
            like_probability: 0.30,
            retweet_probability: 0.15,
            follow_probability: 0.10,
            reply_probability: 0.00,
            quote_probability: 0.00,
            bookmark_probability: 0.00,
            reply_with_ai: false,
            quote_with_ai: false,
        },
        limits: TwitterLimits {
            max_likes_per_session: 5,
            max_retweets_per_session: 3,
            max_follows_per_session: 2,
            max_replies_per_session: 0,
            max_quotes_per_session: 0,
            max_bookmarks_per_session: 0,
        },
        sentiment: TwitterSentimentConfig {
            block_negative_engagement: false,
            negative_keywords: vec![
                "hate", "kill", "die", "awful", "terrible",
                "disgusting", "worst", "disaster", "horrible", "stupid"
            ].into_iter().map(String::from).collect(),
        },
        styles: TwitterStyleConfig {
            prefer_dark_mode: true,
            ignore_theme_mismatch: true,
        },
    }
}
```

---

## 5. Environment Variable Overrides

All Twitter config fields can be overridden via environment variables. Prefix: `TWITTER_`, uppercase, section-key joined by underscore.

| ENV Var | TOML path | Default | Type | Notes |
|---------|-----------|---------|------|-------|
| `TWITTER_ENABLED` | `twitter.enabled` | `true` | bool | Master switch |
| `TWITTER_MIN_CYCLES` | `twitter.min_cycles` | `5` | u32 | Minimum cycles per session |
| `TWITTER_MAX_CYCLES` | `twitter.max_cycles` | `10` | u32 | Maximum cycles per session |
| `TWITTER_MIN_DURATION_SEC` | `twitter.min_duration_sec` | `540` | u64 | Session floor (9min) |
| `TWITTER_MAX_DURATION_SEC` | `twitter.max_duration_sec` | `840` | u64 | Session ceiling (14min) |
| `TWITTER_TIMEOUT_MS` | `twitter.timeout_ms` | `600000` | u64 | Task-level hard timeout |
| `TWITTER_ENTRY_HOME_WEIGHT` | `twitter.entry_points.home_weight` | `59` | u32 | Homepage entry % |
| `TWITTER_ENTRY_EXPLORE_WEIGHT` | `twitter.entry_points.explore_weight` | `32` | u32 | Explore tab % |
| `TWITTER_ENTRY_CONNECT_WEIGHT` | `twitter.entry_points.connect_weight` | `4` | u32 | Connect people % |
| `TWITTER_ENTRY_SUPP_WEIGHT` | `twitter.entry_points.supplementary_weight` | `5` | u32 | News/sports/ent % |
| `TWITTER_ENGAGE_LIKE_PROB` | `twitter.engagement.like_probability` | `0.30` | f64 | P(like | dive) |
| `TWITTER_ENGAGE_RT_PROB` | `twitter.engagement.retweet_probability` | `0.15` | f64 | P(retweet | dive) |
| `TWITTER_ENGAGE_FOLLOW_PROB` | `twitter.engagement.follow_probability` | `0.10` | f64 | P(follow | dive) |
| `TWITTER_ENGAGE_REPLY_PROB` | `twitter.engagement.reply_probability` | `0.00` | f64 | V1: disabled |
| `TWITTER_ENGAGE_QUOTE_PROB` | `twitter.engagement.quote_probability` | `0.00` | f64 | V1: disabled |
| `TWITTER_ENGAGE_BOOKMARK_PROB` | `twitter.engagement.bookmark_probability` | `0.00` | f64 | V1: disabled |
| `TWITTER_ENGAGE_REPLY_WITH_AI` | `twitter.engagement.reply_with_ai` | `false` | bool | V2: enable AI-generated replies |
| `TWITTER_ENGAGE_QUOTE_WITH_AI` | `twitter.engagement.quote_with_ai` | `false` | bool | V2: enable AI-generated quote tweets |
| `TWITTER_LIMIT_LIKES` | `twitter.limits.max_likes_per_session` | `5` | u32 | Absolute cap |
| `TWITTER_LIMIT_RETWEETS` | `twitter.limits.max_retweets_per_session` | `3` | u32 | Absolute cap |
| `TWITTER_LIMIT_FOLLOWS` | `twitter.limits.max_follows_per_session` | `2` | u32 | Absolute cap |
| `TWITTER_LIMIT_REPLIES` | `twitter.limits.max_replies_per_session` | `0` | u32 | V1: disabled |
| `TWITTER_LIMIT_QUOTES` | `twitter.limits.max_quotes_per_session` | `0` | u32 | V1: disabled |
| `TWITTER_LIMIT_BOOKMARKS` | `twitter.limits.max_bookmarks_per_session` | `0` | u32 | V1: disabled |
| `TWITTER_SENTIMENT_BLOCK_NEG` | `twitter.sentiment.block_negative_engagement` | `false` | bool | Enable keyword blocklist |
| `TWITTER_STYLE_PREFER_DARK` | `twitter.styles.prefer_dark_mode` | `true` | bool | Attempt dark theme toggle |

**Example — override at runtime:**
```bash
TWITTER_LIMIT_LIKES=10 TWITTER_ENGAGE_LIKE_PROB=0.45 cargo run twitterActivity
```

This uses TOML defaults for everything except those two fields. The config loader applies `env::var()` overrides on top of TOML values (extend `apply_env_overrides()` in `config.rs` with Twitter-specific handlers).

---

## 5. Persona → Profile Mapping

We already have 21 `BrowserProfile` presets. Twitter-specific personas overlay on top:

```rust
// src/task/twitter_persona.rs
pub enum TwitterPersona {
    Efficient,    // fast cursor, low idle, low dwell time
    Casual,       // average speeds, medium idle
    Researcher,   // slow, deliberate, long reads (scroll deep)
    Hesitant,     // typo-prone, backspace-heavy, overscroll
    Distracted,   // erratic jumps, short attention, frequent tab-switch simulation
    Focused,      // smooth, linear feed consumption, minimal idle
}

impl TwitterPersona {
    pub fn from_profile(profile: &BrowserProfile) -> Self {
        // Map by profile.name (e.g., "PowerUser" → Efficient, "Analytical" → Researcher)
        // or derive from behavior params (high cursor_speed → Efficient, high micro_pause_chance → Hesitant)
    }

    pub fn input_method_weights(&self) -> (f64, f64, f64) {
        // (mouse%, keyboard%, wheel%)
        // Efficient: (80, 15, 5), Casual: (72, 18, 10), Researcher: (60, 30, 10), …
    }

    pub fn idle_chance(&self) -> f64 { … }
    pub fn speed_multiplier(&self) -> f64 { … }
}
```

**Input Method Distribution** — determines how actions are performed:

| Method | Tool | Use case |
|--------|------|----------|
| **Mouse** | `cursor_move_to()`, `click_at()` | Primary navigation, button clicks |
| **Keyboard** | `natural_typing()` (when needed) | Search queries, text entry (rare in V1) |
| **Wheel** | `scroll::scroll_by()` | Feed scrolling, timeline browsing |

V1 will use **mostly mouse actions**; keyboard-only for search (if implemented).

---

*This document is part of the Twitter Activity task planning suite. See [README.md](README.md) for navigation.*
