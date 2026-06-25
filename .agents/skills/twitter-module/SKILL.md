# twitter-module

Expert skill for **working with the Twitter/X automation subsystem** in auto-rust. The Twitter module is the largest subsystem (~27+ modules across `src/utils/twitter/`, 9 task files, 26 JS selector files, sentiment analysis, decision engine, state machine). It is extremely easy to introduce bugs due to the interconnected nature of the modules.

## When to use

- User says "add a new engagement action" or "fix the like/follow/reply flow"
- User wants to understand how Twitter feed scanning and candidate selection works
- User needs to modify the state machine, selectors, or retry logic
- User is debugging a Twitter task failure
- User wants to add LLM-powered reply/quote generation
- User needs to understand engagement limits and persona weights

## 1. Architecture overview

The Twitter pipeline follows this flow:

```
                ┌──────────────────────┐
                │  TwitterActivity task │
                │  (src/task/twitteractivity.rs)
                └──────────┬───────────┘
                           │
                ┌──────────▼───────────┐
                │  Phase 1: Navigation │
                │  twitteractivity_nav │
                │  - Entry point (15   │
                │    weighted URLs)    │
                │  - Dismiss popups    │
                │  - Verify login      │
                └──────────┬───────────┘
                           │
                ┌──────────▼───────────┐
                │  Phase 2: Feed Scan  │
                │  twitteractivity_feed│
                │  - Scroll & stabilize│
                │  - identify_candidates│
                │  - filter_candidates │
                └──────────┬───────────┘
                           │
                ┌──────────▼───────────┐
                │  Phase 3: Engage     │
                │  engagement/mod.rs   │
                │  - process_candidate │
                │  - Sentiment analysis│
                │  - Decision engine   │
                │  - Dispatch action   │
                └──────────┬───────────┘
                           │
                ┌──────────▼───────────┐
                │  Phase 4: Verify     │
                │  - Button state check│
                │  - EngagementOutcome │
                │  - Retry on failure  │
                │  - Record + continue │
                └──────────────────────┘
```

## 2. Module map — all 27+ Twitter utilities

### Core modules (`src/utils/twitter/`)

| Module | File | Purpose |
|---|---|---|
| **Types** | `twitteractivity_types.rs` | `TweetId`, `StatusUrl`, `ComposerFlow` state machine, `EngagementOutcome`, `FollowOutcome`, `PostOutcome` |
| **Constants** | `twitteractivity_constants.rs` | `DEFAULT_TWITTERACTIVITY_DURATION_MS`, scan intervals, scroll limits |
| **Feed** | `twitteractivity_feed.rs` | `identify_engagement_candidates()`, `filter_candidates()`, `is_following_user_at_position()`, scroll stability |
| **Navigation** | `twitteractivity_navigation.rs` | `goto_home()`, `goto_notifications()`, `verify_login()`, `is_feed_visible()`, 15 weighted entry points, `phase1_navigation()` |
| **Interaction** | `twitteractivity_interact.rs` | `like_tweet()`, `retweet_tweet()`, `send_reply()`, `reply_to_tweet()`, `follow_from_tweet()`, `bookmark_tweet()` |
| **Actions** | `twitteractivity_actions.rs` | Position-based clicks: `like_at_position()`, `retweet_at_position()`, `follow_at_position()`, `bookmark_at_position()`, `extract_tweet_text()`, template-based reply/quote generation |
| **Retry** | `twitteractivity_retry.rs` | `RetryConfig` (default/conservative/aggressive), `CircuitBreaker`, `retry_with_backoff()`, `calculate_delay()` |
| **Limits** | `twitteractivity_limits.rs` | `EngagementCounters` (7 action types), `EngagementLimits`, `can_*()` methods, `remaining()`, `available_actions()` |
| **Persona** | `twitteractivity_persona.rs` | `PersonaWeights` (7 probabilities + interest multiplier), `select_persona_weights()`, `apply_behavior_profile()`, `should_*()` decision functions |
| **LLM** | `twitteractivity_llm.rs` | `generate_reply()`, `generate_quote_commentary()`, `extract_tweet_context()`, `llm_instance()` singleton |
| **LLM Execute** | `twitteractivity_llm_execute.rs` | `quote_tweet()` — actual browser quote execution |
| **LLM Validation** | `twitteractivity_llm_validation.rs` | `validate_reply()` — sanitize LLM output |
| **State** | `twitteractivity_state.rs` | Re-exports from `state/` submodule: `SessionState`, `TaskConfig`, `CandidateContext`, `CandidateResult`, `TweetActionTracker`, `SentimentTemplates`, `RateLimitBackoff` |
| **Selectors** | `twitteractivity_selectors.rs` | Pure-function JS string builders for all Twitter DOM selectors |
| **Humanized** | `twitteractivity_humanized.rs` | `human_pause()`, `clustered_engagement_pause()`, `after_navigation_pause()`, `scroll_pause()`, timing utilities |
| **Helpers** | `twitteractivity_helpers.rs` | `selected_candidate_actions()`, `filter_actions_for_decision_level()`, `action_allowed_by_limits()`, `should_navigate_home_after_dive()` |
| **Popup** | `twitteractivity_popup.rs` | `close_active_popup()`, `dismiss_cookie_banner()` |
| **Dive** | `twitteractivity_dive.rs` | `dive_into_thread()`, `identify_thread_replies()`, `get_thread_depth()` |
| **Persistence** | `twitteractivity_persistence.rs` | Session state save/load from disk |
| **Simulation** | `twitteractivity_simulation.rs` | Feed simulation for testing without a real browser |
| **Errors** | `twitteractivity_errors.rs` | `ErrorClass` (Transient/Permanent/Fatal), `ErrorClassifier` trait |

