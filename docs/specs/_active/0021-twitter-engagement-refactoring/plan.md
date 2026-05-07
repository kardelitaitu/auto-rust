# Plan

## What Is the Solution

Refactor `process_candidate` within `twitteractivity_engagement.rs` by extracting helper functions. No new files or directories.

### Step 1: Extract Depth-First Engagement
Extract lines 737-857 (depth-first reply engagement) into:
```rust
async fn engage_replies(
    api: &TaskContext,
    persona: &PersonaWeights,
    task_config: &TaskConfig,
    counters: &mut EngagementCounters,
    actions_this_scan: &mut u32,
) -> Result<()> {
    // ... extracted logic
}
```

### Step 2: Extract Action Execution Block
Extract the repetitive action execution pattern (lines 362-710) into:
```rust
async fn execute_engagement_action(
    api: &TaskContext,
    action: &str,
    tweet_id: &str,
    did_dive: bool,
    counters: &mut EngagementCounters,
    task_config: &TaskConfig,
    sentiment: Sentiment,
) -> Result<bool> {
    // ... extracted logic for like/retweet/quote/reply/follow/bookmark
}
```

### Step 3: Extract Sentiment Modulation
Extract lines 128-178 (sentiment analysis and persona modulation) into:
```rust
fn modulate_persona_by_sentiment(
    tweet: &Value,
    task_config: &TaskConfig,
    persona: &PersonaWeights,
) -> (Sentiment, PersonaWeights) {
    // ... extracted logic
}
```

### Step 4: Simplify Action Selection
Refactor lines 200-258 to use a cleaner pattern for action selection.

## Internal API Outline

```rust
// All functions remain in twitteractivity_engagement.rs (same file)

/// Execute depth-first engagement on tweet replies
async fn engage_replies(
    api: &TaskContext,
    persona: &PersonaWeights,
    task_config: &TaskConfig,
    counters: &mut EngagementCounters,
    actions_this_scan: &mut u32,
) -> Result<()>;

/// Execute a single engagement action (like/retweet/quote/reply/follow/bookmark)
async fn execute_engagement_action(
    api: &TaskContext,
    action: &str,
    tweet_id: &str,
    did_dive: bool,
    counters: &mut EngagementCounters,
    task_config: &TaskConfig,
    sentiment: Sentiment,
) -> Result<bool>;

/// Apply sentiment-based persona modulation
fn modulate_persona_by_sentiment(
    tweet: &Value,
    task_config: &TaskConfig,
    persona: &PersonaWeights,
) -> (Sentiment, PersonaWeights);
```

## Decisions

1. **Keep in same file**: Twitter module already has 27 files. Adding more directories adds complexity without benefit.
2. **Extract functions, not modules**: Helper functions reduce `process_candidate` from 762 to ~200-300 lines.
3. **Keep tests in file**: The 322 lines of tests are properly placed in `#[cfg(test)]` modules.
4. **Don't break the function signature**: `process_candidate` keeps its current signature for compatibility.

## Expected Outcome

After refactoring:
- `process_candidate`: ~200-300 lines (from 762)
- `twitteractivity_engagement.rs`: ~1,400 lines (from 1,325, due to extracted function signatures)
- Readability: Improved (each helper has a single responsibility)
- Testability: Improved (helpers can be tested independently)


