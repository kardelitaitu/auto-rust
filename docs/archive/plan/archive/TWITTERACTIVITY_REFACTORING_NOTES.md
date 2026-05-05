# TwitterActivity Refactoring Notes

**Status:** Partially implemented; the main flow split and coordinate-click path are already in the codebase.

## Current Issues Identified

### 1. Navigation/Engagement Conflicts
- Background scrolling conflicts with engagement actions
- Tweet positions change while trying to engage
- Scrolling pause mechanism is complex and error-prone
- navigate_to_tweet function keeps hanging due to scroll issues

### 2. URL Verification Overhead
- URL checks added before every engagement action
- Adds latency and complexity
- May not be necessary for all actions
- Code duplication across 5 engagement types

### 3. Complex Engagement Flow
- Nested if-else blocks for like/retweet/quote/follow/reply/dive
- Duplicate code for each engagement type:
  - URL verification
  - Navigation
  - Scrolling pause/resume
  - Action tracking
  - Limit checking
- Hard to maintain and debug

### 4. Thread Dive Issues
- Multiple iterations to fix dive function
- Selector-based vs coordinate-based approaches
- Still having timeout issues
- URL context unclear during dive

### 5. Code Duplication
- URL verification logic repeated 6 times
- Navigation logic repeated 6 times
- Scrolling pause/resume repeated 6 times
- Error handling repeated 6 times

## Proposed Architecture

### Phase Separation

```
Phase 1: Setup & Validation
- Load config & persona
- Navigate to home feed
- Verify login state
- Dismiss popups

Phase 2: Feed Scanning (Background Loop)
- Continuous scrolling (independent)
- Candidate identification
- Sentiment analysis
- NO engagement in this phase

Phase 3: Engagement (Foreground Loop)
- Pause scrolling
- Select candidate from scan results
- Verify URL context
- Perform engagement
- Resume scrolling
- Repeat until duration expires

Phase 4: Cleanup
- Navigate back to home
- Log summary
```

### Engagement Flow Design

```
engage_with_tweet(api, tweet, action_type):
  1. Verify context (URL check once per batch, not per action)
  2. Pause background scrolling
  3. Navigate to tweet (if needed)
  4. Perform action (like/retweet/etc)
  5. Track action
  6. Resume scrolling
  7. Return to home (if navigated away)
```

### URL Verification Strategy

**Check only when:**
- Task starts (ensure on home)
- After dive completes (ensure back on home)
- Before engagement batch (not per action)

**Don't check:**
- Before each individual action
- During continuous scrolling
- Unless context changes detected

### Scrolling Strategy

**Two separate threads:**
1. **Background Scroller**: Independent, never paused
   - Scrolls at fixed intervals
   - Feeds candidate queue

2. **Engagement Loop**: Pauses background scroller
   - Takes from candidate queue
   - Engages with tweet
   - Resumes scroller

**Alternative (simpler):**
- Single loop with explicit phases
- Phase A: Scroll & scan (no engagement)
- Phase B: Engage (no scrolling)
- Alternate between phases

### Proposed Code Structure

```rust
// Main task loop
async fn run(api, payload) -> Result<()> {
    setup(api, payload).await?;
    
    loop {
        if should_engage() {
            engage_phase(api).await?;
        } else {
            scan_phase(api).await?;
        }
        
        if time_expired() { break; }
    }
    
    cleanup(api).await?;
}

// Scan phase - only scroll and collect candidates
async fn scan_phase(api) -> Result<Vec<TweetCandidate>> {
    scroll_feed(api).await?;
    let candidates = identify_candidates(api).await?;
    Ok(candidates)
}

// Engage phase - only engage, no scrolling
async fn engage_phase(api, candidates) -> Result<()> {
    for candidate in candidates {
        if should_engage(candidate) {
            perform_engagement(api, candidate).await?;
        }
    }
}

// Single engagement function
async fn perform_engagement(api, candidate, action) -> Result<()> {
    ensure_context(api).await?;  // URL check once
    navigate_to_tweet(api, candidate).await?;
    execute_action(api, action).await?;
    return_to_context(api).await?;
}
```

## Refactoring Approach

### Option A: Incremental Refactoring (Recommended)
1. Extract engagement logic into separate function
2. Remove URL checks from individual actions
3. Simplify navigate_to_tweet (remove scroll)
4. Separate scan and engage phases
5. Clean up scrolling logic
6. Test each step

### Option B: Rewrite (Higher Risk)
1. Create new file with clean architecture
2. Migrate logic piece by piece
3. Test new implementation
4. Replace old file

**Recommendation:** Option A - safer, can test incrementally

## Next Steps

1. ✅ Analyze current architecture
2. ⏳ Design clean separation of concerns
3. ⏳ Plan engagement flow
4. ⏳ Design URL verification strategy
5. ⏳ Choose refactoring approach
6. ⏳ Implement refactoring
7. ⏳ Test thoroughly

## Priority Order

1. **Fix immediate hanging issue** - Remove scroll from navigate_to_tweet
2. **Simplify scrolling logic** - Remove pause/resume complexity
3. **Extract engagement function** - Reduce code duplication
4. **Separate scan/engage phases** - Prevent conflicts
5. **Optimize URL verification** - Check only when needed