### Submodule modules

| Module | Path | Purpose |
|---|---|---|
| **Engagement** | `engagement/mod.rs` | `process_candidate()` — the main engagement orchestration |
| **Engagement Dispatch** | `engagement/dispatch.rs` | `dispatch_action()` — routes actions to the correct handler |
| **Engagement Scoring** | `engagement/scoring.rs` | `handle_engagement_decision()`, `modulate_persona_by_sentiment()` |
| **Decision Engine** | `decision/mod.rs` | `UnifiedEngine`, `DecisionEngineFactory`, `EngagementDecision`, `EngagementLevel` |
| **Decision Strategies** | `decision/strategies/` | Hybrid, legacy, LLM, persona, unified strategies |
| **Sentiment** | `sentiment/mod.rs` | Sentiment analysis core |
| **Sentiment Strategies** | `sentiment/strategies/` | Basic, context, domain, emoji, LLM strategies |
| **State** | `state/mod.rs` | `SessionState`, `TaskConfig`, `TweetActionTracker` |

### JS selector files (`src/utils/twitter/js/`)

26 files organized into two categories:

**Engagement JS (execution):** `js_confirm_retweet_click.js`, `js_extract_all_tweets.js`, `js_extract_username_from_url.js`, `js_find_*` (quote button, reply submit button, reply textarea, retweet confirm button, tweet button), `js_focus_composer.js`, `js_get_current_url.js`, `js_identify_engagement_candidates.js`, `js_root_tweet_button_center.js`, `js_verify_like.js`, `js_verify_quote_posted.js`

**Selector JS (detection):** `selector_all_tweets.js`, `selector_close_button.js`, `selector_element_center.js`, `selector_engagement_buttons.js`, `selector_feed_visible.js`, `selector_following_indicator.js`, `selector_follow_button.js`, `selector_follow_confirm_modal.js`, `selector_health_check.js`, `selector_login_flow.js`, `selector_popup_overlay.js`, `selector_tweet_user_avatar.js`

### Twitter sub-tasks (`src/task/`)

| Task | File | Duration | Actions |
|---|---|---|---|
| twitterdive | `twitterdive.rs` | varies | Dive into thread, scroll, engage replies |
| twitterfollow | `twitterfollow.rs` | 45s | Navigate profile, click follow, verify |
| twitterintent | `twitterintent.rs` | 45s | Navigate tweet intent URL |
| twitterlike | `twitterlike.rs` | 30s | Like from feed or specific tweet |
| twitterquote | `twitterquote.rs` | varies | Quote tweet with commentary |
| twitterreply | `twitterreply.rs` | varies | Reply to tweet |
| twitterretweet | `twitterretweet.rs` | varies | Retweet with confirmation |
| twittertest | `twittertest.rs` | varies | Test infrastructure |
| twitteractivity | `twitteractivity.rs` | 10 min | Main orchestrator — full pipeline |

---

## 3. The main task: `twitteractivity.rs`

This is the top-level orchestrator that calls into the utility modules. Understanding its structure is key to understanding the whole system.

