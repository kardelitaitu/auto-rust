# Twitter/X Activity Automation Task

This document describes the `twitteractivity` task implementation in `src/task/twitteractivity.rs`.

## Overview

The `twitteractivity` task simulates **human-like engagement** on Twitter/X timelines. It navigates through the platform using weighted entry points, discovers tweets, and performs organic engagement actions (like, retweet, follow, reply, quote, bookmark, thread dive) based on configurable probability weights and engagement limits.

**Key Characteristics:**
- Duration-based execution (default: 5 minutes, configurable)
- Profile-based persona system for interaction probabilities
- Engagement rate limiting per session
- Thread diving for deep engagement
- LLM-powered smart decisions (optional)
- Sentiment analysis for tweet filtering (optional)

## Purpose

**Primary Goal:** Create authentic-looking Twitter/X engagement that mimics real user behavior patterns.

**Use Cases:**
- Automated account warmup and maintenance
- Content discovery and curation
- Network building through organic interactions
- Research data collection (with appropriate rate limits)

**Design Philosophy:**
- **Probabilistic engagement** - Not every tweet gets actioned
- **Weighted randomness** - Entry points and actions use probability distributions
- **Rate limiting** - Hard caps prevent runaway engagement
- **Human simulation** - Reading pauses, scroll patterns, action delays

## Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    twitteractivity::run()                         │
│                    (Entry Point with Timeout)                     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  run_inner() - Main Execution                     │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │ Phase 1: Navigation                                          │ │
│  │ • Select weighted entry point (59% home, 41% distributed)    │ │
│  │ • Navigate to URL                                            │ │
│  │ • Simulate reading if not on home (10-20s scroll)           │ │
│  │ • Navigate to home feed                                        │ │
│  └─────────────────────────────────────────────────────────────┘ │
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    MAIN LOOP (Duration-based)                     │
│              Runs until deadline or all limits hit                │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │ Phase 2: Feed Scanning                                         │ │
│  │ • Scroll feed with human-like variance                       │ │
│  │ • Identify engagement candidates (visible tweets)          │ │
│  │ • Rate limit: MIN_CANDIDATE_SCAN_INTERVAL_MS (2.5s)          │ │
│  └─────────────────────────────────────────────────────────────┘ │
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│              Phase 3: Candidate Processing                       │
│                                                                   │
│  For each candidate tweet (up to candidate_count):               │
│  (Candidates are already in viewport — no scrolling needed)      │
│                                                                   │
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │ 3b. Smart Decision (Optional)                                │ │
│  │     • If enabled: Analyze tweet with LLM/ML                │ │
│  │     • Score tweet quality (0-100)                          │ │
│  │     • Apply interest multiplier                            │ │
│  └─────────────────────────────────────────────────────────────┘ │
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │ 3c. Action Selection                                         │ │
│  │     • Check limits (global + per-type caps)                  │ │
│  │     • Check action tracker cooldown (3s min between)         │ │
│  │     • Apply persona probabilities:                         │ │
│  │       - like_prob, retweet_prob, follow_prob                 │ │
│  │       - reply_prob, quote_prob, bookmark_prob                │ │
│  │       - thread_dive_prob                                   │ │
│  └─────────────────────────────────────────────────────────────┘ │
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │ 3d. Execute Actions                                          │ │
│  │     • Perform selected engagement(s)                         │ │
│  │     • Record in action tracker                               │ │
│  │     • Update counters                                        │ │
│  │     • Increment metrics                                      │ │
│  └─────────────────────────────────────────────────────────────┘ │
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │ 3e. Thread Dive (Optional)                                   │ │
│  │     • Open tweet detail view                                 │ │
│  │     • Read thread with scrolling                             │ │
│  │     • Engage with replies (limited by thread_depth)          │ │
│  │     • Return to feed                                         │ │
│  └─────────────────────────────────────────────────────────────┘ │
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   (Loop continues until deadline)                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                Phase 4: Cleanup & Summary                        │
│                                                                   │
│  • Log engagement counters (likes, retweets, follows, etc.)     │
│  • Calculate success rates                                        │
│  • Report remaining time vs limits                                │
└─────────────────────────────────────────────────────────────────┘
```

## Implementation Details

### 1. Entry Point Selection

**File:** `select_entry_point()` in `twitteractivity_navigation.rs`

Weighted random selection from 15 possible entry points:

| Entry Point | Weight | Probability |
|-------------|--------|-------------|
| `https://x.com/` (Home) | 59 | 59.6% |
| Global Trending | 4 | 4.0% |
| Explore | 4 | 4.0% |
| For You | 4 | 4.0% |
| Trending | 4 | 4.0% |
| Bookmarks | 4 | 4.0% |
| Notifications | 4 | 4.0% |
| Mentions | 4 | 4.0% |
| Chat | 4 | 4.0% |
| Connect People | 2 | 2.0% |
| Connect (Creator) | 2 | 2.0% |
| News | 1 | 1.0% |
| Sports | 1 | 1.0% |
| Entertainment | 1 | 1.0% |
| For You (alt) | 1 | 1.0% |

