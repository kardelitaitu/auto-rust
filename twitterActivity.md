# Twitter Activity Task — Planning Document

**Task Name:** `twitterActivity`  
**Helper Modules:** `twitter-{search,feed,interact,follow,profile}.rs` (see section 7)
**Reference Implementation:** `auto-ai/tasks/api-twitterActivity.js` (Node.js reference)
**Status:** Planning phase → Ready for implementation
**Created:** 2026-04-17  
**Last Updated:** 2026-04-17 (post-architecture-review)

---

## 0. Existing Codebase Assets

Before implementing, review these existing utilities you can leverage:

| Module | Location | What it provides |
|--------|----------|------------------|
| Navigation | `src/utils/navigation.rs` | `goto(page, url, timeout_ms)`, `wait_for_load(page, timeout_ms)` |
| Scrolling | `src/utils/scroll.rs` | `random_scroll(page)`, `scroll_to_top/bottom(page)` |
| Mouse | `src/utils/mouse.rs` | `cursor_move_to(page, x, y)`, `click_at(page, x, y)` with Bezier curves |
| Timing | `src/utils/timing.rs` | `human_pause(base_ms, variance_pct)` — Gaussian timing |
| Page size | `src/utils/page_size.rs` | `get_viewport(page)`, `get_element_center(page, selector)` |
| Block media | `src/utils/blockmedia.rs` | `block_heavy_resources(page)` — block images/videos for speed |
| Profiles | `src/utils/profile.rs` | `BrowserProfile` (21 presets), `ProfileParam`, `randomize_profile()` |
| Config | `src/config.rs` | `Config` struct, TOML loader, env overrides, validation |
| Task runner | `src/task/mod.rs` + `cookiebot.rs`, `pageview.rs` | How to register, `perform_task` retry loop, `TaskResult` |
| Metrics | `src/metrics.rs` | `MetricsCollector`, `RunSummary` JSON export |
| Validation | `src/validation/task.rs` | Payload validation per-task |
| CLI | `src/cli.rs` | `cargo run twitterActivity`, or `cargo run twitterActivity cycles=7` |

---

## 0.5. CLI Invocation & Payload

**Basic usage:**
```bash
cargo run twitterActivity
```

**With inline parameter overrides** (supported by existing CLI parser):
```bash
# Override cycle count (future extension, V1 ignores payload)
cargo run twitterActivity=cycles=7

# Override engagement limit for this run
TWITTER_LIMIT_LIKES=10 cargo run twitterActivity

# Override probability
TWITTER_ENGAGE_LIKE_PROB=0.5 cargo run twitterActivity
```

**Payload format**: TwitterActivity accepts any JSON object in V1 (no required fields). The task ignores payload keys; all behavior is driven by config + profile. Future V2 may accept:
- `cycles`: override `min/max_cycles` for this run
- `max_likes`, `max_retweets`, `max_follows`: per-run caps
- `entry_focus`: `"home" | "explore" | "mixed"` to bias entry selection

**Pre-run checklist**:
1. `config/default.toml` has `[twitter]` section with desired limits
2. Browser session connected (RoxyBrowser or local Chrome/Brave)
3. Log directory exists (`log/`) — logger auto-creates but ensure writeable
4. Test with `RUST_LOG=debug` for verbose output: `RUST_LOG=debug cargo run twitterActivity`

---

## 1. Goal & Scope

**Primary Objective:** Simulate human-like Twitter/X browsing and engagement behavior for automation testing and behavioral research.

**Mode:** Public-only (no login required). All actions performed as logged-out/anonymous user.

**Engagement Actions (V1):**
- ✅ Like tweets
- ✅ Retweet (native RT, not Quote)
- ✅ Follow users
- ❌ Reply (deferred — requires LLM text generation)
- ❌ Quote Tweet (deferred — complex UI)
- ✅ Bookmark (included; tracked with limit, but disabled by default)

**Conservative Volume:** Low interaction rate; respects Twitter's rate-limit patterns. Per-session limits: likes ≤5, retweets ≤3, follows ≤2, bookmarks ≤0 (disabled by default).

**Scope Exclusions:**
- No login/authentication flows
- No DMs or interactions with protected accounts
- No video playback (media allowed, but not explicitly blocked)
- **No `block_heavy_resources` call** — Twitter needs images for realistic browsing; we let media load naturally
- LLM-generated replies/quotes are conditional via `REPLY_WITH_AI` / `QUOTE_WITH_AI` flags (default false)

---

## 2. Node.js Reference Architecture (Summary)

### Key Characteristics

```
aiTwitterActivityTask(page, profile, payload)
├─ Config load (replyProb=0.10, quoteProb=0.03)
├─ AITwitterAgent init (LLM engines: vLLM → Ollama → OpenRouter)
├─ Persona resolution (skimmer/balanced/deepdiver/lurker/…)
├─ Theme enforcement (dark/light)
└─ Session: 10 cycles, 540–840s total
    └─ Per-cycle:
        ├─ Input method weighted (mouse 72%, keyboard 18%, wheel 10%)
        ├─ Phase: warmup(0–10%) / active(10–80%) / cooldown(80–100%)
        ├─ Dive decision (profile.diveProbability)
        ├─ Load tweet context (scroll replies, AI context extraction)
        ├─ AI engagement roll (reply/quote/skip)
        ├─ Sentiment guard (negative → block all engagement)
        ├─ Limit guard (maxReplies 3, maxQuotes 1, maxLikes 5, …)
        ├─ LLM routing (vLLM → Ollama → OpenRouter) → generate response
        ├─ Humanize output (lowercase 30%, strip trailing period 80%)
        ├─ Post action (UI automation)
        └─ Update engagement tracker
```

### Entry Points (weighted selection)

| URL | Weight |
|-----|--------|
| `https://x.com/` (home) | 59% |
| `https://x.com/explore` | 4% |
| `https://x.com/explore/tabs/for-you` | 4% |
| `https://x.com/explore/tabs/trending` | 4% |
| `https://x.com/i/jf/global-trending/home` | 4% |
| `https://x.com/i/bookmarks` | 4% |
| `https://x.com/notifications` | 4% |
| `https://x.com/notifications/mentions` | 4% |
| `https://x.com/i/chat/` | 4% |
| `https://x.com/i/connect_people?show_topics=false` | 2% |
| `https://x.com/i/connect_people?is_creator_only=true` | 2% |
| Supplements (news/sports/entertainment) | 5% |

Total = 100%. Weighted random selection per cycle.

### Behavior Modifiers (from `api/personas/`)

| Persona | Input Method | Idle Chance | Speed | Micro-move | Distraction |
|---------|-------------|-------------|-------|-----------|-------------|
| efficient | mouse-heavy | low (0.02) | fast (1.2–1.4×) | minimal | low |
| casual | balanced | medium (0.05–0.1) | normal (1.0) | occasional | medium |
| researcher | deliberate | very low (0.01) | slow (0.7×) | frequent pauses | low |
| hesitant | keyboard/wheel heavy | high (0.15) | slow (0.8×) | frequent corrections | high |
| distracted | erratic | high (0.2) | variable | random wiggles | very high |

These personas map to our `BrowserProfile` values (cursor_speed, typing_speed_mean, cursor_micro_pause_chance, etc.).

---

## 3. Rust Implementation Plan

### Directory Structure

```
src/task/
  mod.rs               # Register twitteractivity module
  twitteractivity.rs   # Main entry: run(session_id, page, payload) -> Result<()>
  twitter_agent.rs     # TwitterAgent struct (counters, persona, config, state)
  twitter_navigation.rs # Entry point selection, weighted random, goto
  twitter_feed.rs      # Scroll feed, find tweets (DOM selectors), extract metadata
  twitter_dive.rs      # Click tweet → expand → context extraction → sentiment
  twitter_interact.rs  # Actions: like(), retweet(), follow()
  twitter_popup.rs     # Close modals ("Sign up", "Enable notifications", …)
  twitter_sentiment.rs # Keyword blocklist (negative words), simple sentiment check
  twitter_limits.rs    # Enforce per-session counters (max_likes, …)
  twitter_persona.rs   # Persona → BrowserProfile mapping + behavior override

src/utils/twitter/
  selectors.rs         # CSS/XPath selectors for tweets, buttons, modals (centralized for easy updates)
  humanized.rs         # Ghost cursor micro-movements, idle wiggle, input method executor
  mod.rs               # Re-export twitter utilities

src/validation/
  task.rs              # Add twitterActivity payload validation schema
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
    let profile = get_session_profile()?; // from session via_context
    let mut agent = TwitterAgent::new(page, profile, config).await?;
    agent.run_session(cycles: 5..10, duration: 540..840).await
}
```

**Task Registration** (in `src/task/mod.rs`):
```rust
pub mod cookiebot;
pub mod pageview;
pub mod twitteractivity; // ← add this

// In perform_task match arm:
match clean_name {
    "cookiebot" => cookiebot::run(session_id, page, payload.clone()).await,
    "pageview" => pageview::run(session_id, page, payload.clone()).await,
    "twitteractivity" => twitteractivity::run(session_id, page, payload.clone(), max_retries).await,
    _ => Err(anyhow::anyhow!("Unknown task: {name}")),
}
```

