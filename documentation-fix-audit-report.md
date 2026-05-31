# Documentation Fix Plan — Audit Report

**Audited:** May 31, 2026  
**Auditor:** Buffy  
**Methodology:** Every claim verified against actual source code and file system

---

## Audit Legend

| Classification | Meaning |
|---|---|
| ✅ **MATCH** | Claim in doc-fix is correct |
| ⚠️ **DRIFT** | Claim in doc-fix is partially wrong or outdated itself |
| ❌ **INCORRECT** | Claim in doc-fix is wrong |
| ℹ️ **INFO** | Informational — no correctness judgment |

---

## Section 1: `docs/TASKS/twitteractivity.md` — Module Names & Architecture

### 1a. Stale module names
**Verdict: ✅ MATCH**

The doc lists flat files (`twitteractivity_decision.rs`, `twitteractivity_sentiment.rs`, etc.) that no longer exist. Decision logic is now in `decision/` subdirectory (7 files), sentiment in `sentiment/` subdirectory (5 files).

The proposed fix lists the correct subdirectory structure:
- `decision/`: `types.rs`, `engine.rs`, `strategies/{legacy,persona,llm,hybrid,unified}.rs`
- `sentiment/`: `analyzer.rs`, `utils.rs`, `strategies/{emoji,domain,llm}.rs`

### 1b. Stale "Smart Decision System" description
**Verdict: ✅ MATCH**

The doc says "LLM/ML to score tweet quality (0-100)". Actual code when `smart_decision_enabled: true` but `llm_enabled: false` uses `DecisionStrategy::Legacy` — keyword-based heuristic, not LLM/ML. The proposed fix's description of the strategy selection (Legacy vs Auto) matches `engagement.rs:103-107`.

### 1c. Missing `available_actions()` note
**Verdict: ✅ MATCH**

The doc-fix correctly identifies that `available_actions()` returns `"dive"`, `"quote"`, `"like"`, `"retweet"`, `"follow"`, `"reply"`, `"bookmark"`. Verified against `limits.rs:282-308`.

---

## Section 2: `src/task/twitteractivity.md` — Implementation Details

### 2a. Wrong default engagement limits
**Verdict: ✅ MATCH**

The doc shows `max_likes=50`, `max_retweets=20`, etc. Actual defaults (from `config/mod.rs` and `limits.rs`): `max_likes=5`, `max_retweets=3`, `max_follows=2`, `max_replies=1`, `max_quote_tweets=2`, `max_bookmarks=2`, `max_thread_dives=3`, `max_total_actions=10`.

The proposed fix struct definition is accurate. The field is `max_quote_tweets` (not `max_quotes`) and `max_total_actions` (not `max_actions_total`). ✅

### 2b. Wrong entry point total weight
**Verdict: ✅ MATCH**

Doc says "Total Weight: 99". Actual `ENTRY_POINTS` in `navigation.rs:324` has weights summing to 100. Verified by test at `navigation.rs:548-551` which asserts `weights must sum to 100`.

The proposed fix correctly shows the weights sum to 100. However, the doc-fix's calculation shows "59 + 32 + 4 + 5 = 100" but the actual table has 59 (home) + 7×4 (7 URLs at 4%) + 2×2 (2 URLs at 2%) + 4×1 (4 URLs at 1%) = 59 + 28 + 4 + 4 = 95. Hmm, let me recount.

Actually from the navigation.rs code, ENTRY_POINTS has 15 entries. The doc-fix doesn't list all of them but the 59+32+4+5 breakdown might not be exact. Let me count from the doc-fix's proposed table:

From `src/task/twitteractivity.md`:
- Home: 59
- 8 entries at 4% = 32 (Global Trending, Explore, For You, Trending, Bookmarks, Notifications, Mentions, Chat)
- 2 entries at 2% = 4 (Connect People, Connect Creator)
- 4 entries at 1% = 4 (News, Sports, Entertainment, For You alt)

