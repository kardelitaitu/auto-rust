# TwitterActivity Rust API Refactor Notes

**Status:** Mostly implemented; remaining JS is coordinate discovery only.

## Objective
Convert all direct JavaScript DOM manipulation in twitteractivity task to use the Rust task-api (coordinate-based clicking). This will enable cursor overlay tracking and align with the task-api design principles.

## Current Issue
The cursor overlay never moves because many twitteractivity functions use direct JavaScript clicks (e.g., `textEl.click()`) which bypass the Rust mouse movement functions that trigger overlay updates.

## Functions Using Direct JavaScript

### 1. dive_into_thread (twitteractivity_dive.rs)
- **Current:** Uses `querySelector` to find tweet and `textEl.click()` to click
- **Problem:** Bypasses Rust mouse API, no overlay tracking
- **Solution:** Use coordinate-based clicking with `api.click_at(x, y)`

### 2. Other functions to investigate
- reply_to_tweet
- retweet_tweet
- follow_from_tweet
- dismiss_cookie_banner
- dismiss_signup_nag
- close_active_popup

## Refactoring Approach

### Phase 1: Identify and Catalog
- [ ] Audit all twitteractivity utility functions for direct JavaScript usage
- [ ] Document each function's current implementation
- [ ] Identify which functions can be converted to coordinate-based clicking

### Phase 2: Convert Critical Functions
- [ ] Convert `dive_into_thread` to use coordinate-based clicking
  - Keep the coordinate parameters (_x, _y) that are already passed in
  - Use `api.click_at(x, y)` instead of JavaScript click
  - Test that thread dive still works

- [ ] Convert `reply_to_tweet` to use coordinate-based clicking
  - Use cached button positions or calculate coordinates
  - Use `api.click_at(x, y)` for input field and send button

### Phase 3: Convert Engagement Functions
- [ ] Convert `retweet_tweet` to use coordinate-based clicking
- [ ] Convert `follow_from_tweet` to use coordinate-based clicking
- [ ] Convert `like_at_position` (already uses Rust API - verify)

### Phase 4: Convert Utility Functions
- [ ] Convert popup dismissal functions if they use JavaScript clicks
- [ ] Convert cookie banner dismissal if it uses JavaScript clicks

### Phase 5: Testing
- [ ] Test all converted functions with cursor overlay enabled
- [ ] Verify cursor overlay moves during all interactions
- [ ] Run full twitteractivity task with overlay tracking
- [ ] Ensure no regressions in functionality

## Implementation Notes

### Coordinate-based vs Selector-based
- **Selector-based:** Find element by CSS selector, click directly (current approach)
- **Coordinate-based:** Use known coordinates, move cursor to position, click (desired approach)

### Benefits of Coordinate-based
1. Cursor overlay tracking works
2. More realistic human-like behavior (cursor movement before click)
3. Consistent with task-api design
4. Better for debugging and visualization

### Challenges
1. Need to ensure coordinates are accurate for each element
2. May need to re-calculate coordinates after page changes
3. Some elements might be easier to find with selectors

## Timeline
- Phase 1: 30 min (audit and documentation)
- Phase 2: 1 hour (critical functions)
- Phase 3: 1 hour (engagement functions)
- Phase 4: 30 min (utility functions)
- Phase 5: 1 hour (testing)

Total estimated: ~4 hours
