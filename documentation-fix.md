# Documentation Fix Plan

A comprehensive review of 10 critical documentation files against the actual codebase.
Each finding includes the file path, the stale/incorrect claim, and the correction needed.

---

## 1. `docs/TASKS/twitteractivity.md` — Module Names & Architecture

### 1a. Stale module names under "Twitter Utility Modules"

**Problem:** Lists module files that no longer exist:

```
twitteractivity_decision.rs          → Don't exist — these were split into decision/ subdirectory
twitteractivity_decision_unified.rs  → decision/types.rs, decision/engine.rs,
twitteractivity_decision_hybrid.rs   → decision/strategies/legacy.rs, persona.rs,
twitteractivity_decision_llm.rs      → llm.rs, hybrid.rs, unified.rs
twitteractivity_decision_persona.rs
twitteractivity_sentiment_llm.rs     → Don't exist — these were split into sentiment/ subdirectory
twitteractivity_sentiment.rs         → sentiment/analyzer.rs, sentiment/utils.rs,
twitteractivity_sentiment_enhanced.rs → sentiment/strategies/mod.rs, emoji.rs,
twitteractivity_sentiment_emoji.rs   → domain.rs, llm.rs
twitteractivity_sentiment_context.rs
twitteractivity_sentiment_domains.rs
```

**Fix:** Replace all stale flat-file references with the actual subdirectory structure:

```markdown
**Decision & Strategy:**
- `decision/types.rs`: `TweetContext`, `EngagementDecision`, `EngagementLevel`
- `decision/engine.rs`: `UnifiedEngine`, `DecisionEngineFactory`
- `decision/strategies/`: `legacy.rs` (keyword-based), `persona.rs` (probabilistic),
  `llm.rs` (LLM-powered), `hybrid.rs` (weighted combo), `unified.rs` (smart fallback)

**Sentiment Analysis:**
- `sentiment/analyzer.rs`: `SentimentAnalyzer`, multi-strategy pipeline
- `sentiment/utils.rs`: Helper functions
- `sentiment/strategies/`: `emoji.rs`, `domain.rs`, `llm.rs` (per-strategy implementations)
```

### 1b. Stale "Smart Decision System" description

**Problem:** The doc says smart decisions use "LLM/ML to score tweet quality (0-100)". When
`smart_decision_enabled: true` but `llm_enabled: false`, the actual engine uses
`DecisionStrategy::Legacy` — a keyword-based heuristic, not LLM/ML.

**Fix:** Replace the Smart Decision section with:

```markdown
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
```

### 1c. Missing `available_actions()` update

**Problem:** `available_actions()` now returns `"dive"` and `"quote"` (not `"thread_dive"`
and `"quote_tweet"`) after the bugfix. The doc doesn't mention `available_actions()` at all.

**Fix:** Add a note under Engagement Limits:

```markdown
> **Note:** `EngagementLimits::available_actions()` returns action strings `"dive"`,
> `"quote"`, `"like"`, `"retweet"`, `"follow"`, `"reply"`, `"bookmark"` — these match
> the keys used by `SessionState::is_action_allowed()` and `EngagementCounters::increment()`.
```

---

## 2. `src/task/twitteractivity.md` — Implementation Details

### 2a. Wrong default engagement limits

**Problem:** The `EngagementLimits` struct and description show wrong defaults:

| Field | Doc says | Actual default |
|-------|----------|----------------|
| max_likes | 50 | 5 |
| max_retweets | 20 | 3 |
| max_follows | 10 | 2 |
| max_replies | 5 | 1 |
| max_quotes | 5 | **Field doesn't exist — it's `max_quote_tweets`** |
| max_bookmarks | 10 | 2 |
| max_thread_dives | 5 | 3 |
| max_actions_total | 100 | 10 |

**Fix:** Replace the entire struct docblock with the actual struct definition:

```rust
pub struct EngagementLimits {
    pub max_likes: u32,           // Default: 5
    pub max_retweets: u32,        // Default: 3
    pub max_follows: u32,         // Default: 2
    pub max_replies: u32,         // Default: 1
    pub max_quote_tweets: u32,    // Default: 2  (NOT max_quotes)
    pub max_bookmarks: u32,       // Default: 2
    pub max_thread_dives: u32,    // Default: 3
    pub max_total_actions: u32,   // Default: 10
}
```