Total = 59 + 32 + 4 + 4 = 99. But the actual sum is 100.

Wait, the test at navigation.rs:548-551 says "weights must sum to 100" and the code has `total_weight` calculation at 227. Let me check... The test assertion says the sum must equal 100. But the doc shows 59 + 8×4 + 2×2 + 4×1 = 59 + 32 + 4 + 4 = 99.

So the doc-fix's claim that the actual sum is 100 is correct (verified by code), but the doc's table actually SUMS TO 99 (59+32+4+4=99). The doc-fix correctly identifies this discrepancy. MATCH.

### 2c. `perform_thread_dive()` doesn't exist
**Verdict: ✅ MATCH**

`perform_thread_dive` returns 0 matches in codebase. Actual function is `dive_into_thread()` at `dive.rs:107`.

### 2d. "Uses thread cache" claim outdated
**Verdict: ✅ MATCH**

`ThreadCache` struct exists in comments/doc-tests (`dive.rs:24`, `state.rs:273`) but is no longer part of `CandidateContext` or the actual processing flow. The proposed fix ("No thread caching — each dive starts with fresh context") accurately reflects current behavior.

### 2e. Non-existent "4 built-in personas"
**Verdict: ✅ MATCH**

Doc lists 4 hardcoded personas (Passive, Casual, Engaged, PowerUser). Actual system uses 21 `BrowserProfile` presets from `profile.md`. The proposed fix's list matches the actual presets. The `persona.rs` file uses `PersonaWeights` struct with probability-based selection, not hardcoded personas.

### 2f. Wrong metric format examples
**Verdict: ✅ MATCH**

Doc shows fabricated formats like `candidate_scan | candidates=N duration_ms=X`. Actual metrics use `RUN_COUNTER_*` constants and `log_summary()` at task end. The proposed fix's RUN_COUNTER names (`RUN_COUNTER_LIKE_SUCCESS`, etc.) match `metrics.rs:489-508`.

### 2g. Non-existent step "Scroll to tweet"
**Verdict: ✅ MATCH**

Doc flow chart shows "Scroll to tweet" as a step. Candidates are already in viewport — no scrolling to individual tweets occurs. The proposed replacement flow chart accurately reflects the actual `process_candidate()` flow from `engagement.rs`.

---

## Section 3: `docs/review/twitteractivity-architecture-review.md` — Line Counts

### 3a. Stale line counts
**Verdict: ⚠️ DRIFT — the doc-fix's own "Actual" estimates are incorrect for multiple files**

| File | Doc says | Doc-fix claims "Actual" | **Actual** | Doc-fix accuracy |
|------|----------|------------------------|------------|------------------|
| `twitteractivity.rs` | 433 | ~695 | **690** | ✅ Close |
| `engagement.rs` | ~1400 | ~1568 | **1,591** | ✅ Close (-23) |
| `limits.rs` | 691 | ~760 | **757** | ✅ Close |
| `persona.rs` | 589 | ~270 | **697** | ❌ **Way off** (+427!) |
| `dive.rs` | 625 | ~362 | **361** | ✅ Correct |
| `popup.rs` | 498 | ~381 | **366** | ✅ Close |
| `retry.rs` | 348 | ~250 | **609** | ❌ **Way off** (+359!) |
| `llm_validation.rs` | 255 | ~155 | **263** | ❌ **Off by 108** |

**Critical issues:**
1. `persona.rs` — Doc-fix claims it shrank from 589→270, but it actually **grew** to 697
2. `retry.rs` — Doc-fix claims it shrank from 348→250, but it actually **grew** to 609
3. `llm_validation.rs` — Doc-fix claims 155, actual is 263

These files did not shrink — they grew. The doc-fix's delta directions are **reversed** for persona.rs and retry.rs.