**Flow:**
1. **Entry point config** — Parse payload for `duration_ms`, `candidate_count`, `thread_depth`, `max_actions_per_scan`
2. **Phase 1: Navigation** — `phase1_navigation()` picks a random entry point from 15 weighted URLs, navigates, simulates reading, dismisses popups, verifies login
3. **Phase 2: Scan + Engage loop** — Loop until duration budget exhausted:
   - `identify_engagement_candidates()` → get visible tweets with metadata
   - `filter_candidates()` → exclude empty IDs, tweets above viewport, too-short tweets
   - `process_candidate()` → sentiment → decision → dispatch → verify → record
   - Scroll and repeat
4. **Phase 3: Session summary** — Log engagement counters

**Key entry points (weighted random selection):**
```
59%  → x.com/ (home)
4%   → x 8 more (explore, trending, bookmarks, notifications, etc.)
2%   → x 3 more (connect_people)
1%   → x 3 more (explore tabs: news, sports, entertainment)
```

---

## 4. The state machine: `ComposerFlow`

The `ComposerFlow` in `twitteractivity_types.rs` tracks reply/quote composer state:

```
Idle ──record_composer_opened()──→ ComposerOpen ──record_text_entered()──→ TextEntered ──record_posted()──→ Posted
```

All 4 states with transition validation — invalid transitions return `FlowError`.

### Session state: `SessionState`

From `state/session.rs` — tracks per-session:
- `engagement_counters: EngagementCounters` — likes, retweets, follows, replies, bookmarks, quotes, dives
- `action_tracker: TweetActionTracker` — deduplication per session
- `behavior_runtime: BehaviorRuntime` — typing speed, action delay, scroll behavior
- `config: TaskConfig` — per-task configuration from payload + defaults

### Task config: `TaskConfig`

From `state/config.rs` — parsed from task payload:
- `duration_ms` — scan duration
- `candidate_count` — max candidates per scan
- `max_actions_per_scan` — action budget per scan cycle
- `dry_run_actions` — simulate without clicking
- `llm_api_key` — optional LLM key override
- Persona weights overrides from payload

---

## 5. The selector system

### How selectors work

Selectors are **pure functions that return JavaScript strings**. They are NOT evaluated JS files — they build JS at runtime:

```rust
// In twitteractivity_selectors.rs
pub fn selector_feed_visible() -> String {
    r#"
        (function() {
            return document.querySelector('main[role="main"]') !== null;
        })()
    "#.to_string()
}

// Used via:
let js = selector_feed_visible();
let result = api.page().evaluate(js).await?;
```

### Selector categories

**Detection selectors** — return `bool` or presence:
- `selector_feed_visible()` — is home feed loaded?
- `selector_login_flow()` — is login screen showing?
- `selector_close_button()` — is there a dismissible popup?
- `selector_popup_overlay()` — is a modal overlay visible?
- `selector_following_indicator()` — check if already following

**Position selectors** — return `{x, y}` coordinates via `getBoundingClientRect()`:
- `js_root_tweet_button_center(selector)` — find button in root tweet article
- `selector_follow_button()` — find follow button coordinates
- `selector_element_center(selector)` — generic element center finder

**Data extraction selectors** — return tweet metadata:
- `js_identify_engagement_candidates()` — all visible tweets with positions and button coords
- `js_extract_all_tweets()` — tweet text, author, replies for LLM
- `js_get_current_url()` — current page URL

**Action verification selectors** — verify action succeeded:
- `js_verify_like(x, y)` — check if like button state changed
- `js_verify_quote_posted()` — check if quote was posted

### Key selectors defined as constants in `twitteractivity_selectors.rs`

- `LIKE_BUTTON_SELECTOR` — `[data-testid="like"]`
- `RETWEET_BUTTON_SELECTOR` — `[data-testid="retweet"]`
- `REPLY_BUTTON_SELECTOR` — `[data-testid="reply"]`
- `BOOKMARK_BUTTON_SELECTOR` — `[data-testid="bookmark"]`
- `RETWEET_CONFIRM_SELECTOR` — retweet modal confirm button

### How to add a new selector

1. Create the JS file in `src/utils/twitter/js/` (if non-trivial)
2. Add the function in `twitteractivity_selectors.rs` that returns the JS string
3. Export it via `pub use` in `mod.rs`
4. Write a unit test verifying the JS contains expected DOM selectors

---

## 6. Retry with backoff

### `retry_with_backoff(operation, config, api, name)`

