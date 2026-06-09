# Twitteractivity Module — Cross-Cutting Audit Summary

## Coverage

All 20 source files in `src/utils/twitter/` analyzed across 5 groups:

| Group | Files | Lines | Status |
|-------|-------|-------|--------|
| 1: Pipeline Core | actions (226), constants (13), types (546) | 785 | CLEAN |
| 2: Navigation | feed (349), dive (246), interact (666), popup (105), selectors (231), navigation (265) | 1,862 | 2 MEDIUM, 2 LOW |
| 3: Engagement & LLM | engagement (369), persona (162), llm (87), llm_execute (91), llm_validation (98) | 807 | CLEAN |
| 4: State & Limits | state (1,225), limits (1,310), retry (787) | 3,322 | 1 MINOR (existing) |
| 5: Utilities & Processor | humanized (305), helpers (156), errors (412), simulation (757), unified_processor (796) | 2,426 | 2 MEDIUM, 2 LOW |
| **Total** | **20 files** | **9,202** | **4 MEDIUM, 4 LOW/MINOR** |

## All Findings (ranked)

### CRITICAL (0) — none

### HIGH (0) — none (3 original HIGH bugs were fixed)

### MEDIUM (4)

| ID | File | Line | Description | Impact |
|----|------|------|-------------|--------|
| FEED-1 | feed.rs | 163-170 | `is_following_user_at_position` takes `_x`, `_y` params but ignores them — scans entire DOM for "Following" indicators instead of scoping to a specific user position. Returns a single global boolean | Blocks per-user follow-state detection; can cause incorrect skip/follow decisions |
| INTERACT-1 | interact.rs | 578 | `follow_from_tweet` uses `querySelectorAll` for "Following" span* without scoping to a specific user — same root cause as FEED-1 | May attempt to follow already-followed users (wasted action + rate limit) |
| UNIFIED-1 | unified_processor.rs | 347-363 | `clean_reply_content` strips `@`, `#`, non-alphanumeric chars including emoji, parens, colons, semicolons | Silently loses semantic content from LLM-generated replies |
| UNIFIED-2 | unified_processor.rs | 441-444 | `extract_content_from_quote` is a no-op: `Ok(response.to_string())` regardless of actual content | Misleading function name; content not cleaned before sentiment analysis |

### LOW (4)

| ID | File | Line | Description |
|----|------|------|-------------|
| NAV-1 | navigation.rs | 224-226 | Stale doc comment on `select_entry_point`: says *"If seed is 0"* but fn takes no `seed` param — doc drift |
| NAV-2 | navigation.rs | 103 | `get_element_center` embeds `'{selector}'` in inline JS without quote escaping — latent, not triggered by current callers |
| UNIFIED-3 | unified_processor.rs | 383-412 | Hardcoded English sentiment keywords only |
| UNIFIED-4 | unified_processor.rs | 416-431 | Empty text gets 0.5 confidence base |

### MINOR (1)

| ID | File | Line | Description |
|----|------|------|-------------|
| STATE-1 | state.rs | 236 | `_action_type` unused in `can_perform_action` — per-tweet, not per-action cooldown |

## Fixed Bugs (from initial session)

| Bug | File | Line | Fix |
|-----|------|------|-----|
| Poisoned mutex | engagement.rs | 250 | `try_lock` → `.lock()` with normal mutex |
| Retry-defeating error | dive.rs | 318 | `?` → `Err(...)` to preserve retry classification |
| Jitter underflow | retry.rs | 193 | `.max(0.0) as u64` prevents u64 overflow |

## Cross-Cutting Patterns

1. **DOM position scoping** (FEED-1, INTERACT-1): Two files do global `querySelectorAll` for "Following" state instead of scoping to a specific user's DOM position. Same root cause, same fix pattern.
2. **LLM output hygiene** (UNIFIED-1, UNIFIED-2): The unified processor doesn't sufficiently sanitize LLM output before downstream consumption.
3. **String-based error matching** (errors.rs, retry.rs): Both modules use string heuristics for error classification — fragile but acceptable for this domain.
4. **No tests for helpers.rs** (helpers.rs): Small utility file with no test coverage — low risk but noted.
5. **English-only sentiment** (unified_processor.rs, llm_validation.rs): Consistently English-only throughout.

## Recommendation

- **Fix FEED-1 + INTERACT-1** together (same root cause) — they cause real incorrect behavior during follow-state detection.
- **UNIFIED-1** is debatable: stripping `@`/`#` is intentional sanitization but may surprise users. Document as known behavior.
- **UNIFIED-2** is a rename-only fix: rename `extract_content_from_quote` to `get_raw_response` and add a cleaning step before returning.
- Skip the LOW/MINOR items unless they become actionable.