**Total Weight:** 100

### 2. Navigation and Reading Simulation

**Function:** `navigate_and_read()` in `twitteractivity_navigation.rs`

When navigating to a non-home entry point:
1. Navigate to selected URL (60s timeout)
2. Pause 2 seconds for page load
3. If not on home feed:
   - Scroll for 10-20 seconds (random duration)
   - Scroll amount: configurable via `scroll_amount_pixels` or profile default (typically 500-1500px)
   - Uses profile-based scroll configuration
4. Navigate to home feed afterward

This simulates a real user checking notifications/explore, then returning to their main feed.

### 3. Candidate Identification

**Function:** `identify_engagement_candidates()` (from `twitteractivity_feed`)

Scans the current viewport for tweet elements using selectors:
- Primary: `article[data-testid="tweet"]`
- Fallbacks for various Twitter UI versions

Returns a list of tweet JSON objects containing:
- Tweet ID
- Text content (for sentiment analysis)
- User info
- Status URL

### 4. Engagement Limits System

**Struct:** `EngagementLimits`

Hard caps per session to prevent runaway engagement:

```rust
pub struct EngagementLimits {
    pub max_likes: u32,           // Default: 5
    pub max_retweets: u32,        // Default: 3
    pub max_follows: u32,         // Default: 2
    pub max_replies: u32,         // Default: 1
    pub max_quote_tweets: u32,    // Default: 2  (NOTE: `max_quotes` is not a field — use `max_quote_tweets`)
    pub max_bookmarks: u32,       // Default: 2 (default.toml overrides to 0 — "Disabled in V1")
    pub max_thread_dives: u32,    // Default: 3
    pub max_total_actions: u32,   // Default: 10  (NOTE: `max_actions_total` is not a field — use `max_total_actions`)
}
```

**Checked before every action.**

> **Note:** `EngagementLimits::available_actions()` returns action strings `"dive"`,
> `"quote"`, `"like"`, `"retweet"`, `"follow"`, `"reply"`, `"bookmark"` — these match
> the keys used by `SessionState::is_action_allowed()` and `EngagementCounters::increment()`.

### 5. Persona-Based Action Selection

**Struct:** `PersonaWeights`

Probability thresholds for each action type:

```rust
pub struct PersonaWeights {
    pub like_prob: f64,         // 0.0 - 1.0
    pub retweet_prob: f64,
    pub follow_prob: f64,
    pub reply_prob: f64,
    pub quote_prob: f64,
    pub bookmark_prob: f64,
    pub thread_dive_prob: f64,
    pub interest_multiplier: f64, // Modifies base probabilities
}
```

**Persona Presets (21):**
`Average`, `Teen`, `Senior`, `Enthusiast`, `PowerUser`, `Cautious`, `Impatient`,
`Erratic`, `Researcher`, `Casual`, `Professional`, `Novice`, `Expert`, `Distracted`,
`Focused`, `Analytical`, `QuickScanner`, `Thorough`, `Adaptive`, `Stressed`, `Leisure`

Personas are loaded from `BrowserProfile` — they control scroll behavior, timing
variance, and engagement probabilities. See `src/utils/profile.md` for details.