---

## 4. Config Schema Extensions (`config/default.toml` + ENV)

### 4.1 Top-Level Config Changes

Add `TwitterConfig` section to `config/default.toml`:

```toml
[browser]
max_discovery_retries = 3
discovery_retry_delay_ms = 5000
connectors = []
connection_timeout_ms = 30000

[browser.circuit_breaker]
enabled = true
failure_threshold = 5
success_threshold = 3
half_open_time_ms = 30000

[[browser.profiles]]
name = "brave-local"
type = "brave"
ws_endpoint = ""

[browser.roxybrowser]
enabled = true
api_url = "http://127.0.0.1:50000/"
api_key = "c6ae203adfe0327a63ccc9174c178dec"

[orchestrator]
max_global_concurrency = 20
task_timeout_ms = 600000
group_timeout_ms = 600000
worker_wait_timeout_ms = 10000
stuck_worker_threshold_ms = 120000
task_stagger_delay_ms = 2000
max_retries = 2
retry_delay_ms = 500

# ─── Twitter Activity Task Configuration ────────────────────────────────────
[twitter]
enabled = true
min_cycles = 5
max_cycles = 10
min_duration_sec = 540
max_duration_sec = 840
timeout_ms = 600000  # Task-level hard timeout (orchestrator.task_timeout_ms covers group-level)

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

### 4.2 Config Struct Changes (`src/config.rs`)

Add new structs:

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

---

**Fallback defaults** (`config.rs` `default_twitter_config()`):

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
| `TWITTER_ENGAGE_LIKE_PROB` | `twitter.engagement.like_probability` | `0.30` | f64 | P(like \| dive) |
| `TWITTER_ENGAGE_RT_PROB` | `twitter.engagement.retweet_probability` | `0.15` | f64 | P(retweet \| dive) |
| `TWITTER_ENGAGE_FOLLOW_PROB` | `twitter.engagement.follow_probability` | `0.10` | f64 | P(follow \| dive) |
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
This uses TOML defaults for everything except those two fields. The config loader applies `env::var()` overrides on top of TOML values (see `config.rs:apply_env_overrides` → extend with Twitter-specific handlers).

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

## 6. Per-Cycle Execution Flow

### 6.1 High-Level State Machine

```rust
// TwitterAgent orchestrates N cycles (5–10) within a total duration budget (540–840s)
pub struct TwitterAgent {
    page: Page,
    profile: BrowserProfile,
    config: TwitterConfig,
    state: AgentState,
    persona: TwitterPersona, // derived from profile at startup
}

struct AgentState {
    cycle: u32,
    start_time: Instant,
    counters: EngagementCounters,
    current_phase: Phase,
}

struct EngagementCounters {
    likes: u32,
    retweets: u32,
    follows: u32,
    replies: u32,
    quotes: u32,
    bookmarks: u32,
    skipped_sentiment: u32,
    skipped_limits: u32,
    skipped_dive_decision: u32,
}
```

### 6.2 Full Cycle Pseudocode

```rust
/// Possible engagement actions the agent can take on a tweet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engagement {
    Like,
    Reply,
    Retweet,
    Quote,
    Follow,
    Bookmark,
}

/// Outcome of a single cycle execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleOutcome {
    SessionComplete,
    DurationBudgetExceeded,
    NavigationFailed,
    NavigationTimeout,
    SkippedDive,
    NoTweetsFound,
    FeedScanError,
    ClickFailed,
    ContextLoadFailed,
    BlockedBySentiment,
    AllLimitsReached,
    ActionFailed(Engagement),
    ActionDisabled,
    Engaged(Engagement),
}