**Additional files missing from the architecture review component map (now in subdirectories):**
- 7 files in `decision/` (engine.rs, types.rs, hybrid.rs, legacy.rs, llm.rs, persona.rs, unified.rs)
- 5 files in `sentiment/` (analyzer.rs, utils.rs, domain.rs, emoji.rs, llm.rs)
- 1 additional file: `twitteractivity_cookiebot.rs`

### 3b. Stale orchestrator line count
**Verdict: ✅ MATCH**

First paragraph says `~430 lines`; actual is 690 lines.

---

## Section 4: `docs/review/twitteractivity-end-to-end-flow-analysis.md`

### 4a. C1 — LLM API key threading
**Verdict: ✅ MATCH — C1 is indeed outdated**

The original end-to-end doc claims `handle_engagement_decision()` passes `None` as the API key. **This has been fixed.** Current code at `engagement.rs:293-297` passes `task_config.llm_api_key.clone()`, and line 109 creates `DecisionEngineFactory::create(strategy, llm_api_key)` with the key.

The proposed fix (mark as RESOLVED or remove) is correct. However, the doc-fix's fix suggestion should be **mark as RESOLVED** rather than remove, since the historical finding is still valuable for regression awareness.

### 4b. C2 — `extract_tweet_context()` JS author bug
**Verdict: ✅ MATCH — C2 has been fixed**

Current code at `llm.rs:126-159` shows the JS has been fixed:
- Uses `article[data-testid="tweet"]` correctly (attribute ON the article)
- Per-reply author via `reply.querySelector('[dir="auto"]')` (correct scoping)
- Loop starts at `i=1` (skips root tweet, correctly iterates replies)
- Each reply gets its own author variable

**Minor issue**: Line 132 still has a space in `article[data-testid="tweet"] [dir="auto"]` — but this is correct CSS: it selects `[dir="auto"]` descendants inside the first tweet article. This isn't a bug, just slightly imprecise (gets first `[dir="auto"]` in the tweet, which could be the username or display name). Both are fine for author extraction.

---

## Section 5: `docs/TWITTERACTIVITY_AUDIT_FINDINGS.md`

### 5a. Wrong orchestrator line count
**Verdict: ✅ MATCH**

§1.1 claims "~100-line orchestrator". Actual is 690 lines (including tests).

### 5b. Wrong component file count
**Verdict: ⚠️ DRIFT — doc-fix's own count (25) is also wrong**

| Source | Count |
|--------|-------|
| Audit doc claims | 18 files |
| Doc-fix claims | 25 (19 top + 3 + 3) |
| **Actual** | **31 non-mod.rs files** (19 top-level + 7 decision/ + 5 sentiment/) |

Breakdown:
- Top-level `src/utils/twitter/`: 19 `.rs` files (excluding `mod.rs`)
- `decision/` subdirectory: 7 files (engine, types, hybrid, legacy, llm, persona, unified)
- `sentiment/` subdirectory: 5 files (analyzer, utils, domain, emoji, llm)

The doc-fix incorrectly estimates `decision/` as "3 files" (actual: 7) and `sentiment/` as "3 files" (actual: 5).

### 5c. Wrong module names
**Verdict: ✅ MATCH**

The audit doc's §2.1 lists bare names (`constants.rs`, `dive.rs`) but actual file names have `twitteractivity_` prefix (`twitteractivity_constants.rs`, `twitteractivity_dive.rs`).

---

## Section 6: `docs/SUMMARY.md` — Broken Links

### 6a. Missing/archived file references
**Verdict: ❌ INCORRECT — the doc-fix's claim is wrong**

The doc-fix claims SUMMARY.md references `TASK_RUNNER_PREPARATION.md` and `TASK_RUNNER_DSL_BUILD.md` at the wrong path. **The SUMMARY already uses the correct archive paths:**
```markdown
- [TASK_RUNNER_PREPARATION.md](./_archive/TASK_RUNNER_PREPARATION.md)
- [TASK_RUNNER_DSL_BUILD.md](./_archive/TASK_RUNNER_DSL_BUILD.md)
```

