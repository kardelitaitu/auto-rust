# Plan

### Step 1: Thread LLM API key to decision engine

**Files:** `src/config/mod.rs`, `src/task/twitteractivity.rs`, `src/utils/twitter/twitteractivity_engagement.rs`

1. In `engagement.rs::handle_engagement_decision()`, accept `api_key: Option<String>` parameter.
2. Pass through to `DecisionEngineFactory::create(strategy, api_key)` instead of `None`.
3. In `process_candidate()`, extract the API key from config and pass to `handle_engagement_decision()`.

### Step 2: Fix extract_tweet_context() JS

**File:** `src/utils/twitter/twitteractivity_llm.rs`

1. Fix line 119: use `article[data-testid="tweet"] [dir="auto"]` scoped properly.
2. Fix line 128: for each reply element, extract its OWN author and text independently.
3. Update unit tests to verify correct author attribution.

### Step 3: Reorder phase1_navigation popup dismissal

**File:** `src/utils/twitter/twitteractivity_navigation.rs`

1. Move popup dismissal (lines 339-351) BEFORE `verify_login()` (line 332).
2. Fix `dismiss_signup_nag()` or remove the call.
3. Update any tests that depend on the old order.

### Step 4: Verify

- Run `.\check-fast.ps1` for scoped tests.
- Run `.\check.ps1` for full validation.
- Verify no regressions in existing test assertions.