async fn run_cycle(&mut self) -> Result<CycleOutcome> {
    // ═══════════════════════════════════════════════════════════════
    // PHASE 0: Session budget check
    // ═══════════════════════════════════════════════════════════════
    if self.state.cycle >= self.config.max_cycles {
        return Ok(CycleOutcome::SessionComplete);
    }
    let elapsed = self.state.start_time.elapsed().as_secs();
    if elapsed >= self.config.max_duration_sec {
        return Ok(CycleOutcome::DurationBudgetExceeded);
    }
    self.state.cycle += 1;

    info!(
        "[twitter][cycle {}/{}] Starting (elapsed: {}s)", 
        self.state.cycle, 
        self.config.max_cycles,
        elapsed
    );

    // ═══════════════════════════════════════════════════════════════
    // PHASE 1: Select entry point (weighted random)
    // ═══════════════════════════════════════════════════════════════
    let url = self.select_entry_point().await?;
    info!("[twitter][cycle {}] Entry: {}", self.state.cycle, url);

    // ═══════════════════════════════════════════════════════════════
    // PHASE 2: Navigate with timeout
    // ═══════════════════════════════════════════════════════════════
    let nav_result = timeout(
        Duration::from_secs(20),
        navigation::goto(&self.page, &url, 15000)
    ).await;

    match nav_result {
        Ok(Ok(())) => { /* proceed */ }
        Ok(Err(e)) => {
            warn!("[twitter][cycle {}] Navigation failed: {}", self.state.cycle, e);
            return Ok(CycleOutcome::NavigationFailed);
        }
        Err(_) => {
            warn!("[twitter][cycle {}] Navigation timeout", self.state.cycle);
            return Ok(CycleOutcome::NavigationTimeout);
        }
    }

    // Wait for network idle (or timeout)
    let _ = navigation::wait_for_load(&self.page, 5000).await;

    // ═══════════════════════════════════════════════════════════════
    // PHASE 3: Close popups/modals (defensive)
    // ═══════════════════════════════════════════════════════════════
    if let Err(e) = twitter_popup::close_all_modals(&self.page).await {
        warn!("[twitter][cycle {}] Modal close error: {}", self.state.cycle, e);
        // Non-fatal — continue
    }

    // ═══════════════════════════════════════════════════════════════
    // PHASE 4: Determine persona phase (warmup/active/cooldown)
    // ═══════════════════════════════════════════════════════════════
    let progress = self.state.cycle as f64 / self.config.max_cycles as f64;
    self.state.current_phase = if progress < 0.10 {
        Phase::Warmup
    } else if progress < 0.80 {
        Phase::Active
    } else {
        Phase::Cooldown
    };

    // ═══════════════════════════════════════════════════════════════
    // PHASE 5: Dive decision — roll against profile's dive_probability × persona multiplier
    // ═══════════════════════════════════════════════════════════════
    let base_dive_p = self.profile.dive_probability.random();
    let adjusted_dive_p = base_dive_p * self.persona.dive_probability_multiplier();
    let should_dive = rand::random::<f64>() < adjusted_dive_p;

    if !should_dive {
        info!("[twitter][cycle {}] Skipped (dive roll: {:.3} < {:.3})",
            self.state.cycle, adjusted_dive_p, adjusted_dive_p);
        self.state.counters.skipped_dive_decision += 1;
        return Ok(CycleOutcome::SkippedDive);
    }

    // ═══════════════════════════════════════════════════════════════
    // PHASE 6: Locate a tweet in feed (surface level)
    // ═══════════════════════════════════════════════════════════════
    let tweet = match twitter_feed::find_random_tweet(&self.page).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            warn!("[twitter][cycle {}] No tweets found in feed", self.state.cycle);
            return Ok(CycleOutcome::NoTweetsFound);
        }
        Err(e) => {
            error!("[twitter][cycle {}] Feed scan error: {}", self.state.cycle, e);
            return Ok(CycleOutcome::FeedScanError);
        }
    };

    info!("[twitter][cycle {}] Found tweet by @{} (id: {})",
        self.state.cycle, tweet.author_handle, tweet.id);

    // ═══════════════════════════════════════════════════════════════
    // PHASE 7: Click tweet to expand (dive into context)
    // ═══════════════════════════════════════════════════════════════
    if let Err(e) = twitter_dive::click_tweet(&self.page, &tweet).await {
        warn!("[twitter][cycle {}] Click failed: {}", self.state.cycle, e);
        return Ok(CycleOutcome::ClickFailed);
    }

    // ═══════════════════════════════════════════════════════════════
    // PHASE 8: Simulate reading — scroll within tweet context, maybe open replies
    // ═══════════════════════════════════════════════════════════════
    self.simulate_reading(&tweet).await?;

    // ═══════════════════════════════════════════════════════════════
    // PHASE 9: Load tweet context (post-click, post-reading)
    // ═══════════════════════════════════════════════════════════════
    let context = match twitter_dive::load_tweet_context(&self.page, &tweet).await {
        Ok(ctx) => ctx,
        Err(e) => {
            warn!("[twitter][cycle {}] Context load failed: {}", self.state.cycle, e);
            return Ok(CycleOutcome::ContextLoadFailed);
        }
    };

    // ═══════════════════════════════════════════════════════════════
    // PHASE 10: Sentiment guard (keyword blocklist)
    // ═══════════════════════════════════════════════════════════════
    if self.config.sentiment.block_negative_engagement {
        if twitter_sentiment::contains_negative_sentiment(&context.full_text, &self.config.sentiment.negative_keywords) {
            info!("[twitter][cycle {}] Blocked by sentiment guard (negative content)", self.state.cycle);
            self.state.counters.skipped_sentiment += 1;
            return Ok(CycleOutcome::BlockedBySentiment);
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // PHASE 11: Limit guard — build list of actions still available
    // ═══════════════════════════════════════════════════════════════
    let available_actions = self.available_engagement_actions();
    if available_actions.is_empty() {
        info!("[twitter][cycle {}] All engagement limits reached", self.state.cycle);
        return Ok(CycleOutcome::AllLimitsReached);
    }

    // ═══════════════════════════════════════════════════════════════
    // PHASE 12: Roll engagement action (weighted by config probabilities)
    // ═══════════════════════════════════════════════════════════════
    let action = self.roll_engagement(&available_actions).await;
    debug!("[twitter][cycle {}] Rolled action: {:?}", self.state.cycle, action);

    // ═══════════════════════════════════════════════════════════════
    // PHASE 13: Execute engagement action via UI automation
    // ═══════════════════════════════════════════════════════════════
    let action_start = Instant::now();
    match action {
        Engagement::Like => {
            if let Err(e) = twitter_interact::like_tweet(&self.page, &tweet).await {
                warn!("[twitter][cycle {}] Like failed: {}", self.state.cycle, e);
                return Ok(CycleOutcome::ActionFailed(action));
            }
        }
        Engagement::Retweet => {
            if let Err(e) = twitter_interact::retweet_tweet(&self.page, &tweet).await {
                warn!("[twitter][cycle {}] Retweet failed: {}", self.state.cycle, e);
                return Ok(CycleOutcome::ActionFailed(action));
            }
        }
        Engagement::Follow => {
            if let Err(e) = twitter_interact::follow_user(&self.page, &tweet).await {
                warn!("[twitter][cycle {}] Follow failed: {}", self.state.cycle, e);
                return Ok(CycleOutcome::ActionFailed(action));
            }
        }
        Engagement::Reply => {
            // Reply requires LLM text generation if REPLY_WITH_AI=true
            // If false, we skip with log (treated as disabled)
            if !self.config.engagement.reply_with_ai {
                info!("[twitter][cycle {}] Reply skipped (REPLY_WITH_AI=false)", self.state.cycle);
                return Ok(CycleOutcome::ActionDisabled);
            }
            // V1 stub: LLM not implemented yet
            bail!("Reply action with AI not implemented in V1");
        }
        Engagement::Quote => {
            if !self.config.engagement.quote_with_ai {
                info!("[twitter][cycle {}] Quote skipped (QUOTE_WITH_AI=false)", self.state.cycle);
                return Ok(CycleOutcome::ActionDisabled);
            }
            bail!("Quote action with AI not implemented in V1");
        }
        Engagement::Bookmark => {
            if self.config.limits.max_bookmarks_per_session == 0 {
                info!("[twitter][cycle {}] Bookmark skipped (limit=0)", self.state.cycle);
                return Ok(CycleOutcome::ActionDisabled);
            }
            if let Err(e) = twitter_interact::bookmark_tweet(&self.page, &tweet).await {
                warn!("[twitter][cycle {}] Bookmark failed: {}", self.state.cycle, e);
                return Ok(CycleOutcome::ActionFailed(action));
            }
         }
     }
     let action_duration = action_start.elapsed();

    // ═══════════════════════════════════════════════════════════════
    // PHASE 11: Update counters and log
    // ═══════════════════════════════════════════════════════════════
    self.increment_counter(&action);
    info!(
        "[twitter][cycle {}] ✓ {:?} @{} ({}ms)", 
        self.state.cycle, 
        action,
        tweet.author_handle,
        action_duration.as_millis()
    );

    // ═══════════════════════════════════════════════════════════════
    // PHASE 12: Human pause before next cycle
    // ═══════════════════════════════════════════════════════════════
    timing::human_pause(3000, 50).await;

    Ok(CycleOutcome::Engaged(action))
}

// ─────────────────────────────────────────────────────────────────────────────
// Additional TwitterAgent helper methods
// ─────────────────────────────────────────────────────────────────────────────

    /// Increment the counter for the given engagement action.
    fn increment_counter(&mut self, action: &Engagement) {
        self.state.counters.increment(*action);
    }

    /// Build the list of engagement actions still allowed by limits and probability > 0.
    /// Each action is checked against its per-session counter and engagement config.
    fn available_engagement_actions(&self) -> Vec<Engagement> {
        let mut actions = Vec::new();
        if self.state.counters.likes < self.config.limits.max_likes_per_session
            && self.config.engagement.like_probability > 0.0
        {
            actions.push(Engagement::Like);
        }
        if self.state.counters.retweets < self.config.limits.max_retweets_per_session
            && self.config.engagement.retweet_probability > 0.0
        {
            actions.push(Engagement::Retweet);
        }
        if self.state.counters.follows < self.config.limits.max_follows_per_session
            && self.config.engagement.follow_probability > 0.0
        {
            actions.push(Engagement::Follow);
        }
        if self.config.limits.max_replies_per_session > 0
            && self.state.counters.replies < self.config.limits.max_replies_per_session
            && self.config.engagement.reply_with_ai
            && self.config.engagement.reply_probability > 0.0
        {
            actions.push(Engagement::Reply);
        }
        if self.config.limits.max_quotes_per_session > 0
            && self.state.counters.quotes < self.config.limits.max_quotes_per_session
            && self.config.engagement.quote_with_ai
            && self.config.engagement.quote_probability > 0.0
        {
            actions.push(Engagement::Quote);
        }
        if self.state.counters.bookmarks < self.config.limits.max_bookmarks_per_session
            && self.config.engagement.bookmark_probability > 0.0
        {
            actions.push(Engagement::Bookmark);
        }
        actions
    }

/// Roll an engagement action from the available actions list using
/// weighted probabilities from config (normalized to sum=1 over available set).
async fn roll_engagement(&self, available: &[Engagement]) -> Engagement {
    use rand::distributions::{Distribution, WeightedIndex};
    use std::collections::HashMap;

    // Build weight map: action → configured probability
    let mut action_weights: HashMap<Engagement, f64> = HashMap::new();
    action_weights.insert(Engagement::Like, self.config.engagement.like_probability);
    action_weights.insert(Engagement::Retweet, self.config.engagement.retweet_probability);
    action_weights.insert(Engagement::Follow, self.config.engagement.follow_probability);
    action_weights.insert(Engagement::Reply, self.config.engagement.reply_probability);
    action_weights.insert(Engagement::Quote, self.config.engagement.quote_probability);
    action_weights.insert(Engagement::Bookmark, self.config.engagement.bookmark_probability);

    // Extract weights for available actions only
    let mut weights_vec = Vec::new();
    for action in available {
        let w = action_weights.get(action).cloned().unwrap_or(0.0);
        weights_vec.push(w.max(0.0)); // no negative weights
    }

    let sum: f64 = weights_vec.iter().sum();
    if sum <= 0.0 || weights_vec.is_empty() {
        // Fallback: uniform random among available
        let idx = rand::random::<usize>() % available.len();
        return available[idx];
    }

    // Normalize to positive integers for WeightedIndex
    let int_weights: Vec<u32> = weights_vec
        .iter()
        .map(|&w| (w / sum * 1000.0) as u32)
        .map(|w| if w == 0 { 1 } else { w }) // avoid zero weight collapse
        .collect();

    match WeightedIndex::new(&int_weights) {
        Ok(dist) => {
            let idx = dist.sample(&mut rand::thread_rng());
            available[idx]
        }
        Err(_) => {
            // Fallback uniform
            let idx = rand::random::<usize>() % available.len();
            available[idx]
        }
    }
}

/// Simulate human reading behavior after tweet expansion.
/// Includes short pauses, minor scroll within tweet, maybe open replies.
async fn simulate_reading(&self, tweet: &TweetMetadata) -> Result<()> {
    // Pause to "read" the tweet text (3–6 seconds)
    let read_time = rand::thread_rng().gen_range(3000..6000);
    timing::human_pause(read_time, 30).await;

    // Maybe scroll a bit within tweet if it's long (20% chance)
    if rand::random::<f64>() < 0.2 {
        let small_scroll = rand::thread_rng().gen_range(50..200);
        scroll::scroll_down(&self.page, small_scroll).await?;
        timing::human_pause(500, 30).await;
    }

    // Small chance (10%) to glance at replies — scroll down 1–2 screenfuls
    if rand::random::<f64>() < 0.1 {
        let vp = page_size::get_viewport(&self.page).await?;
        let scroll_amount = (vp.height * (1.0 + rand::random::<f64>())) as i32;
        scroll::scroll_down(&self.page, scroll_amount).await?;
        timing::human_pause(2000, 40).await;
        // Scroll back to tweet
        scroll::scroll_up(&self.page, scroll_amount).await?;
        timing::human_pause(500, 30).await;
    }

    Ok(())
}
```

### 6.3 Cycle Termination Conditions

The task ends when **any** of these become true:

1. `cycle >= config.max_cycles` (upper bound)
2. `elapsed_sec >= config.max_duration_sec` (time budget exhausted)
3. Critical error (browser crash, navigation failure → abort task with error)
4. ALL engagement counters hit their limits (early exit)

Return `TaskResult::success()` with metrics including final counter state.

---

## 7. Helper Modules Specification

### `twitter_navigation.rs`

**Responsibility**: Entry URL selection and navigation with error handling and retry.

```rust
/// Pre-defined entry point URLs with weights for weighted random selection.
/// 
/// Weights map to categories; actual URL chosen is weighted-random within category.
/// 
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
    // 1. Pick category by weight
    let category = weighted_pick(ENTRY_POINTS)?;
    
    // 2. Pick concrete URL within category (call category-specific router)
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
pub async fn navigate(page: &Page, url: &str) -> Result<()> {
    let nav_start = std::time::Instant::now();
    
    // Step 1: GOTO (max 15s)
    navigation::goto(page, url, 15000).await?;
    
    // Step 2: Wait for load (remaining time from 20s budget)
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
- `pick_supplementary_url() -> String` — random from news/sports/entertainment topic pages (expandable)

**Error strategy**: Never panic — on failure, return `Err` with descriptive message; cycle handler records `CycleOutcome::NavigationFailed` and continues.

---

### `twitter_feed.rs`

**Responsibility**: Scroll the feed and locate candidate tweets using resilient selectors.

```rust
/// Representation of a tweet found in the feed.
#[derive(Debug, Clone)]
pub struct TweetMetadata {
    /// Tweet ID (numeric or alphanumeric depending on URL structure)
    pub id: String,
    /// Author handle (without @)
    pub author_handle: String,
    /// Author display name
    pub author_name: String,
    /// Preview text (first 140 chars, stripped)
    pub text_preview: String,
    /// Whether tweet contains images/videos
    pub has_media: bool,
    /// Raw element reference for later clicking
    pub element_handle: Option<ElementHandle>,
    /// Selector that found this tweet (for re-querying if stale)
    pub selector_used: String,
}

/// Scans the current feed viewport and returns up to `max_candidates` tweet candidates.
#[allow(dead_code)]
pub async fn find_random_tweet(page: &Page) -> Result<Option<TweetMetadata>> {
    // Strategy: Try selector families in order of reliability until at least one tweet found.
    // Selector families live in `crate::utils::twitter::selectors`.
    //
    // Order of preference (most reliable first):
    // 1. `article[data-testid="tweet"]`  — Twitter's standard tweet container (modern)
    // 2. `div[data-testid="cellInnerDiv"]` — inner cell wrapper
    // 3. `article` — fallback to all articles on page
    // 4. XPath `//article[contains(@*, "tweet")]` — last resort
    
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
            for el in elements {
                if let Some(meta) = extract_metadata_from_element(el, selector).await? {
                    candidates.push(meta);
                }
            }
        }
        
        // Stop after first successful selector family
        if !candidates.is_empty() {
            break;
        }
    }
    
    if candidates.is_empty() {
        return Ok(None);
    }
    
    // Pick random candidate from available
    let idx = rand::thread_rng().gen_range(0..candidates.len());
    Ok(Some(candidates[idx].clone()))
}