Note: The 4-hardcoded-persona model (`Passive`, `Casual`, `Engaged`, `PowerUser`) no
longer exists. Use the BrowserProfile-based presets above instead.

### 6. Action Tracker (Anti-Detection)

**Struct:** `TweetActionTracker`

Prevents rapid-fire actions on the same tweet:
- Minimum 3-second delay between different action types on same tweet
- Tracks `(tweet_id, action_type, timestamp)`
- Prevents patterns like: like→retweet→reply in 500ms

### 7. Thread Diving

**Function:** `dive_into_thread()` in `twitteractivity_dive.rs`

Deep engagement on individual tweets:
1. Click tweet to open detail view
2. Read original tweet (hover, pause)
3. Scroll through replies (configurable depth)
4. Engage with replies (like, reply)
5. Return to main feed

No thread caching — each dive starts with fresh context to prevent cross-tweet
contamination.

## Engagement Types and Mechanism

### Action Execution Flow

```
select_entry_point() ──► navigate_and_read() ──► is_on_home_feed()
                                                        │
                                                        ▼
                              ┌─────────────────────────────────────────┐
                              │         MAIN SCAN LOOP                  │
                              │  (until deadline or limits exhausted)   │
                              └─────────────────────────────────────────┘
                                          │
                                          ▼
                         identify_engagement_candidates()
                                          │
                                          ▼
                              ┌──────────────────────────┐
                              │   For Each Candidate:    │
                              │                          │
                              │  1. Modulate persona by  │
                              │     sentiment analysis   │
                              │  2. Smart decision (opt) │
                              │  3. Select actions by    │
                              │     persona probabilities│
                              │  4. Check limits         │
                              │  5. Dive into thread?    │
                              │  6. Execute action(s)    │
                              │  7. Engage replies       │
                              │  8. Return to home feed  │
                              └──────────────────────────┘

> Note: Candidates are already in viewport — no scrolling to individual
> tweets occurs. The flow above reflects the actual `process_candidate()`
> implementation in `twitteractivity_engagement.rs`.
```

### Engagement Actions

| Action | Probability | Limit | Mechanism |
|--------|-------------|-------|-----------|
| **Like** | `like_prob` | max_likes | Click like button with hover |
| **Retweet** | `retweet_prob` | max_retweets | Click retweet → confirm |
| **Follow** | `follow_prob` | max_follows | Click follow on tweet author |
| **Reply** | `reply_prob` | max_replies | Open reply box → type → submit |
| **Quote** | `quote_prob` | max_quote_tweets | Retweet with comment |
| **Bookmark** | `bookmark_prob` | max_bookmarks | Click bookmark button |
| **Thread Dive** | `thread_dive_prob` | max_thread_dives | Open tweet → read replies |

### Smart Decision System (Optional)

**Enabled by:** `smart_decision_enabled: true`

When enabled, the decision engine gates persona-selected actions by engagement level.
The strategy depends on the `llm_enabled` flag:
- **`llm_enabled: false`** (default): Uses `LegacyStrategy` — keyword-based heuristic
  scoring (controversial topics, spam patterns, reply analysis, media detection)
- **`llm_enabled: true`**: Uses `Auto` strategy — `UnifiedStrategy` with `PersonaStrategy`
  fallback, via DashScope/Qwen API

Decision levels narrow persona-selected actions:
- `Full`: keep all selected actions
- `Medium`: keep like and retweet
- `Minimal`: keep like only
- `None`: skip engagement

**Fallback:** If smart decision unavailable, uses base persona probabilities.

## Entropy and Randomization

The task uses multiple sources of entropy to appear human:

### 1. Timing Randomization

| Component | Randomization |
|-----------|---------------|
| Entry point | Weighted random (15 options) |
| Reading duration | 10-20 seconds uniform |
| Scroll amount | Configurable via `scroll_amount_pixels` or profile default (typically 500-1500px) |
| Candidate scan interval | 2.5s minimum + processing time |
| Action chain delay | 3s minimum between same-tweet actions |

### 2. Engagement Randomization