Three presets via `RetryConfig`:

| Preset | Max attempts | Base delay | Max delay | When to use |
|---|---|---|---|---|
| `default()` | 3 | 500ms | 5,000ms | Standard operations |
| `conservative()` | 5 | 1,000ms | 10,000ms | LLM calls, critical actions |
| `aggressive()` | 2 | 250ms | 2,000ms | Fast toggles, verifications |

### Error classification

The system classifies errors into 3 categories via `ErrorClassifier` trait:
- **`Transient`** — retried (stale element, rate limit, network timeout)
- **`Permanent`** — not retried (invalid selector, bad input)
- **`Fatal`** — not retried (browser disconnected, session killed)

### Circuit breaker

`CircuitBreaker` — prevents cascade failures:
- Configurable threshold (default: 5 failures)
- Reset timeout (default: 30s)
- Atomic state machine: CLOSED → OPEN → HALF_OPEN → CLOSED
- CAS-based transition ensures only one caller probes on half-open

### Delay formula

```rust
fn calculate_delay(attempt: u32, config: &RetryConfig) -> u64 {
    let base = config.base_delay_ms * config.backoff_multiplier.pow(attempt - 1);
    let delay = min(base, config.max_delay_ms);
    // Add jitter: ±(delay * jitter_factor / 2)
    delay + jitter
}
```

---

## 7. The persona system

### `PersonaWeights`

Controls decision-making probabilities per candidate tweet:

| Field | Default | Range | Purpose |
|---|---|---|---|
| `like_prob` | 0.3 | 0–1 | Likelihood to like |
| `retweet_prob` | 0.1 | 0–1 | Likelihood to retweet |
| `quote_prob` | 0.05 | 0–1 | Likelihood to quote |
| `follow_prob` | 0.05 | 0–1 | Likelihood to follow |
| `reply_prob` | 0.02 | 0–1 | Likelihood to reply |
| `bookmark_prob` | 0.0 | 0–1 | Likelihood to bookmark |
| `thread_dive_prob` | 0.2 | 0–1 | Likelihood to dive into thread |
| `interest_multiplier` | 1.0 | 0–1 | Modulates all probabilities |

### How weights flow

```
config/default.toml
  → TwitterProbabilitiesConfig (env overrides possible)
    → select_persona_weights(WEIGHTS_FROM_PAYLOAD, config_probabilities)
      → PersonaWeights (with payload overrides)
        → apply_behavior_profile(persona, browser_profile, sentiment_score)
          → .with_sentiment_modulation(sentiment)
          → .with_profile_variance(profile)
          → .normalized()
```

### Decision functions

Each `should_*(persona) -> bool` rolls a random number:
```rust
pub fn should_like(persona: &PersonaWeights) -> bool {
    let prob = effective_probability(persona.like_prob, persona);
    rand::thread_rng().gen_bool(prob)
}
```

---

## 8. LLM integration

### Flow

```
extract_tweet_context(api)
  → JS evaluation extracts author + tweet text + top 10 replies
  → Sentiment analysis runs on tweet text
  → build_reply_messages() / build_quote_messages() constructs prompt
  → retry_with_backoff(llm.chat_with_fallback(messages), conservative config)
  → validate_reply() sanitizes output (removes @mentions, #hashtags, emojis)
  → ensure non-empty after sanitization
```

### Key components

- **`llm_instance()`** — `OnceLock<Llm>` singleton with fallback chain
- **`generate_reply(api, author, text, replies, sentiment)`** — returns sanitized reply text
- **`generate_quote_commentary(api, author, text, replies, sentiment)`** — returns sanitized quote commentary
- **`quote_tweet(api, commentary)`** — executes the quote in the browser
- **`validate_reply(text)`** — removes prohibited content from LLM output

### LLM providers

Configured via env vars (`LLM_PROVIDER`, `OLLAMA_API_URL`, `OPENROUTER_API_KEY`, etc.) and the `llm/client/` module handles fallback between providers.

---

## 9. Engagement limits

### `EngagementCounters`

Tracks per-session actions with `Option<NonZeroU32>` encoding (None = 0):

| Counter | Default max | Increment method |
|---|---|---|
| `likes` | 5 | `increment_like()` |
| `retweets` | 3 | `increment_retweet()` |
| `follows` | 2 | `increment_follow()` |
| `replies` | 1 | `increment_reply()` |
| `thread_dives` | 3 | `increment_thread_dive()` |
| `bookmarks` | 2 | `increment_bookmark()` |
| `quote_tweets` | 2 | `increment_quote_tweet()` |
| **total_actions** | 10 | `cached_total_actions` (auto-summed) |