/// Extracts tweet metadata from a DOM element.
async fn extract_metadata_from_element(
    element: &ElementHandle, 
    source_selector: &str
) -> Result<Option<TweetMetadata>> {
    // Use element.evaluate() to safely extract data without consuming handle
    let data: serde_json::Value = element.evaluate(|el| {
        let id = el.getAttribute("data-tweet-id")
            .or_else(|| el.getAttribute("data-item-id"))
            .or_else(|| {
                // Fallback: extract from link within tweet
                let link = el.querySelector('a[href*="/status/"]');
                link?.getAttribute("href")
            })
            .unwrap_or("unknown");
        
        let author_handle = el.getAttribute("data-screenname")
            .or_else(|| {
                let link = el.querySelector('a[href*="/"]');
                link?.getAttribute("href")?.strip_prefix("/")?.to_string()
            })
            .unwrap_or("unknown");
        
        let text = el.querySelector('div[data-testid="tweetText"]')
            .map(|t| t.textContent().unwrap_or_default())
            .unwrap_or_default();
        
        let has_media = el.querySelector('img[alt="Image"], video') != null;
        
        serde_json::json!({
            "id": id,
            "author_handle": author_handle,
            "author_name": el.querySelector('span[data-testid="User-Names"]').map(|n| n.textContent().unwrap_or_default()).unwrap_or_default(),
            "text_preview": text.chars().take(140).collect::<String>(),
            "has_media": has_media,
        })
    }).await?;

    Ok(Some(TweetMetadata {
        id: data["id"].as_str().unwrap_or("unknown").to_string(),
        author_handle: data["author_handle"].as_str().unwrap_or("unknown").to_string(),
        author_name: data["author_name"].as_str().unwrap_or("").to_string(),
        text_preview: data["text_preview"].as_str().unwrap_or("").to_string(),
        has_media: data["has_media"].as_bool().unwrap_or(false),
        element_handle: Some(element.clone()),
        selector_used: source_selector.to_string(),
    }))
}
```

**Selector resilience**: Store selectors in `src/utils/twitter/selectors.rs` as centralized constants. Update cycle daily if selectors break. Provide 2–3 fallback selectors per element type.

---

### `twitter_dive.rs`

**Responsibility**: Click into tweet, expand content, extract full context for sentiment/action decision.

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
    // Step 1: Scroll tweet into view (center of viewport)
    let viewport = page_size::get_viewport(page).await?;
    let target_y = viewport.height / 2.0;
    
    // Use smooth scroll for natural movement
    page.evaluate(format!(
        "window.scrollTo({{top: {}, behavior: 'smooth'}});",
        target_y as i32
    )).await?;
    
    human_pause(500, 30).await;

    // Step 2: Click tweet to expand (if collapsed)
    // ... use mouse::cursor_move_to + click_at on tweet.element_handle center
    
    // Step 3: Wait for expansion animation (max 2s)
    human_pause(1000, 50).await;

    // Step 4: Scrape full context via evaluate
    let context_json: serde_json::Value = page.evaluate(r#"
        (() => {
            const tweet = document.querySelector('[data-testid="tweet"]') ||
                         document.querySelector('article');
            if (!tweet) return null;
            
            const textEl = tweet.querySelector('[data-testid="tweetText"]');
            const text = textEl ? textEl.innerText : '';
            
            const getCount = (label) => {
                const el = tweet.querySelector(`[data-testid="${label}"]`);
                if (!el) return 0;
                const txt = el.innerText.trim();
                // Parse "1.2K", "3,405" etc. (Twitter uses shorthand)
                return parseTwitterCount(txt);
            };
            
            const authorLink = tweet.querySelector('a[href*="/"]');
            const href = authorLink?.getAttribute('href') || '';
            const handle = href.split('/')[1] || 'unknown';
            
            return {
                tweet_id: tweet.getAttribute('data-tweet-id') || tweet.getAttribute('data-item-id') || 'unknown',
                author_handle: handle,
                full_text: text,
                reply_count: getCount('reply'),
                retweet_count: getCount('retweet'),
                like_count: getCount('like'),
                quote_count: getCount('quote'),
                is_from_followed_account: tweet.querySelector('div[data-testid="unfollow"]') != null
            };
        })
        // Helper injected inline for count parsing
        function parseTwitterCount(txt) {
            if (!txt) return 0;
            txt = txt.replace(/,/g, '').toLowerCase();
            if (txt.includes('k')) {
                return Math.floor(parseFloat(txt) * 1000);
            }
            if (txt.includes('m')) {
                return Math.floor(parseFloat(txt) * 1000000);
            }
            return parseInt(txt) || 0;
        }
    "#).await?;

    if context_json.is_null() {
        bail!("Failed to extract tweet context — tweet element not found");
    }

    Ok(TweetContext {
        tweet_id: context_json["tweet_id"].as_str().unwrap_or("unknown").to_string(),
        author_handle: context_json["author_handle"].as_str().unwrap_or("unknown").to_string(),
        full_text: context_json["full_text"].as_str().unwrap_or("").to_string(),
        reply_count: context_json["reply_count"].as_u64().unwrap_or(0) as u32,
        retweet_count: context_json["retweet_count"].as_u64().unwrap_or(0) as u32,
        like_count: context_json["like_count"].as_u64().unwrap_or(0) as u32,
        quote_count: context_json["quote_count"].as_u64().unwrap_or(0) as u32,
        timestamp: context_json["timestamp"].as_str().map(|s| s.to_string()),
        is_from_followed_account: context_json["is_from_followed_account"].as_bool().unwrap_or(false),
    })
}
```