| Component | Mechanism |
|-----------|-----------|
| Action selection | Persona-weighted probability |
| Multiple actions | Random subset of enabled actions |
| Thread dive depth | Configurable `thread_depth` |

### 3. Scroll Patterns

Uses `scroll_read()` with profile-based configuration:
- Smooth scrolling enabled/disabled
- Back-scroll (reverse scroll) probability
- Variable scroll distances

### 4. Hover Behaviors

Before clicking:
- Hover over element for random duration
- Cursor movement simulation (via `hover_before_click`)
- Post-click settle pause

## Anti-Detection Measures

### Rate Limiting
- **Per-type limits:** Separate caps for likes, retweets, etc.
- **Total action cap:** Global limit across all types
- **Time-based:** Minimum intervals between scans

### Action Cooldowns
- **Tweet-level:** 3s minimum between actions on same tweet
- **Session-level:** Engagement limits prevent burst patterns

### Entry Point Distribution
- 15 different starting URLs
- Weighted toward home (59%) but includes exploration
- Mimics real user navigation patterns

### Human-Like Pauses
- Page load waits (2s)
- Reading simulation (10-20s)
- Hover delays before clicks
- Scroll pauses between movements

## Metrics and Observability

The task emits metrics via `RunCounters` and logs summary at task end:

```
[twitter] Engagement summary | likes=5 retweets=2 follows=1 ... duration=120.3s
[twitter] Remaining limits | likes=0 retweets=1 follows=4 ...
```

Per-action success/failure is tracked through run counters (`src/metrics.rs`):
- `RUN_COUNTER_CANDIDATE_SCANNED` — total candidates identified
- `RUN_COUNTER_LIKE_SUCCESS` / `RUN_COUNTER_LIKE_FAILURE`
- `RUN_COUNTER_RETWEET_SUCCESS` / `RUN_COUNTER_RETWEET_FAILURE`
- `RUN_COUNTER_FOLLOW_SUCCESS` / `RUN_COUNTER_FOLLOW_FAILURE`
- `RUN_COUNTER_REPLY_SUCCESS` / `RUN_COUNTER_REPLY_FAILURE`
- `RUN_COUNTER_BOOKMARK_SUCCESS` / `RUN_COUNTER_BOOKMARK_FAILURE`
- `RUN_COUNTER_QUOTE_SUCCESS` / `RUN_COUNTER_QUOTE_FAILURE`
- `RUN_COUNTER_DIVE_SUCCESS` / `RUN_COUNTER_DIVE_FAILURE`
- `RUN_COUNTER_BUTTON_MISSING` / `RUN_COUNTER_CLICK_VERIFY_FAILED`
- `RUN_COUNTER_DIVE_TARGET_FALLBACK_USED`

See `TwitterActivityRunCounters` struct in `src/metrics.rs` for the full set of 18 fields.

## Payload Configuration

```json
{
  "duration_ms": 300000,
  "candidate_count": 5,
  "thread_depth": 3,
  "max_actions_per_scan": 3,
  "profile": "Casual",
  "weights": {
    "like_prob": 0.4,
    "retweet_prob": 0.15,
    "follow_prob": 0.05,
    "reply_prob": 0.02,
    "quote_prob": 0.03,
    "bookmark_prob": 0.01,
    "thread_dive_prob": 0.25
  },
  "smart_decision_enabled": false,
  "llm_enabled": false,
  "enhanced_sentiment_enabled": false,
  "dry_run_actions": false
}
```

## Notes

### Thread Cache Isolation
Each tweet starts with fresh cache (`None`) to prevent cross-tweet contamination. The dive cache is only used within a single tweet's processing and discarded afterward.

### Graceful Degradation
- If smart decision fails → falls back to persona probabilities
- If sentiment analysis fails → uses neutral sentiment
- If limits reached → continues scanning without engaging
- If timeout → exits cleanly with summary

### Error Handling
Recoverable errors (element not found, timeout) are logged and continue. Unrecoverable errors propagate up and end the task.

### Testing
The module includes comprehensive unit tests for:
- Entry point selection weight distribution
- Action tracker cooldown enforcement
- Limit checking logic
- Candidate action selection flows
