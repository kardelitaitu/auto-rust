# Twitter Activity Task

last audited 16-06-26 by opencode

Simulates human-like Twitter/X engagement with persona-based behavior.

## Quick Start

```bash
# Run with default persona
cargo run twitteractivity

# Run with custom duration and engagement limits
cargo run twitteractivity,duration_ms=120000,scroll_count=12

# Run with custom persona weights
cargo run 'twitteractivity,weights={"like_prob":0.4,"retweet_prob":0.15,"follow_prob":0.05}'
```

## Features

- 🎭 **Persona-Based Behavior**: 21 preset personas
- 🧠 **Smart Decisions**: AI-powered engagement decisions (7.1 unified engine)
- ❤️ **Like Tweets**: Human-like cursor movement and timing
- 🔁 **Retweet**: Native retweets with modal confirmation
- 👤 **Follow Users**: From tweet context or profile pages
- 💬 **Reply**: Context-aware reply composition with LLM
- 🧵 **Thread Dives**: Read full conversation threads (no caching, fresh context)
- 🔖 **Bookmark**: Save tweets (config-driven)
- 🤖 **LLM Integration**: AI-generated replies and quotes
- 🎯 **Enhanced Sentiment**: Multi-layer sentiment analysis
- 🔄 **Error Recovery**: Retry logic with exponential backoff

## Engagement Limits (Default)

| Action | Limit | Configurable |
|--------|-------|--------------|
| Likes | 5 | `TWITTER_MAX_LIKES` |
| Retweets | 3 | `TWITTER_MAX_RETWEETS` |
| Follows | 2 | `TWITTER_MAX_FOLLOWS` |
| Replies | 1 | `TWITTER_MAX_REPLIES` |
| Thread Dives | 3 | `TWITTER_MAX_THREAD_DIVES` |
| Quote Tweets | 2 | `TWITTER_MAX_QUOTE_TWEETS` |
| Bookmarks | 0 (V1 disabled) | `TWITTER_MAX_BOOKMARKS` |
| **Total** | **10** | `TWITTER_MAX_TOTAL_ACTIONS` |

> **Note:** `EngagementLimits::available_actions()` returns action strings `"dive"`,
> `"quote"`, `"like"`, `"retweet"`, `"follow"`, `"reply"`, `"bookmark"` — these match
> the keys used by `SessionState::is_action_allowed()` and `EngagementCounters::increment()`.

## Configuration

```toml
[twitter_activity]
feed_scan_duration_ms = 120000    # 2 minutes
feed_scroll_count = 12             # Scroll actions
engagement_candidate_count = 5     # Tweets to consider

[twitter_activity.engagement_limits]
max_likes = 5
max_retweets = 3
max_follows = 2
max_replies = 1
max_quote_tweets = 2
max_thread_dives = 3
max_bookmarks = 0                  # Disabled in V1 (code default is 2, default.toml overrides to 0)
max_total_actions = 10

# Enables reply/quote text generation. Provider settings come from
# config/llm.toml and LLM_* / OPENROUTER_* / NVIDIA_* environment variables.
[twitter_activity.llm]
enabled = false
```

Smart-decision flags are task payload fields, not TOML fields. The strategy
depends on the `llm_enabled` flag:
- **`llm_enabled: false`** (default): Uses `LegacyStrategy` — keyword-based heuristic
  scoring (controversial topics, spam patterns, reply analysis, media detection)
- **`llm_enabled: true`**: Uses `Auto` strategy — `UnifiedStrategy` with `PersonaStrategy`
  fallback, via DashScope/Qwen API (`DASHSCOPE_API_KEY` or `QWEN_API_KEY`)

When both `smart_decision_enabled: true` and `llm_enabled: true` are set, decision scoring
reads `DASHSCOPE_API_KEY` or `QWEN_API_KEY`. Reply and quote generation still use the
general app LLM config.

## Payload Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `duration_ms` | u64 | 120000 | Session duration |
| `scroll_count` | u32 | 12 | Scroll actions |
| `candidate_count` | u32 | 5 | Engagement candidates |
| `weights` | object | persona | Engagement probabilities |
| `profile` | string | Average | Persona preset |
| `smart_decision_enabled` | bool | false | Enable AI-powered engagement decisions |
| `llm_enabled` | bool | config | Enable LLM decision mode and reply/quote text generation |
| `enhanced_sentiment_enabled` | bool | false | Enable multi-layer sentiment analysis |
| `dry_run_actions` | bool | false | Simulate actions without executing |

