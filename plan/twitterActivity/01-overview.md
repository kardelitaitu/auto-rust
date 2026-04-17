# Twitter Activity Task — Overview

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