### 2b. Wrong entry point total weight

**Problem:** The entry point table shows "Total Weight: 99" but the actual code sums to 100:

```
59 (home) + 32 (8 URLs × 4%) + 4 (2 URLs × 2%) + 5 (4 URLs × 1-2%) = 100
```

**Fix:** Change "Total Weight: 99" to "Total Weight: 100".

### 2c. `perform_thread_dive()` function doesn't exist

**Problem:** Section 7 says:

```markdown
**Function:** `perform_thread_dive()` (from `twitteractivity_dive`)
```

This function does not exist. The actual function is `dive_into_thread()`.

**Fix:** Replace with:

```markdown
**Function:** `dive_into_thread()` in `twitteractivity_dive.rs`
```

### 2d. "Uses thread cache" claim is outdated

**Problem:** The doc says:

```markdown
**Uses thread cache** to avoid re-processing same thread.
```

`ThreadCache` struct exists in code comments but is no longer used in the actual processing
flow. `CandidateContext` comment references it but it was removed from the struct.

**Fix:** Remove the thread cache claim. Replace with:

```markdown
No thread caching — each dive starts with fresh context to prevent cross-tweet
contamination.
```

### 2e. Non-existent "4 built-in personas"

**Problem:** Lists 4 hardcoded personas: Passive, Casual, Engaged, PowerUser. The actual
system uses 21 `BrowserProfile` presets from `src/utils/profile.md` (e.g., Average, Teen,
Senior, Enthusiast, PowerUser, Cautious, etc.).

**Fix:** Replace with the actual list:

```markdown
**Persona Presets (21):**
`Average`, `Teen`, `Senior`, `Enthusiast`, `PowerUser`, `Cautious`, `Impatient`,
`Erratic`, `Researcher`, `Casual`, `Professional`, `Novice`, `Expert`, `Distracted`,
`Focused`, `Analytical`, `QuickScanner`, `Thorough`, `Adaptive`, `Stressed`, `Leisure`

Personas are loaded from `BrowserProfile` — they control scroll behavior, timing
variance, and engagement probabilities. See `src/utils/profile.md` for details.
```

### 2f. Wrong metric format examples

**Problem:** The "Metrics and Observability" section shows structured log formats
that don't match the actual code:

```
candidate_scan | candidates=N duration_ms=X           → No such format exists
engagement_action | action=like tweet_id=XXX result=success  → No such format
dive_complete | depth=N engagements=M duration_ms=X          → No such format
```

The actual metrics use `increment_run_counter()` calls like `RUN_COUNTER_LIKE_SUCCESS`
and summary logs via `log_summary()`.

**Fix:** Replace with actual metric patterns:

```markdown
The task emits metrics via `RunCounters` and logs summary at task end:

```
[twitter] Engagement summary | likes=5 retweets=2 follows=1 ... duration=120.3s
[twitter] Remaining limits | likes=0 retweets=1 follows=4 ...
```

Per-action success/failure is tracked through run counters:
- `RUN_COUNTER_LIKE_SUCCESS` / `RUN_COUNTER_LIKE_FAILURE`
- `RUN_COUNTER_RETWEET_SUCCESS` / `RUN_COUNTER_RETWEET_FAILURE`
- `RUN_COUNTER_FOLLOW_SUCCESS` / `RUN_COUNTER_FOLLOW_FAILURE`
- `RUN_COUNTER_DIVE_SUCCESS` / `RUN_COUNTER_DIVE_FAILURE`
- And more (see `src/metrics.rs`)
```

### 2g. Non-existent step "Scroll to tweet"

**Problem:** The action execution flow chart shows "Scroll to tweet" as step 1, but the
actual code does NOT scroll to individual tweets — candidates are already in viewport.

**Fix:** Replace the flow chart with the actual process_candidate flow:

```
┌──────────────────────────────┐
│   For Each Candidate:       │
│                              │
│  1. Modulate persona by     │
│     sentiment analysis       │
│  2. Smart decision (opt.)   │
│  3. Select actions by       │
│     persona probabilities   │
│  4. Check limits & tracker  │
│  5. Dive into thread?       │
│  6. Execute action(s)       │
│  7. Engage replies (depth)  │
│  8. Return to home feed     │
└──────────────────────────────┘
```