## Persona Presets

`Average`, `Teen`, `Senior`, `Enthusiast`, `PowerUser`, `Cautious`, `Impatient`, `Erratic`, `Researcher`, `Casual`, `Professional`, `Novice`, `Expert`, `Distracted`, `Focused`, `Analytical`, `QuickScanner`, `Thorough`, `Adaptive`, `Stressed`, `Leisure`

## How It Works

1. Navigates to Twitter/X home feed
2. Scrolls through feed (respecting `scroll_count`)
3. Identifies candidate tweets for engagement
4. Applies persona-based action selection and optional smart-decision gating
5. Executes engagements (like, retweet, follow, reply)
6. Respects all engagement limits
7. Optionally dives into threads for context
8. Keeps X/Twitter selectors scoped to the active tweet or captured container, not page-wide button scans

## Twitter Utility Modules

The implementation lives in `src/utils/twitter/` and is split into focused modules with rustdoc coverage.

**Core Engagement:**
- `twitteractivity_engagement.rs`: Main engagement logic, `process_candidate()`, action orchestration
- `twitteractivity_feed.rs`: Feed scrolling, candidate identification, and progress tracking
- `twitteractivity_dive.rs`: Thread diving (`dive_into_thread()`) and reading
- `twitteractivity_interact.rs`: DOM interaction actions (like, retweet, follow, reply, bookmark)

**Decision & Strategy:**
- `decision/types.rs`: `TweetContext`, `EngagementDecision`, `EngagementLevel` types
- `decision/engine.rs`: `UnifiedEngine`, `DecisionEngineFactory`
- `decision/strategies/`: `legacy.rs` (keyword-based), `persona.rs` (probabilistic),
  `llm.rs` (LLM-powered), `hybrid.rs` (weighted combo), `unified.rs` (smart fallback)

Smart decisions gate the persona-selected action set by engagement level:
`Full` keeps all selected actions, `Medium` keeps like/retweet, `Minimal` keeps
like only, and `None` skips engagement.
When `llm_enabled: false` (default), uses `LegacyStrategy` — keyword-based heuristic.
When `llm_enabled: true`, uses `Auto` strategy — `UnifiedStrategy` with `PersonaStrategy`
fallback, via DashScope/Qwen API.

**LLM Integration:**
- `twitteractivity_llm.rs`: LLM-powered reply/quote generation (`generate_reply()`, `generate_quote_commentary()`)
- `twitteractivity_llm_execute.rs`: Quote tweet DOM interaction flow
- `twitteractivity_llm_validation.rs`: LLM output sanitization and banned word filtering

**Sentiment Analysis:**
- `sentiment/analyzer.rs`: `SentimentAnalyzer`, multi-strategy pipeline
- `sentiment/utils.rs`: Helper functions
- `sentiment/strategies/`: `emoji.rs`, `domain.rs`, `llm.rs` (per-strategy implementations)

**State & Configuration:**
- `twitteractivity_state.rs`: `TaskConfig`, `SessionState`, `CandidateContext`, `CandidateResult`
- `twitteractivity_constants.rs`: Timing constants
- `twitteractivity_limits.rs`: `EngagementCounters`, `EngagementLimits`, `available_actions()`
- `twitteractivity_persona.rs`: `PersonaWeights`, `select_persona_weights()`

**Infrastructure:**
- `twitteractivity_navigation.rs`: Page navigation and entry points
- `twitteractivity_selectors.rs`: DOM selectors and CSS generators
- `twitteractivity_humanized.rs`: Human-like timing and cursor movements
- `twitteractivity_popup.rs`: Popup/modal handling
- `twitteractivity_retry.rs`: Retry logic with exponential backoff, `CircuitBreaker`
- `twitteractivity_errors.rs`: Error classification and recovery (`ErrorClassifier`)
- `twitteractivity_cookiebot.rs`: Cookie consent automation

**Documentation:**
All functions include detailed rustdoc with Arguments, Returns, Errors, Behavior, and Selectors sections. Generate with `cargo doc --all-features`.

## Related Tasks

- `twitterdive` - Thread diving and reading
- [`twitterfollow`](twitterfollow.md) - Profile following
- `twitterintent` - Intent-based actions
- `twitterlike` - Like specific tweets
- `twitterquote` - Quote tweets with LLM
- [`twitterreply`](twitterreply.md) - Tweet replies
- `twitterretweet` - Retweet specific tweets