### Limit checks

Each `can_*(counters)` method checks BOTH:
- Individual counter < individual max
- Total actions < total max

Example: `can_like()` → `counters.likes() < self.max_likes && counters.total_actions() < self.max_total_actions`

### Remaining capacity

`limits.remaining(&counters)` returns a HashMap with remaining budget per action type.

---

## 10. The decision engine

Located in `decision/` — handles smart engagement decisions at the candidate level.

### Architecture

```
DecisionEngineFactory::create(strategy, api_key)
  → UnifiedEngine
    → evaluate(candidate_tweet) -> Option<EngagementDecision>
      → EngagementDecision { level, score, reason }
```

### Engagement levels

```rust
pub enum EngagementLevel {
    None,        // Skip entirely (score <= 20)
    Low,         // Like only (score 21-50)
    Medium,      // Like + retweet (score 51-70)
    High,        // Like + retweet + reply/follow/quote (score 71+)
}
```

### Decision strategies

- **`Auto`** — picks the best strategy based on scoring complexity
- **`Legacy`** — rule-based (keyword matching, engagement history)
- **`Persona`** — profile-driven (behavior weights + sentiment)
- **`Hybrid`** — combines legacy rules with persona weights
- **`LLM`** — LLM-powered (uses LLM to evaluate tweet quality)

---

## 11. Sentiment analysis

Located in `sentiment/` — analyzes tweet text sentiment to guide engagement decisions.

### Strategy modules

| Strategy | Method | Best for |
|---|---|---|
| `basic` | Keyword matching (positive/negative word lists) | Speed, offline |
| `context` | Analyzes surrounding context | Threads, conversations |
| `domain` | Twitter-specific domain knowledge | Hashtag/topic awareness |
| `emoji` | Emoji-based sentiment | Quick reads |
| `llm` | LLM-powered sentiment analysis | Accuracy-critical |

### Sentiment integration

```
tweet text → analyze_sentiment() → Sentiment { Positive | Neutral | Negative }
  → modulate_persona_by_sentiment(tweet, config, persona)
    → PersonaWeights.with_sentiment_modulation(sentiment_score)
```

---

## 12. Thread diving

### `dive_into_thread(api, status_url)`

Opens a tweet's detail view by clicking the tweet link:
1. Escapes CSS-special characters in the status URL
2. Clicks `a[href='{escaped_url}']`
3. Waits for thread view indicators (dialog, tweetDetail, tweetThread, or tweet article)
4. Verifies URL matches expected status ID
5. Returns `DiveIntoThreadOutcome { opened, used_fallback_target }`

### `ThreadDiveGuard`

Drop guard that navigates back to home if the task is cancelled mid-dive. If the enclosing future is dropped (e.g., by `run_with_timeout`), the guard spawns a fire-and-forget `goto_home()` call.

### `process_candidate()` integration

1. Check if detail-view actions needed (reply, quote, follow)
2. Check `should_dive()` persona gate
3. If yes: pause scrolling, retry dive with backoff, handle failure
4. After dive: execute actions, optionally engage replies depth-first
5. Navigate back to home, resume scrolling

---

## 13. Engagement dispatch (`dispatch_action`)

Located in `engagement/dispatch.rs` — routes action type to correct handler:

| Action | Handler | Requires dive? |
|---|---|---|
| `like` | `like_at_position()` from tweet coordinates | No |
| `retweet` | `retweet_at_position()` + confirm modal | No |
| `bookmark` | `bookmark_at_position()` | No |
| `follow` | Navigates profile → clicks follow button | Yes |
| `reply` | LLM generate → open composer → type → send | Yes |
| `quote` | LLM generate → open quote composer → type → post | Yes |

---

## 14. Common pitfalls

### Pitfall 1: Adding a new selection without updating all pipeline stages

The candidate flow is: `feed.rs` → `engagement/mod.rs` → `dispatch.rs` → `selectors`. If you add a new field to the candidate JSON in `js_identify_engagement_candidates.js`, you must update:
- `filter_candidates()` in `feed.rs` (filter on new field)
- `process_candidate()` in `engagement/mod.rs` (use new field)
- `dispatch.rs` (handle new action)
- `twitteractivity_types.rs` (add to `EngagementOutcome` if new outcome)