---

## 3. `docs/review/twitteractivity-architecture-review.md`

### 3a. Stale line counts

**Problem:** Multiple file line counts are outdated. Verified against actual code:

| File | Doc says | Actual | Delta |
|------|----------|--------|-------|
| `src/task/twitteractivity.rs` | 433 | 690 | -257 |
| `twitteractivity_engagement.rs` | ~1400 | 1,591 | -191 |
| `twitteractivity_limits.rs` | 691 | 757 | -66 |
| `twitteractivity_persona.rs` | 589 | 697 | -108 |
| `twitteractivity_dive.rs` | 625 | 361 | +264 |
| `twitteractivity_popup.rs` | 498 | 366 | +132 |
| `twitteractivity_retry.rs` | 348 | 609 | -261 |
| `twitteractivity_llm_validation.rs` | 255 | 263 | -8 |

Many are off (persona.rs -108 vs doc's +319 claim, retry.rs -261 vs doc's +98 claim), and the doc-fix had delta directions reversed for persona.rs and retry.rs.
The "Total: ~8200" is also inaccurate.

**Fix:** Re-audit every file line count and update the component map table.

### 3b. Stale orchestrator line count

**Problem:** First paragraph says `src/task/twitteractivity.rs` is "~430 lines" — it's now ~695.

**Fix:** Update to current line count.

---

## 4. `docs/review/twitteractivity-end-to-end-flow-analysis.md`

### 4a. Critical Bug C1 is outdated/incorrect

**Problem:** Claim C1 says "LLM API key never reaches the decision engine" — specifically
says `handle_engagement_decision()` passes `None` as the API key.

**Actual code** (engagement.rs:206, 109):
```rust
// Line 206: called with:
handle_engagement_decision(tweet, task_config, &candidate_persona, task_config.llm_api_key.clone())

// Line 109: DecisionEngineFactory::create receives the key:
let engine = DecisionEngineFactory::create(strategy, llm_api_key);
```

The API key IS threaded correctly. This claim is **incorrect** in the end-to-end analysis
doc. It may have been true at the time of writing but was fixed since.

**Fix:** Either:
1. Remove C1 entirely (if the bug was fixed)
2. Add "**RESOLVED** — API key now correctly threaded through `CandidateContext`" status

### 4b. C2 author extraction bug — verify current state

**Problem:** C2 describes bugs in `extract_tweet_context()` JS that existed at review time.
Verify whether the JS in `llm.rs` has been fixed since.

**Fix:** Read the current `extract_tweet_context()` JS in `llm.rs` and update the doc.
If fixed, mark as resolved. If not, keep the finding but update line numbers.

---

## 5. `docs/TWITTERACTIVITY_AUDIT_FINDINGS.md`

### 5a. Wrong orchestrator line count

**Problem:** §1.1 claims `twitteractivity.rs` is "~100-line orchestrator". The actual
file is ~695 lines. While ~300 lines are test code, the orchestrator itself is ~350 lines.

**Fix:** Change to "~350-line orchestrator" or "~700-line file (with tests)".

### 5b. Wrong component file count

**Problem:** §2.1 says "18 component files". The actual `src/utils/twitter/` directory has:
- 19 `.rs` files (excluding `mod.rs`)
- Plus `decision/` subdirectory with 7 files (engine.rs, types.rs, hybrid.rs, legacy.rs, llm.rs, persona.rs, unified.rs)
- Plus `sentiment/` subdirectory with 5 files (analyzer.rs, utils.rs, domain.rs, emoji.rs, llm.rs)

So the total is 19 + 7 + 5 = 31 files.

**Fix:** Update to "31 component files (19 top-level + 12 in subdirectories)".

### 5c. Wrong module names in 2.1

**Problem:** Lists module files without the `twitteractivity_` prefix, but then
includes `simulation.rs` which does have the prefix in it. The full list in the
doc shows bare names like `constants.rs`, `dive.rs` etc., but the actual file names
are `twitteractivity_constants.rs`, `twitteractivity_dive.rs` etc.

