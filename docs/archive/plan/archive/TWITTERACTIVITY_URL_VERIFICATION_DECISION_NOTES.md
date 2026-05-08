# Twitter Activity URL Verification Decision Notes

last audited 08-05-26 by Kilo

**Date:** 2026-04-21   
**Purpose:** Historical decision notes on URL verification. 

## Decision: When to Verify URL Context 

### Current Implementation 
URL verification checks added before **every** engagement action: 
- `like_tweet()` 
- `retweet_tweet()` 
- `follow_from_tweet()` 
- `reply_to_tweet()` 
- `bookmark_tweet()` 

**Problem:** Adds latency and code duplication (~6 copies of verification logic). 

---

## Proposal: Verify URL Only When Context Changes 

### Check URL When: 
1. **Task starts** - Ensure on home feed (https://x.com/home) 
2. **After dive completes** - Ensure back on home feed 
3. **Before engagement batch** - Not per individual action 

### Don't Check When: 
- Before each individual action 
- During continuous scrolling 
- Unless context change detected 

---

## Implementation Plan 

### Phase 1: Add URL Check Function 
```rust 
/// Verify current page is on expected Twitter URL 
pub fn verify_twitter_context(api: &TaskContext, expected: &str) -> Result<bool> { 
    let current_url = api.url().await?; 
    Ok(current_url.contains(expected)) 
} 
```

### Phase 2: Call Once per Batch 
```rust 
// In main loop 
verify_twitter_context(&api, "x.com/home").await?;  // Once at start 

loop { 
    // Scan phase 
    let candidates = scan_feed(&api).await?; 
    
    // Engage phase 
    for candidate in candidates { 
        engage_with_tweet(&api, candidate).await?; 
    } 
    
    // No URL check here - context hasn't changed 
} 

// After dive completes 
verify_twitter_context(&api, "x.com/home").await?;  // Ensure back on home 
```

### Phase 3: Remove Individual Checks 
- Remove URL verification from: 
  - `like_tweet()` 
  - `retweet_tweet()` 
  - `follow_from_tweet()` 
  - `reply_to_tweet()` 
  - `bookmark_tweet()` 

---

## Benefits 

1. **Reduced latency** - No per-action URL checks 
2. **Cleaner code** - No duplication across 6 engagement types 
3. **Maintainable** - Single URL check function 
4. **Testable** - Easy to verify URL logic 

---

## Risks & Mitigations 

| Risk | Likelihood | Impact | Mitigation | 
|------|-----------|--------|------------| 
| Wrong context | Medium | High | Verify at task start + after dive | 
| Context drift | Low | Medium | Periodic checks (every N actions) | 
| Missed detection | Low | Medium | Log warnings on unexpected URLs | 

---

## Decision Status: ⏳ PENDING IMPLEMENTATION 

**Next Steps:** 
1. ⏳ Implement `verify_twitter_context()` function 
2. ⏳ Update main loop to call once at start 
3. ⏳ Remove individual URL checks from actions 
4. ⏳ Test with real Twitter sessions 
5. ⏳ Monitor for context-related failures 