### Pitfall 2: Not updating `cached_total_actions` in `EngagementCounters`

Each `increment_*()` method must update both the individual counter AND `cached_total_actions`. If you add a new counter type without updating the cached total, all `can_*()` checks that use `total_actions()` will be wrong.

### Pitfall 3: Modifying JS selectors without updating tests

There are `#[test]` functions that verify JS strings contain specific DOM selectors (e.g., `data-testid="like"`). If you change a selector, update the corresponding JS string assertion tests in `twitteractivity_selectors.rs`, `twitteractivity_interact.rs`, `twitteractivity_feed.rs`, etc.

### Pitfall 4: Forgetting the `ComposerFlow` state machine

Reply and quote flows must call `record_composer_opened()` → `record_text_entered()` → `record_posted()` in order. Calling `record_posted()` from `Idle` state returns `Err(FlowError)`. Always propagate these errors.

### Pitfall 5: Not handling the `ThreadDiveGuard`

When diving into a thread, a `ThreadDiveGuard` is armed. If the dive succeeds AND navigation home succeeds, the guard must be disarmed. If the task times out mid-dive, the guard's `Drop` impl navigates home. **Never explicitly drop the guard without disarming it first** — this would cause a redundant `goto_home()`.

### Pitfall 6: Confusing `engagement/` submodule with `twitteractivity_engagement.rs`

The `engagement/` directory contains `mod.rs`, `dispatch.rs`, `scoring.rs` — these are the **orchestration layer**. The `twitteractivity_interact.rs` and `twitteractivity_actions.rs` files contain the **actual browser interaction code**. When adding a new engagement action:
1. Add the browser interaction in `twitteractivity_actions.rs`
2. Add the dispatch routing in `engagement/dispatch.rs`
3. Wire it through `process_candidate()` in `engagement/mod.rs`

### Pitfall 7: Overriding persona weights without considering normalization

Payload overrides in `select_persona_weights()` are NOT normalized until the end. If you set `like_prob: 2.0`, it will pass through `effective_probability()` which clamps to `[0, 1]`. But if you use the raw weight directly (bypassing `effective_probability()`), you'll get `rng.gen_bool(2.0)` which always returns `true`. Always route through `effective_probability()` or the `should_*()` functions.

### Pitfall 8: Not checking both limits in `can_*()` methods

Each `can_*()` check must verify BOTH the individual counter AND total actions:
```rust
// Correct
counters.likes() < self.max_likes && counters.total_actions() < self.max_total_actions
// If you drop the second check, total_actions won't block actions
```

### Pitfall 9: LLM rate limits without conservative retry

LLM calls use `RetryConfig::conservative()` (5 attempts, 10s max delay). If you call `generate_reply()` or `generate_quote_commentary()` without wrapping in `retry_with_backoff`, transient LLM errors (rate limits, model overloaded) will fail immediately.

### Pitfall 10: CSS selector injection when building dynamic selectors

When building selectors from user input (e.g., `a[href='{status_url}']`), always escape CSS-special characters using `css_escape_attr_value()`:
```rust
let escaped = css_escape_attr_value(status_url);
let selector = format!("a[href='{escaped}']");
```
Without escaping, a status URL containing a single quote would break the CSS selector syntax.

---

## 15. Quick reference: adding a new action

1. **Add JS selector** in `twitteractivity_selectors.rs` — build JS string that finds the button and returns `{x, y}` coordinates
2. **Add browser interaction** in `twitteractivity_actions.rs` — `action_at_position(api, x, y)` function
3. **Add to `EngagementOutcome`** if needed (in `twitteractivity_types.rs`)
4. **Add to dispatch** in `engagement/dispatch.rs` — match arm + handler call
5. **Add counter** in `twitteractivity_limits.rs` — `increment_x()`, `can_x()`, update `to_summary()` and `remaining()`, update `cached_total_actions`
6. **Add persona weight** in `twitteractivity_persona.rs` — `x_prob` field + `should_x()` function
7. **Wire through `process_candidate()`** — add action type to `actions_to_do` selection
8. **Add tests** — unit test per new function + update existing
9. **Add task** if standalone — new file in `src/task/` + register in `mod.rs` + policy in `policy.rs`
10. **Run validation** — `cargo check && cargo test --lib twitterlike twitterfollow`

> last audited 26-06-26 by docs-auditor