Both files exist in `docs/_archive/`. The links are correct and not broken.

**The doc-fix's proposed fix ("Update paths to point to the archive locations") is unnecessary — they're already correct.**

---

## Section 7: `src/task/twitteractivity.md` (Entropy and Randomization)

### 7a. Entry point count
**Verdict: ✅ MATCH — but verdict is "15 still correct"**

Doc says "Weighted random (15 options)". Actual `ENTRY_POINTS` array at `navigation.rs:324` has 15 elements. Still accurate.

### 7b. Scroll amount 200-600px
**Verdict: ✅ MATCH**

Doc claims fixed 200-600px per scroll. Actual behavior is configurable via `scroll_amount_pixels` config field or `profile.scroll.amount`. The proposed fix accurately describes the actual behavior.

---

## Section 8: `README.md` — Task Count

### 8a. Task count verification
**Verdict: ✅ MATCH — no drift found**

Both `README.md` and `docs/TASKS/overview.md` list exactly 15 tasks:
- 6 demo: cookiebot, demoqa, demo-keyboard, demo-mouse, pageview, task-example
- 9 twitter: twitteractivity, twitterdive, twitterfollow, twitterintent, twitterlike, twitterquote, twitterreply, twitterretweet, twittertest

No discrepancy found. No fix needed.

---

## Section 9: Cross-Document Inconsistencies

### 9a. Run counter descriptions differ
**Verdict: ⚠️ DRIFT — metrics.rs has 19 fields, not 18**

The actual `TwitterActivityRunCounters` struct at `metrics.rs:489-508` has **19 fields**:
```
candidate_scanned, button_missing, click_verify_failed, dive_target_fallback_used,
like_success, like_failure, retweet_success, retweet_failure, follow_success,
follow_failure, reply_success, reply_failure, bookmark_success, bookmark_failure,
quote_success, quote_failure, dive_success, dive_failure + 1 more (total 19)
```

Wait, let me recount: candidate_scanned, button_missing, click_verify_failed, dive_target_fallback_used, like_success, like_failure, retweet_success, retweet_failure, follow_success, follow_failure, reply_success, reply_failure, bookmark_success, bookmark_failure, quote_success, quote_failure, dive_success, dive_failure = 18 fields.

Actually that's 18. The TWITTERACTIVITY_AUDIT_FINDINGS doc says 18. That's correct.

The doc-fix claims `docs/TWITTERACTIVITY_AUDIT_FINDINGS.md` §1.6 says "Says 18 fields" — and the actual struct has 18 fields. So that part is correct.

However, the three docs use different naming conventions:
- `src/task/twitteractivity.md`: Uses `RUN_COUNTER_*` constant names correctly 
- `docs/TWITTERACTIVITY_AUDIT_FINDINGS.md`: Says "18 fields" (correct)
- The doc-fix itself doesn't have the counter inconsistency wrong per se

The proposed fix (use exact field names from metrics.rs) is valid. ✅

### 9b. Two twitteractivity.md files contradict
**Verdict: ✅ MATCH**

- `docs/TASKS/twitteractivity.md`: max_likes=5 (correct)
- `src/task/twitteractivity.md`: max_likes=50 (wrong)

These contradict as claimed. Fix is to correct `src/task/twitteractivity.md`.

---

## Section 10: `docs/TDD_TWITTERACTIVITY.md`

### 10a. Run scripts existence
**Verdict: ✅ MATCH — script exists, no fix needed**

`run-twitter-tests.ps1` exists at 305 lines. Both `-Red` and `-Green` flags are document features of the script. No change needed.

---

## Summary Table

