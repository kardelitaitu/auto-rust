# Twitter Activity Task

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
| **Total** | **10** | `TWITTER_MAX_TOTAL_ACTIONS` |

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
max_thread_dives = 3
max_bookmarks = 0                  # Disabled in V1
max_total_actions = 10

# LLM Configuration (for smart replies & quote tweets)
[twitter_activity.llm]
enabled = false                    # Set true for AI-powered features
provider = "ollama"                # Options: ollama, openrouter
model = "llama3.2:latest"

# Smart Decision Engine (7.1 feature)
smart_decision_enabled = false     # AI-powered engagement decisions
enhanced_sentiment_enabled = false # Multi-layer sentiment analysis
dry_run_actions = false            # Simulate actions without executing
```

## Payload Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `duration_ms` | u64 | 120000 | Session duration |
| `scroll_count` | u32 | 12 | Scroll actions |
| `candidate_count` | u32 | 5 | Engagement candidates |
| `weights` | object | persona | Engagement probabilities |
| `profile` | string | Average | Persona preset |
| `smart_decision_enabled` | bool | false | Enable AI-powered engagement decisions |
| `enhanced_sentiment_enabled` | bool | false | Enable multi-layer sentiment analysis |
| `dry_run_actions` | bool | false | Simulate actions without executing |

## Persona Presets

`Average`, `Teen`, `Senior`, `Enthusiast`, `PowerUser`, `Cautious`, `Impatient`, `Erratic`, `Researcher`, `Casual`, `Professional`, `Novice`, `Expert`, `Distracted`, `Focused`, `Analytical`, `QuickScanner`, `Thorough`, `Adaptive`, `Stressed`, `Leisure`

## How It Works

1. Navigates to Twitter/X home feed
2. Scrolls through feed (respecting `scroll_count`)
3. Identifies candidate tweets for engagement
4. Applies persona-based decision logic
5. Executes engagements (like, retweet, follow, reply)
6. Respects all engagement limits
7. Optionally dives into threads for context

## Related Tasks

- [`twitterdive`](twitterdive.md) - Thread diving and reading
- [`twitterfollow`](twitterfollow.md) - Profile following
- [`twitterintent`](twitterintent.md) - Intent-based actions
- [`twitterlike`](twitterlike.md) - Like specific tweets
- [`twitterquote`](twitterquote.md) - Quote tweets with LLM
- [`twitterreply`](twitterreply.md) - Tweet replies
- [`twitterretweet`](twitterretweet.md) - Retweet specific tweets
