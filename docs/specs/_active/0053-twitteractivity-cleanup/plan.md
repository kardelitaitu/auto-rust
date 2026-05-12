# Plan

### Step 1: Remove dead functions
**Files:** `dive.rs`, `interact.rs`, `navigation.rs`, `feed.rs`, `humanized.rs`, `retry.rs`

Remove: `read_full_thread`, `ThreadCache`, `navigate_to_tweet`, `check_selector_health`, `retry_with_fallback`, `get_tweet_engagement_buttons`, `ensure_feed_populated`, `scroll_to_bottom_feed`, `scroll_feed`, `read_content_for`, `verify_element_hover`, `get_scroll_progress`, `extract_initial_thread_data`.

If public API: mark with `#[allow(dead_code)]` + reason comment instead.

### Step 2: Fix dead constants and config
**Files:** `selectors.rs`, `config/mod.rs`, `limits.rs`, `constants.rs`
- Fix `HOME_LOGO_SELECTOR` quoting (wrong backslashes)
- Remove `EngagementCheck` enum (unused)
- Remove `DEFAULT_TWITTERACTIVITY_DURATION_MS` (unreferenced)
- Remove `config.persona_file_path` (unreferenced)

### Step 3: Deduplicate persona building
**Files:** `simulation.rs`, `persona.rs`
- Replace `build_persona_weights()` with call to `select_persona_weights()`.

### Step 4: LLM client once per session
**File:** `llm.rs`
- Accept optional `&Llm` parameter. Create once in `process_candidate()`, pass to both `generate_reply()` and `generate_quote_commentary()`.

### Step 5: Lazy regex
**File:** `llm_validation.rs`
- Use `std::sync::OnceLock<regex::Regex>` for mentions and hashtags patterns.

### Step 6: Fix selector quoting consistency
**File:** `selectors.rs`
- Pick one quoting style and apply to all constants.

### Step 7: Verify
- `.\check-fast.ps1`
- `.\check.ps1`
- Confirm no compilation warnings about unused items.