**Fix:** Use the actual file names with their `twitteractivity_` prefix consistently.

---


## 7. `src/task/twitteractivity.md` (Section "Entropy and Randomization")

### 7a. Stale entry point count

**Problem:** Says "Weighted random (15 options)" — verify this is still 15.

**Fix:** Count the actual entry points in `twitteractivity_navigation.rs` `ENTRY_POINTS` array.
If still 15, OK. If changed, update.

### 7b. "Scroll amount 200-600px" — verify

**Problem:** Doc says scroll amount per scroll is "200-600px". Actual scroll is driven by
`scroll_amount_pixels` from config or `profile.scroll.amount`.

**Fix:** Update to: "Scroll amount configurable via `scroll_amount_pixels` or profile default
(typically 500-1500px)."

---

## 8. `README.md` — Basic Verification

### 8a. Task count in docs

**Problem:** README's "Available Tasks" section should be cross-referenced against
`docs/TASKS/overview.md`. If new tasks were added (like the twitter variants), README
may be missing them.

**Fix:** Verify README task list matches `docs/TASKS/overview.md` task list (15 tasks).

---

## 9. Cross-Document Inconsistencies

### 9a. Run counter descriptions differ across docs

**Problem:** The docs mention different run counter fields:
- `docs/review/twitteractivity-architecture-review.md`: Uses `RUN_COUNTER_*` names
- `src/task/twitteractivity.md`: Uses made-up format `RUN_COUNTER_CANDIDATE_SCANNED`
  (actual name may differ)
- `docs/TWITTERACTIVITY_AUDIT_FINDINGS.md` §1.6: Says 18 fields

**Fix:** Pull the actual `TwitterActivityRunCounters` struct definition from `src/metrics.rs`
and use the exact field names consistently across all docs.

### 9b. `docs/TASKS/twitteractivity.md` vs `src/task/twitteractivity.md` contradict

**Problem:** Two separate `twitteractivity.md` files at different paths serve different
audiences but contradict each other on defaults:
- `docs/TASKS/twitteractivity.md` says max_likes=5 (correct)
- `src/task/twitteractivity.md` says max_likes=50 (wrong)

**Fix:** Audit both docs and make default values consistent.

---

## 10. `docs/TDD_TWITTERACTIVITY.md`

### 10a. Verify run scripts still exist

**Problem:** References `.\run-twitter-tests.ps1 -Red`, `.\run-twitter-tests.ps1 -Green`.
Verify these PowerShell scripts still exist and work.

**Fix:** If scripts were removed or renamed, update the doc accordingly. If they still
exist, no change needed.

---

## Summary of Changes Required

| Doc | Issues | Priority |
|-----|--------|----------|
| `docs/TASKS/twitteractivity.md` | Stale module names, wrong strategy description, missing `available_actions()` note | **High** — misleading for new devs |
| `src/task/twitteractivity.md` | Wrong defaults (50 vs 5), wrong field names, non-existent functions, fabricated metric formats | **High** — misleading for implementers |
| `docs/review/twitteractivity-end-to-end-flow-analysis.md` | C1 is outdated/incorrect (API key threading was fixed) | **High** — incorrect bug claim |
| `docs/review/twitteractivity-architecture-review.md` | Line counts outdated, orchestrator size wrong | Medium |
| `docs/TWITTERACTIVITY_AUDIT_FINDINGS.md` | Wrong file count (18 vs 31), wrong line count, inconsistent naming | Medium |
| `README.md` | Task list drift possibility | Low |
| Cross-doc | Default limits contradict each other | **High** |
| `docs/TDD_TWITTERACTIVITY.md` | Script existence check | Low |

## Implementation Order

1. Fix incorrect bug claims (C1 in end-to-end doc) — **causes confusion for future audits**
2. Fix wrong default values (50/20/10 vs 5/3/2 in src/task/twitteractivity.md) — **misleads operators**
3. Fix stale module names and architecture (docs/TASKS/twitteractivity.md) — **misleads new devs**
4. Fix non-existent function references (`perform_thread_dive`, `ThreadCache`)
5. Fix line counts and file counts in review/audit docs
6. Cross-doc consistency pass for run counter names and limits