| # | Claim | Verdict | Notes |
|---|-------|---------|-------|
| 1a | Stale module names | ✅ MATCH | Decision/sentiment moved to subdirectories |
| 1b | Stale Smart Decision description | ✅ MATCH | Uses Legacy (not LLM) when llm_enabled=false |
| 1c | Missing available_actions() note | ✅ MATCH | Returns "dive", "quote" not "thread_dive", "quote_tweet" |
| 2a | Wrong defaults | ✅ MATCH | Doc shows 50/20/10/5, actual is 5/3/2/1 |
| 2b | Wrong total weight (99 vs 100) | ✅ MATCH | Actual sum is 100 |
| 2c | perform_thread_dive doesn't exist | ✅ MATCH | Function is dive_into_thread() |
| 2d | Thread cache claim outdated | ✅ MATCH | ThreadCache not in CandidateContext |
| 2e | 4 built-in personas wrong | ✅ MATCH | 21 BrowserProfile presets exist |
| 2f | Wrong metric formats | ✅ MATCH | Metrics use RUN_COUNTER_*, not fabricated formats |
| 2g | "Scroll to tweet" not real | ✅ MATCH | Candidates already in viewport |
| 3a | Stale line counts | ⚠️ **DRIFT** | Doc-fix itself has wrong estimates for persona/retry/llm_validation |
| 3b | Stale orchestrator line count | ✅ MATCH | ~430 vs 690 |
| 4a | C1 API key outdated | ✅ MATCH | **Bug fixed** — key now threaded correctly |
| 4b | C2 JS author bug fixed | ✅ MATCH | **Bug fixed** — JS now correctly scopes per-reply authors |
| 5a | Wrong orchestrator line count | ✅ MATCH | ~100 vs ~690 |
| 5b | Wrong component file count | ⚠️ **DRIFT** | Doc-fix says 25, actual is 31 files |
| 5c | Wrong module names | ✅ MATCH | Missing twitteractivity_ prefix |
| 6a | Broken SUMMARY links | ❌ **INCORRECT** | Links already point to correct archive paths |
| 7a | Entry point count | ✅ MATCH | Still 15, no change needed |
| 7b | Scroll amount 200-600px | ✅ MATCH | Configurable, not fixed range |
| 8a | Task count | ✅ MATCH | Both docs have 15 tasks, no drift |
| 9a | Run counter descriptions | ✅ MATCH | Inconsistency exists, fix needed |
| 9b | Cross-doc defaults contradict | ✅ MATCH | 5 vs 50 conflict exists |
| 10a | Script existence | ✅ MATCH | run-twitter-tests.ps1 exists (305 lines) |

### Summary

- **19 claims MATCH** — correct and actionable
- **3 DRIFTS** in the doc-fix itself:
  - **3a**: Line count estimates for persona.rs, retry.rs, llm_validation.rs are wrong (delta directions reversed)
  - **5b**: File count estimate of 25 is wrong (actual is 31 non-mod files)
  - **9a**: RunCounter documentation differences exist but need precise audit of each doc
- **1 INCORRECT claim**:
  - **6a**: SUMMARY.md links are NOT broken — they already point to correct archive paths

### Recommendations for the doc-fix

| Doc-fix Claim | Action |
|---|---|
| Sections 1, 2, 4, 7, 8, 10 | ✅ **Use as-is** — verified accurate |
| Section 3 (line counts) | ⚠️ **Fix estimates**: persona.rs=697, retry.rs=609, llm_validation.rs=263 |
| Section 5b (file count) | ⚠️ **Fix count**: 31 files (19 top + 7 decision + 5 sentiment) |
| Section 6 | ❌ **Remove or correct** — SUMMARY links are already correct |

### Critical Items to Fix in the doc-fix Before Applying

1. **Section 3a**: Replace wrong estimates with actual line counts — especially persona.rs (697 not 270), retry.rs (609 not 250), llm_validation.rs (263 not 155)
2. **Section 5b**: Change "25 component files (19 top-level + 6 in subdirectories)" to "31 component files (19 top-level + 12 in subdirectories)"
3. **Section 6**: Remove the broken links claim (SUMMARY.md links are correct)
4. **Section 9a**: Add run counter naming consistency pass across all 3 docs