---

### `twitter_interact.rs`

**Responsibility**: UI automation for engagement actions. Each function clicks and verifies state change.

```rust
/// Like a tweet — clicks like button and verifies state flips to "liked".
#[allow(dead_code)]
pub async fn like_tweet(page: &Page, tweet: &TweetMetadata) -> Result<()> {
    // Selector: `[data-testid="like"]` (unliked state) or `[data-testid="unlike"]` (already liked)
    // We want to click "like" only if currently unliked.
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
    
    // Click using humanized cursor movement
    let (x, y) = page_size::get_element_center(page, selectors::BTN_LIKE).await?;
    let profile = /* from context */;
    mouse::cursor_move_to_with_profile(page, x, y, &profile).await?;
    human_pause(200, 30).await;  // hover pause
    mouse::click_at(page, x, y).await?;
    
    // Verify state change: like → unlike (or color change)
    // Wait up to 2s for button state to update
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
    // Step 1: Click retweet button (opens modal)
    let rt_btn = page.querySelector(selectors::BTN_RETWEET).await?
        .ok_or_else(|| anyhow!("Retweet button not found"))?;
    
    let (x, y) = page_size::get_element_center(page, selectors::BTN_RETWEET).await?;
    mouse::click_at(page, x, y).await?;
    human_pause(300, 40).await;
    
    // Step 2: Modal appears — click "Retweet" confirm button (not "Quote")
    let confirm_selector = selectors::MODAL_RETWEET_CONFIRM;
    let confirm_btn = page.querySelector(confirm_selector).await?
        .ok_or_else(|| anyhow!("Retweet confirm button not found"))?;
    
    let (cx, cy) = page_size::get_element_center(page, confirm_selector).await?;
    mouse::click_at(page, cx, cy).await?;
    
    // Step 3: Verify modal closes and retweet count increments (best effort)
    human_pause(1000, 50).await;
    
    Ok(())
}

/// Follow user.
#[allow(dead_code)]
pub async fn follow_user(page: &Page, tweet: &TweetMetadata) -> Result<()> {
    // Author button selector varies: can be `[data-testid="User-Name"]` following sibling,
    // or author profile page's follow button.
    // Strategy: navigate to author profile? No — we follow inline from tweet context.
    
    // Find follow button near tweet
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
    // Bookmarks have a different UI flow — bookmark modal appears
    // Defer until V2 due to UI volatility and low value 
    bail!("Bookmark action disabled in V1");
}
```

**Verification strategy**: Each action waits 1–2s post-click and re-queries button state (e.g., `like` → `unlike`). If state unchanged, log warning but don't retry (to avoid rate-limit cascades).

---

### `twitter_popup.rs`

**Responsibility**: Dismiss known modal types after every navigation and before interactions.

```rust
/// Known modal/popup types with dismissal selectors.
/// 
/// These appear frequently for logged-out users on x.com:
/// - "Sign up to follow" overlay
/// - "Turn on notifications" prompt
/// - "Cookie consent" banner
/// - "Update your email" nag
/// - "Install app" interstitial
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
    // Iterate through known modal dismiss buttons
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

**Integration points**:
- Call `close_all_modals()` after each navigation, before each interaction, and at start of `run_cycle`.
- Inject CSS overlay blocker at page load via `page.add_style_tag()` to hide non-essential modals (CSS `pointer-events: none` on overlay divs).
- If modal persists after 3 close attempts, log warning and continue (non-fatal).

---

### `twitter_sentiment.rs`

**Responsibility**: Lightweight sentiment detection using keyword blocklist. V1: keyword-only. V2+: integrate lightweight NLP or LLM call.

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

/// Advanced: compute sentiment score (0–1) via naive wordlist (future).
#[allow(dead_code)]
pub fn sentiment_score(_text: &str) -> f64 {
    // Future: AFINN-165 wordlist or vader sentiment mapping
    // V1: No-op, return neutral
    0.5
}
```

**Design notes**:
- Keyword list is case-insensitive substring match (fast, no dependencies).
- List size: ~10–20 words (prevent false positives on neutral words like "hate" in "I hate Mondays" vs "hate speech").
- If blocklist trigger: log the matched keyword and tweet ID for audit.

---

### `twitter_limits.rs`

**Responsibility**: Track per-session engagement counters with hard caps.

```rust
#[derive(Debug, Clone, Copy)]
pub struct EngagementCounters {
    pub likes: u32,
    pub retweets: u32,
    pub follows: u32,
    pub bookmarks: u32,
}

impl EngagementCounters {
    pub fn new() -> Self {
        Self {
            likes: 0,
            retweets: 0,
            follows: 0,
            replies: 0,
            quotes: 0,
            bookmarks: 0,
            skipped_sentiment: 0,
            skipped_limits: 0,
            skipped_dive_decision: 0,
        }
    }
    
    /// Check if action would exceed configured limit.
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
    
    /// Increment counter for action.
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

**Edge case**: If all counters hit limit simultaneously, task may exit early before max_cycles. This is acceptable behavior.

---

### `twitter_persona.rs`

**Responsibility**: Map `BrowserProfile` → `TwitterPersona` and adjust behavior overlays.

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TwitterPersona {
    Efficient,    // fast cursor, low idle, minimal dwell
    Casual,       // average speeds, medium idle, occasional wiggle
    Researcher,   // slow, deliberate, long reads (deep scroll)
    Hesitant,     // high typo-like micro-corrections, overscroll
    Distracted,   // erratic jumps, short attention, frequent idle
    Focused,      // smooth, linear, few pauses
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
            _ => Self::Casual, // default fallback
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
        // Persona affects likelihood of diving into a tweet
        match self {
            Self::Efficient => 1.2,     // more likely to engage
            Self::Casual => 1.0,
            Self::Researcher => 1.5,    // reads deeply
            Self::Hesitant => 0.7,      // skips most
            Self::Distracted => 0.6,    // skips many
            Self::Focused => 1.3,       // focused reading
        }
    }
}
```

**Integration**: `TwitterAgent::new()` calls `TwitterPersona::from_profile(&profile)` and stores multiplier. `should_dive()` applies multiplier to profile's base diveProbability.

---

### `twitter_llm.rs` (future, V2)

```rust
#[allow(dead_code)]
pub enum LLMProvider { VLLM, Ollama, OpenRouter }

/// Generate a reply text using configured LLM provider chain.
#[allow(dead_code)]
pub async fn generate_reply(
    context: &TweetContext,
    persona: &TwitterPersona,
) -> Result<String> {
    // Provider chain: vLLM → Ollama → OpenRouter
    // Build prompt from context + persona instructions
    // Humanize output: lowercase 30%, strip trailing period 80%
    unimplemented!()
}
```

---

### `src/utils/twitter/mod.rs`

```rust
pub mod selectors;
pub mod humanized;

// Re-export for convenience
pub use selectors::*;
pub use humanized::*;
```

### `src/utils/twitter/selectors.rs`

**Centralized selector definitions with fallbacks**. Update this file only when Twitter UI changes.

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

**Fallback strategy**: Each action tries primary selector, then secondary, then XPath alternative. Log which selector succeeded for audit trail.

### `src/utils/twitter/humanized.rs`

Humanization helpers specific to Twitter interaction:

```rust
/// Hover briefly before clicking (simulates visual verification).
pub async fn hover_pause(page: &Page, x: f64, y: f64) -> Result<()> {
    mouse::cursor_move_to(page, x, y).await?;
    human_pause(200, 40).await; // 200ms ±80ms hover
    Ok(())
}

/// Scroll through feed with variable speed.
pub async fn twitter_scroll(page: &Page, amount: Option<i32>) -> Result<()> {
    let scroll_amount = amount.unwrap_or_else(|| {
        // Gaussian around 500px with 30% variance
        gaussian(500.0, 150.0, 200.0, 1200.0) as i32
    });
    
    scroll::scroll_down(page, scroll_amount).await?;
    human_pause(800, 60).await; // pause to scan loaded tweets
    Ok(())
}

/// Idle wiggle — tiny cursor movements simulating inattention.
pub async fn idle_wiggle(page: &Page) -> Result<()> {
    if rand::random::<f64>() < 0.15 { // 15% chance per cycle
        // Move 5–15px in random direction, then back
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

## 8. Monitoring & Metrics

### 8.1 Task-Level Metrics (TwitterActivity-specific)

`TwitterAgent` maintains in-memory counters exported at task completion:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct TwitterMetrics {
    pub session_id: String,
    pub profile_name: String,
    pub total_cycles: u32,
    pub engagement: EngagementCounters,
    pub skipped_by_phase: SkippedCounters,
    pub entry_point_breakdown: HashMap<String, u32>,
    pub errors: Vec<TwitterError>,
    pub total_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EngagementCounters {
    pub likes: u32,
    pub retweets: u32,
    pub follows: u32,
    pub replies: u32,
    pub quotes: u32,
    pub bookmarks: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkippedCounters {
    pub dive_decision: u32,
    pub sentiment_block: u32,
    pub limit_exceeded: u32,
    pub navigation_failed: u32,
    pub context_load_failed: u32,
    pub no_tweets_found: u32,
    pub action_failed: u32,
}
```

