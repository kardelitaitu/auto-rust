# Baseline

## What I Find

### Finding 1: LLM API key never reaches decision engine

- `engagement.rs:88` calls `DecisionEngineFactory::create(strategy, None)` -- API key hardcoded to None.
- `engine.rs:49-90`: ALL LLM-dependent strategies (`Llm`, `Hybrid`, `Unified`, `Auto`) silently fall back to `PersonaStrategy` when api_key is None.
- `state.rs:189-192`: correctly reads `llm_enabled` from payload/config into `TaskConfig`.
- `engagement.rs:81-85`: selects `DecisionStrategy::Auto` when `llm_enabled: true`, but engine never receives the API key.
- **Result**: Users setting `llm_enabled: true` believe LLM decisions are active but get rule-based heuristics only.

### Finding 2: extract_tweet_context() JS is broken

- `llm.rs:119`: `document.querySelector('[data-testid="tweet"] [dir="auto"]')` -- space before `[data-testid]` makes it a descendant selector. Should be `article[data-testid="tweet"]`.
- `llm.rs:128`: `querySelectorAll('article [data-testid="tweet"] [dir="auto"]')` -- same descendant bug. Twitter DOM is `article[data-testid="tweet"]` (attr on article). Likely returns zero results.
- `llm.rs:133`: `replies.push({ author: author, text: replyText })` -- every reply gets root tweet's author. Each reply should extract its OWN author.
- Compare `selector_all_tweets.js` which correctly uses `article[data-testid="tweet"]` (no space).

### Finding 3: Popups block login detection

- `navigation.rs:327-354` order: (1) navigate, (2) verify_login(), (3) dismiss popups.
- `verify_login()` calls `is_feed_visible()` which checks for primary column in DOM.
- Cookie banner or overlay covering feed causes `is_feed_visible()` false even when logged in.
- `dismiss_signup_nag()` at `popup.rs:139-141` is hard-disabled but still called.
- **Result**: Every task logs "User appears not logged in" false positive when popups present.

## What I Claim

Fix these three bugs to make LLM decisions work, correct reply/quote context, and eliminate false login warnings. Each fix is 3-15 lines.

## What Is the Proof

- `selector_all_tweets.js` confirms correct `article[data-testid="tweet"]` vs buggy descendant in `llm.rs`
- Phase1 navigation order is explicitly sequential in `navigation.rs:328-354`