### 8.2 Run Summary Extension

The existing `run-summary.json` (from `metrics.rs`) currently exports generic task stats. Enhance it to include Twitter break-down:

**File: `run-summary.json`**

```json
{
  "timestamp": "2026-04-17T00:29:52Z",
  "task": "twitterActivity",
  "session_id": "roxy-0001",
  "profile": "Casual",
  "twitter_persona": "casual",
  "total_duration_ms": 723450,
  "cycles": {
    "attempted": 8,
    "completed": 8,
    "skipped_dive": 2,
    "skipped_sentiment": 0,
    "skipped_limits": 0
  },
  "engagement": {
    "likes": 4,
    "retweets": 2,
    "follows": 1,
    "bookmarks": 0
  },
  "entry_points": {
    "home": 5,
    "explore": 2,
    "notifications": 1
  },
  "errors": [
    {
      "cycle": 3,
      "phase": "navigation",
      "message": "Navigation timeout after 15000ms"
    }
  ],
  "limits_reached": {
    "likes": false,
    "retweets": false,
    "follows": false
  }
}
```

### 8.3 Structured Log Format

All logs use structured logging via `log::info!()` etc. with `LogContext`:

```
HH:MM:SS [session=roxy-0001][profile=Casual][task=twitterActivity][cycle=3/8] 
        INFO  Engagement: like @user123 (duration=234ms)
```

**Log event types** (use appropriate levels):
- `INFO`: cycle start/complete, engagement successful, entry point selected
- `WARN`: navigation failure, modal close fallback, selector miss, action verification failed
- `ERROR`: critical failure (browser crash, config load failure)
- `DEBUG`: persona parameters, dive roll values, sentiment check result

### 8.4 Metrics Export to `run-summary.json`

Extend `TwitterAgent` to export its metrics at task completion, then modify `perform_task` (in `task/mod.rs`) to collect task-specific data:

```rust
// In perform_task wrapper (task/mod.rs: execute_single_attempt)
match name {
    "twitteractivity" => {
        let result = twitteractivity::run(session_id, page, payload.clone(), max_retries).await?;
        // Optional: collect TwitterAgent::metrics() and store in TaskResult::metadata
        //   → implement TaskResult::with_metadata(key/value pairs)
        // For V1: metrics already exported via `metrics.export_summary()` (generic)
        result
    }
    // ...
}
```

**V1 approach** (simple): Have `twitteractivity::run()` log a final summary line with all counters. The orchestrator's existing `metrics.export_summary()` writes generic run stats; Twitter-specific breakdown is recovered from the main log file.

**V2 approach** (enhanced): Extend `TaskResult` struct with `metadata: Option<serde_json::Value>` field. Twitter task populates this with its detailed counters. Orchestrator aggregates all task metadata into a single `run-summary.json` under a `"tasks"` array:

```json
{
  "summary": { "total_tasks": 10, "succeeded": 9, ... },
  "tasks": [
    {
      "name": "twitterActivity",
      "session_id": "roxy-0001",
      "duration_ms": 723450,
      "status": "success",
      "twitter": { ... detailed counters ... }
    }
  ]
}
```

For V1 we keep the existing `MetricsCollector` unchanged. Twitter logs go to the unified log file.

---

## 9. Dependencies & Risks

### 9.1 Dependencies (No new crates expected)

**Reuse existing utilities:**
- `crate::utils::navigation::{goto, wait_for_load}` — navigation with timeout
- `crate::utils::scroll::{random_scroll, scroll_to_bottom}` — feed scrolling
- `crate::utils::mouse::{cursor_move_to, click_at}` — humanized click execution
- `crate::utils::timing::human_pause` — all timing delays
- `crate::utils::page_size::{get_viewport, get_element_center}` — coordinate calculations
- `crate::utils::block_heavy_resources` — block images/videos for performance
- `crate::utils::profile::{BrowserProfile, randomize_profile}` — behavior parameters

**No new third-party dependencies.** All functionality achievable with `chromiumoxide`, `serde_json`, `rand`, `tokio`, `log`, `anyhow` which are already in `Cargo.toml`.

### 9.2 Risk Matrix

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| **Twitter DOM volatility** (selector breakage) | High | Task failure | • 3–4 fallback selectors per element (`selectors.rs`)<br>• Centralized selector definitions (single-file update)<br>• Log which selector succeeded; monitor failure patterns daily |
| **Rate-limit / session ban** | Medium | Task fail, IP/profile ban | • Conservative limits (5L/3RT/2F per session)<br>• Random entry points (no single URL bias)<br>• 9–14min session duration (human-typical)<br>• Idle wiggle + realistic cursor curves |
| **Popup/modal flood** | High | Interaction blocked | • `close_all_modals()` after every navigation<br>• CSS overlay injection (`pointer-events: none`)<br>• ESC key fallback<br>• Log count of modals encountered per session |
| **Slow network / timeout** | Medium | Navigation fails | • 20s per-cycle navigation budget (15+5)<br>• Graceful degradation: if navigation fails → record `NavigationFailed` and continue (not fatal)<br>• Session continues up to max_cycles or duration |
| **Element stale / DOM mutation** | Medium | Click fails | • Re-query element immediately before click (don't cache stale handles)<br>• After scroll/expand, wait 500–1000ms for DOM reflow |
| **"Like" already engaged** | High (if revisiting same tweets) | Duplicate action error | • Pre-check button state: if `[data-testid="unlike"]` visible → skip<br>• Log "already liked" as info, not error |
| **"Follow" already in progress** | Medium | Button shows "Pending" instead of "Follow" | • Check for `[data-testid="following"]` or `[data-testid="pending"]` → skip |
| **Retweet confirmation modal changes** | Medium | RT fails | • Modal confirm button selector has fallback: `[data-testid="retweetConfirm"]` OR `button:has-text("Retweet")`<br>• Verify modal closes; if not → log and continue |
| **Bookmark UI complexities** | High | Skipped V1 | • Bookmark feature disabled V1 (limit = 0, probability = 0)<br>• Revisit V2 after UI stabilizes |

### 9.3 Selector Maintenance Strategy

Centralize all Twitter CSS selectors in `src/utils/twitter/selectors.rs`. Implement a **selector health check** at startup:

```rust
/// Verify critical selectors still match page elements. If not, log warning.
pub async fn validate_selectors(page: &Page) -> Result<()> {
    let critical = [
        (BTN_LIKE, "like button"),
        (BTN_RETWEET, "retweet button"),
        (BTN_FOLLOW, "follow button"),
    ];
    
    for (selector, name) in &critical {
        if page.querySelector(selector).await?.is_none() {
            warn!("Twitter selector health: {name} ({}) not found", selector);
        }
    }
    Ok(())
}
```

**Update cadence**: Manual update to selectors as needed. No auto-discovery (avoids false positives).

### 9.4 Rate Limit & Ban Avoidance

Conservative design choices:
- Max 10 cycles per session → max 5 likes, 3 RTs, 2 follows
- Session duration randomized within 9–14min
- Entry points varied to avoid repetitive pattern
- Human-like cursor Bezier curves (non-linear, variable speed)
- Micro-pauses after hover, before click
- Scroll amounts random (200–1200px) with Gaussian distribution
- Post-action pause 3000±1500ms

These parameters are configurable via TOML and env vars for tuning once live.

### 9.5 Testing Strategy

**Unit tests** (for V1 logic only - no browser):
- `weighted_pick()` correctness
- `engagement_counters.can_engage()` boundary conditions
- `contains_negative_sentiment()` blocklist matching
- `selectors.rs` constants compile and are valid CSS

**Integration tests** (manual + CI if browser available):
- Run `cargo run twitterActivity` on live x.com (no login)
- Verify: log output shows like/retweet/follow actions (check account after)
- Verify: counters never exceed limits
- Verify: no panics on missing elements
- Verify: `run-summary.json` exports

**CI gate**: Run unit tests on PR. Integration requires manual trigger (browser dependency).

---

## 10. Implementation Milestones

### Phase 0 — Foundations (Day 1)
**M0.1: Config Extension**
- [ ] Add `TwitterConfig` struct to `src/config.rs`
- [ ] Add TOML deserialization for `[twitter]` section
- [ ] Add `apply_env_overrides()` handlers for all `TWITTER_*` vars
- [ ] Add `default_twitter_config()` fallback in `load_code_config()`
- [ ] Add `validate_config()` checks for Twitter fields (weights sum ~100, probs ≤ 1.0, limits ≥ 0)
- [ ] Write unit test: `load_config()` + env override integration

**M0.2: Task Registration**
- [ ] Create `src/task/twitteractivity.rs` (stub: `run()` returning `TaskResult::success(0)`)
- [ ] Register in `src/task/mod.rs` (`pub mod twitteractivity`)
- [ ] Add match arm in `perform_task()` (in `task/mod.rs`)
- [ ] Add validation schema in `src/validation/task.rs` (any object OK for V1)
- [ ] Verify task discovered: `cargo run twitterActivity` reaches stub

**M0.3: Profile Extension (Twitter-specific)**
- [ ] Add `dive_probability: ProfileParam` field to `BrowserProfile` in `src/utils/profile.rs`
- [ ] Add `#[serde(default)]` to maintain backward compatibility with existing configs
- [ ] Provide default of `p(0.35, 20.0)` in `BrowserProfile::default()` (or in each preset)
- [ ] Initialize `dive_probability` in all 21 BrowserProfile preset constructors (average(), teen(), senior(), etc.)
- [ ] Unit test: `BrowserProfile::from_preset()` returns profile with non-zero `dive_probability`

---

### Phase 1 — Core Infrastructure (Day 1–2)

**M1: Navigation & Entry Points**
- [ ] Create `src/task/twitter_navigation.rs`
- [ ] Implement `weighted_pick()` utility
- [ ] Define `ENTRY_POINTS` constant with weights per spec
- [ ] Implement `select_entry_point()` (category → concrete URL)
- [ ] Implement `navigate()` with 20s timeout budget
- [ ] Add unit test: `select_entry_point()` returns valid URL for all categories
- [ ] Integration: `TwitterAgent::run_cycle()` Phase 1–2 logs entry URL

**M2: Feed Scanning & Tweet Extraction**
- [ ] Create `src/task/twitter_feed.rs`
- [ ] Implement `find_random_tweet()` using selector family cascade
- [ ] Write `extract_metadata_from_element()` with JS evaluate
- [ ] Add `TweetMetadata` struct (id, author, text_preview, has_media)
- [ ] Create `src/utils/twitter/selectors.rs` with all selectors
- [ ] Integration: verify agent finds at least 1 tweet per cycle (log count)
- [ ] Unit test: mock page DOM → extraction returns expected metadata

**M3: Popup Handler**
- [ ] Create `src/task/twitter_popup.rs`
- [ ] Implement `close_all_modals()` iterating dismiss selectors
- [ ] Add ESC key fallback
- [ ] Inject in `run_cycle()` after navigation arrive
- [ ] Integration: run on x.com — observe any modals dismissed (log each)

---

### Phase 2 — Engagement Actions (Day 2)

**M4: Like Action**
- [ ] Implement `twitter_interact::like_tweet()`
- [ ] Locate `[data-testid="like"]` button
- [ ] Hover → click via `mouse::cursor_move_to` + `click_at`
- [ ] Verify state change (wait up to 2s for `[data-testid="unlike"]`)
- [ ] Add skip-if-already-liked logic
- [ ] Integration test: run 1 cycle → verify like appears on test account (manual check)

**M5: Retweet Action**
- [ ] Implement `twitter_interact::retweet_tweet()`
- [ ] Click retweet → modal → click confirm (native RT, no quote)
- [ ] Verify modal closes (best-effort)
- [ ] Integration: RT a known tweet, verify on account

**M6: Follow Action**
- [ ] Implement `twitter_interact::follow_user()`
- [ ] Find follow button inline or via author profile link
- [ ] Click, verify state → "Following" or "Pending"
- [ ] Skip if already following
- [ ] Integration: follow a test user, verify state change

**M7: Bookmark Stub (disabled V1)**
- [ ] Implement `bookmark_tweet()` returning `Err("disabled in V1")`
- [ ] Ensure config `max_bookmarks=0` blocks this action at limit guard

---

### Phase 3 — Agent Logic & Orchestration (Day 2–3)

**M8: TwitterAgent State Machine**
- [ ] Create `src/task/twitter_agent.rs`
- [ ] Implement struct: `TwitterAgent { page, profile, config, state }`
- [ ] `run_session()`: loop cycles until max_cycles or duration budget
- [ ] `run_cycle()`: full flow (nav → modal close → dive decision → tweet find → context load → sentiment → limit check → action)
- [ ] `select_entry_point()` delegates to navigation module
- [ ] `should_dive()`: `rand::random::<f64>() < profile.dive_probability * persona.multiplier()`
- [ ] `roll_engagement()`: pick action weighted by config probabilities
- [ ] `can_engage()`: consult counters + limits
- [ ] `increment_counter()`: update `EngagementCounters`
- [ ] Phase tracking (warmup/active/cooldown) — used for V2 timing overrides

**M9: Limits & Counters**
- [ ] Create `src/task/twitter_limits.rs` (or inline in agent)
- [ ] `EngagementCounters` struct with per-action u32
- [ ] `can_engage()` checks each action's limit
- [ ] Log when limit hit (info level)

**M10: Sentiment Guard**
- [ ] Create `src/task/twitter_sentiment.rs`
- [ ] `contains_negative_sentiment()`: lowercase substring search against blocklist
- [ ] Config flag: `block_negative_engagement` (default false)
- [ ] If enabled: block action, log matched keyword, record skip

**M11: Persona Mapping**
- [ ] Create `src/task/twitter_persona.rs`
- [ ] `TwitterPersona::from_profile(profile)` heuristic mapping
- [ ] `input_method_weights()` — currently informational (V1 all mouse), V2 for keyboard/wheel mix
- [ ] `dive_probability_multiplier()` adjusts base profile.dive_probability

---

### Phase 4 — Config & Validation (Day 3)

**M12: Config Integration**
- [ ] Extend `Config` struct with `twitter: TwitterConfig`
- [ ] Add `default_twitter_config()` in `config.rs`
- [ ] Merge TOML + env overrides in `apply_env_overrides()`
- [ ] Add validation in `validate_config()`:
  - `max_cycles >= min_cycles`
  - `max_duration_sec >= min_duration_sec`
  - engagement probabilities sum ≤ 1.0 (warning only, not error)
  - entry point weights sum to ~100 (±5 tolerance)

**M13: Validation Schema**
- [ ] Extend `src/validation/task.rs` with `validate_twitteractivity()`
- [ ] V1: accept any JSON object (no required fields)
- [ ] Future: validate optional fields like `cycles=3` or `max_likes=2`
- [ ] Unit test: valid payload accepted, invalid rejected

---

### Phase 5 — Metrics & Logging (Day 3)

**M14: Task Metrics**
- [ ] `TwitterAgent` accumulates `TwitterMetrics`
- [ ] At task end, log final summary:
  ```
  [twitter] Session complete: cycles=8, likes=4, retweets=2, follows=1,
           skipped_dive=2, duration=723s, profile=Casual
  ```
- [ ] Ensure all log lines include `[twitter]` tag for log parsing

**M15: Run Summary Compatibility**
- [ ] Verify `perform_task()` returns `TaskResult::success()` if at least one engagement succeeded
- [ ] If all cycles skipped (no engagements), still return Success (not failure)
- [ ] `run-summary.json` from `MetricsCollector` captures top-level stats

---

### Phase 6 — Integration & Polish (Day 3)

**M16: End-to-End Dry Run**
- [ ] `cargo run twitterActivity` on machine with browser connected
- [ ] Observe log: entry point selection, tweet found, actions executed
- [ ] Verify no panics, no unwrap() crashes
- [ ] Check counters never exceed limits
- [ ] Monitor for modal storms — increase dismiss selector coverage if needed

**M17: Error Path Verification**
- [ ] Simulate network timeout (block x.com) → verify navigation timeout handling
- [ ] Simulate selector rot (rename a selector) → verify graceful skip (no panic)
- [ ] Simulate action failure (element detached) → verify warning logged

**M18: Config Edge Cases**
- [ ] Test `TWITTER_ENABLED=false` → task should exit early with info log
- [ ] Test `TWITTER_MAX_CYCLES=0` → zero cycles, task completes (edge, but valid)
- [ ] Test probabilities sum > 1.0 → warning log, but proceeds (over-limit is OK)

**M19: Documentation & Final Review**
- [ ] Update `README.md` with `twitterActivity` usage example
- [ ] Add sample `config/default.toml` Twitter section to README
- [ ] Document environment variables table
- [ ] Add `twitterActivity.md` → `docs/tasks/twitter.md`
- [ ] Code review checklist: error handling, no unwraps, proper logging

---

### Phase 7 — Deferred (V2, Future)

**V2 Roadmap** (not in initial scope):
- [ ] LLM-powered replies & quote tweets (`twitter_llm.rs`)
- [ ] Sentiment analysis with NLP wordlists (VADER-style)
- [ ] Bookmark action implementation
- [ ] Reply action implementation (keyboard typing simulation)
- [ ] Dynamic entry point weights (per-session randomization ±10%)
- [ ] Advanced persona behaviors: hesitation micro-movements, overscroll, tab-switch simulation
- [ ] `run-summary.json` embedded per-task metadata
- [ ] Dashboard: real-time metrics UI (web sockets)

---

### Estimated Effort

- **Full V1 implementation**: 2–3 days (conservative scope, no LLM)
- **Plus integration testing + tuning**: +1 day
- **Total to MVP**: 3–4 days (assuming single engineer)

---

## 11. Success Criteria (Definition of Done)

V1 is complete when **all** of the following are verified:

**Functional Requirements**
- [ ] `cargo run twitterActivity` executes without panics or unwrap crashes
- [ ] Task completes 5–10 cycles (random within config range)
- [ ] Total session duration falls within 540–840 seconds (9–14min)
- [ ] At least one engagement (like/retweet/follow) occurs per session on average (configurable probability ≥ 0.1)
- [ ] Per-session limits respected: `likes ≤ max_likes`, `retweets ≤ max_retweets`, `follows ≤ max_follows` (hard stops)
- [ ] Sentiment guard (when enabled) blocks engagement on tweets containing blocklist keywords

**Reliability Requirements**
- [ ] Navigation failures (timeout, DNS error) do NOT crash the task — individual cycles are skipped, session continues
- [ ] Missing UI elements (e.g., already-liked tweet) are handled gracefully with info-level logs
- [ ] Modals are dismissed automatically; if a modal persists, task continues without crashing
- [ ] No evidence of `anyhow!` or `panic!` in logs across 10 consecutive runs

**Observability Requirements**
- [ ] Every log line includes `[twitter]` tag (searchable in log files)
- [ ] Per-cycle logs: entry URL, tweet found, action taken (or skip reason), duration
- [ ] Final summary line: `[twitter] Session complete: cycles=X, likes=Y, retweets=Z, follows=W, skipped=N, duration=Ts`
- [ ] `run-summary.json` exports with `task="twitterActivity"` and success status

**Configuration Requirements**
- [ ] All config fields in `[twitter]` section are loaded from `config/default.toml`
- [ ] All `TWITTER_*` environment variables override TOML values
- [ ] Validation passes at startup: weights sum ~100, probabilities ≤ 1.0, non-negative limits
- [ ] Task gracefully handles `TWITTER_ENABLED=false` (early exit with info log)

**Testing Requirements**
- [ ] Unit tests pass: `cargo test`
- [ ] Clippy warnings addressed: `cargo clippy --all-targets --all-features`
- [ ] Build succeeds: `cargo build --all-features`
- [ ] Manual integration: run task on live x.com (public), verify actions appear on test account (or at least logs show engagements attempted)
- [ ] Verify counters reset on subsequent runs (no cross-session state bleed)

**Code Quality Requirements**
- [ ] No `unwrap()` or `expect()` on `Result`/`Option` in production code (only in tests)
- [ ] All public functions have `#[allow(dead_code)]` removed (or justified)
- [ ] Module docs (`//!`) explain purpose
- [ ] Functions have inline comments for non-obvious logic
- [ ] Error types use `anyhow::Result` with context-rich messages

**Deployment Readiness**
- [ ] `README.md` updated with `twitterActivity` usage examples
- [ ] Environment variable table included in docs
- [ ] `config/default.toml` sample includes `[twitter]` section
- [ ] Known limitations documented (no LLM, no bookmarks, no replies)

---

## 12. Rollout & Monitoring Plan

**Phase 1 — Shadow Mode (Week 1)**
- Run task on 1–2 sessions with `max_likes=1`, `max_retweets=0`, `max_follows=0`
- Observe logs: entry point distribution, tweet find success rate, modal frequency
- No actual engagement (limits near zero) → pure dry-run
- Verify: no errors, selector health OK, session duration within budget

**Phase 2 — Conservative Engagement (Week 2)**
- Increase limits to `likes=1`, `retweets=1`, `follows=1`
- Verify actions appear on test account
- Monitor for rate-limit responses (403, 429 HTTP codes in DevTools)
- If any signs of throttling, reduce limits further

**Phase 3 — Full Limits (Week 3)**
- Enable full limits: `likes=5`, `retweets=3`, `follows=2`
- Run 10–20 sessions, collect logs, check `run-summary.json`
- Verify no bans, no CAPTCHAs, UI still navigable

**Phase 4 — Tuning**
- Adjust probabilities if engagement rate too low/high
- Tweak entry point weights if certain pages yield poor dive rates
- Update selectors in `selectors.rs` as Twitter UI evolves

---

## 13. Known Gaps & Future Work

| Feature | Status | Notes |
|---------|--------|-------|
| LLM-powered replies/quotes | Deferred V2 | Requires local/cloud LLM integration, prompt engineering |
| Bookmark action | Deferred V2 | UI involves modal; low engagement value |
| Sentiment analysis (NLP) | Deferred V2 | Keyword-only in V1; consider VADER or transformer in V2 |
| Advanced persona behaviors (hesitation, distraction) | Deferred V2 | Profile already encodes some of this via `dive_probability`; micro-move not yet used |
| Quote Tweet | Deferred V2 | Complex flow (compose modal), low priority |
| Dynamic entry weight jitter | Deferred V2 | Static weights OK for V1 |
| Thread engagement (click "Show more replies") | Deferred V2 | Currently only surface-level tweets |
| Video/audio playback | Blocked via `block_media` | By design to reduce bandwidth/noise |
| Cookie consent handling | Basic in `twitter_popup.rs` | May need expansion if new consent banners appear |

---

## 14. References

- Node.js Reference: `auto-ai/tasks/api-twitterActivity.js`
- Browser automation patterns: `task/cookiebot.rs`, `task/pageview.rs`
- Utilities: `src/utils/{navigation,scroll,mouse,timing,profile,blockmedia}.rs`
- Config system: `src/config.rs`
- Task orchestration: `src/orchestrator.rs`, `src/task/mod.rs`
- Metrics: `src/metrics.rs`
- Validation: `src/validation/task.rs`

---

## 15. Decisions Made (Confirmed)

Based on user feedback (2026-04-17), the following decisions are confirmed:

| # | Decision Topic | Chosen Option | Notes |
|---|----------------|---------------|-------|
| 1 | **LLM Integration** | V1: Exclude replies & quotes (no LLM) | Flags `reply_with_ai=false`, `quote_with_ai=false`. V2 can enable via config |
| 2 | **Sentiment** | Keyword-only, off-by-default | `block_negative_engagement = false`. Simple substring blocklist |
| 3 | **Bookmarks** | Disabled in V1 | `max_bookmarks=0`, `bookmark_probability=0` |
| 4 | **Media Blocking** | Do NOT use `block_heavy_resources` | Twitter needs images for realism; let media load naturally |
| 5 | **Engagement Roll Timing** | After click + simulate reading | Sequence: dive (click) → simulate_reading() → load context → roll → act |
| 6 | **Entry Point Weights** | Use provided weighted list | Home=59%, Explore=32%, Connect=4%, Supplementary=5% |
| 7 | **Entry Point Dynamism** | Static weights (no jitter) | Same distribution every cycle; V2 may add jitter |
| 8 | **Rate Limits** | Likes=5, Retweets=3, Follows=2 | Conservative caps; configurable via env |
| 9 | **Sentiment Blocklist** | ~10–15 conservative keywords | Avoid false positives; expand after manual review |
| 10 | **Validation Schema** | Simple object check (any JSON) | No required fields; extensible for V2 |
| 11 | **Fallback Selector Order** | First-match cascade | Try article→cellInnerDiv→fallback→XPath |
| 12 | **Action Failure Handling** | Skip silently, continue | No retry; log warning, proceed to next cycle |
| 13 | **AI Reply/Quote Flags** | `reply_with_ai = false`, `quote_with_ai = false` | Default off; require explicit config to enable (V2) |
| 14 | **Persona Support** | Extend `BrowserProfile` with `dive_probability` | Base engage rate per profile; persona provides multiplier |

**Implementation implications:**
- `Engagement` enum includes Reply/Quote but they are unreachable unless flags set true
- `simulate_reading()` phase added after tweet click, before context extraction
- No call to `block_heavy_resources()` in Twitter task
- `TwitterAgent::persona` derived from profile; `dive_probability` field added to all presets

---

**Remaining open items** (non-blocking):

| Item | Owner | Notes |
|------|-------|-------|
| Add `dive_probability` to all 21 `BrowserProfile` presets | Engineering | Set `p(0.35, 20.0)` or per-preset tuning |
| Create `src/utils/twitter/` module with `selectors.rs` and `humanized.rs` | Engineering | Centralize Twitter-specific selectors |
| Populate initial negative keyword blocklist (15 words) | Product | Provide seed list; expand based on logs |
| Define per-preset `dive_probability` values (if not uniform) | Research | Tune after observing engagement rates |
| Deploy selector health check script (optional) | Ops | Periodic validation of critical selectors |

---

*Document version: 0.2 — Implementation-ready planning draft.*
