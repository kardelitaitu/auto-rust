# Twitter Activity Task - Engagement Loop

## Main Loop Structure

The task operates in a continuous loop for the specified `duration_ms`, alternating between home feed scanning and tweet detail engagement.

### Step 1: Entry Point Navigation
- **Select Entry Point**: Choose from weighted list of URLs (home, explore, notifications, etc.) using `select_entry_point()`
- **Navigate**: Use `api.navigate()` for initial URL loading with timeout (first navigation only)
- **Simulate Reading**: If not on home feed, scroll and pause to mimic reading non-timeline pages
- **Go to Home**: Use `goto_home()` with mouse click on X logo element, verify feed visibility
- **Login Check**: Verify user is logged in using `verify_login()`
- **Popup Dismissal**: Handle cookie banners, signup nags, and active popups
- **Transition**: Proceed to continuous feed scanning

### Step 2: Home Feed Scanning and Candidate Selection
- **Continuous Scrolling**: Scroll feed at intervals (`scroll_pause_ms`) to load new content
- **Periodic Candidate Scans**: Every `MIN_CANDIDATE_SCAN_INTERVAL_MS` (2.5s), scan for engagement candidates
- **Candidate Identification**: Use `identify_engagement_candidates()` to find visible tweets via DOM query
- **Filtering**: Only consider tweets with >10 words (implemented in JS extraction)
- **Selection**: Take top `candidate_count` (default 5) candidates from scan
- **Sentiment Analysis**: Analyze tweet text sentiment (positive/neutral/negative)
- **Smart Decision**: If enabled, use rule-based filtering to skip inappropriate tweets
- **Action Planning**: For each candidate, determine which engagements to perform based on persona weights
- **Dive Decision**: If any engagement except like is planned, mark for dive
- **Transition**: If candidates found, proceed to engagement; else continue scanning

### Step 3: Tweet Detail Engagement
- **Dive Execution**: Click tweet link using `dive_into_thread()` with `api.click()` on status URL
- **Thread Reading**: Use `read_full_thread()` to scroll through thread content with reading pauses
- **Scroll to Top**: Ensure thread is fully loaded by scrolling to top implicitly
- **Engagement Execution**: Perform only 1 engagement per dive
  - **Like**: Use `like_tweet()` with `api.click()` on like button
  - **Retweet**: Use `retweet_tweet()` with `api.click()` on retweet button, 1-2s pause, then confirm button
  - **Quote**: Generate commentary (LLM/template) and `quote_tweet()`
- **Follow**: Use `follow_from_tweet()` with reading simulation
  - Simulates reading replies with scroll down and pause
  - Scrolls up slowly to bring follow button into view
  - Clicks follow button using `api.click()`
  - **Limits Enforcement**: Check per-action limits and daily caps
  - **Action Chaining**: Enforce minimum delays between actions on same tweet
  - **Pauses**: Use appropriate human-like pauses between actions
  - **Post-Engagement Wait**: 3-5s random wait after engagement before navigating home
  - **Go to Home**: Use `goto_home()` with mouse click on home logo

### Step 4: Loop Continuation
- **Time Check**: Continue loop until `duration_ms` elapsed
- **Final Summary**: Log engagement counts and total duration
- **Cleanup**: Ensure proper session termination

## Key Implementation Details

### Navigation Strategy
- **Initial Navigation**: `api.navigate()` for entry point loading
- **Subsequent Navigation**: `api.click()` for element-based navigation (home logo, tweet links)
- **History Navigation**: `api.back()` for browser back button equivalent
- **Fallback Handling**: Multiple strategies ensure reliable page transitions

### Candidate Requirements
- Tweet must be visible in viewport
- Tweet text must contain more than 10 words
- Tweet must have valid status URL for diving
- Must pass sentiment/smart decision filters

### Context-Specific Actions
- **Home Feed**: Likes only (using pre-extracted coordinates for performance)
- **Tweet Detail**: All engagements (retweet, quote, follow, reply, bookmark, likes) using `api.click()` selectors

### Safety and Realism
- Per-scan action limits prevent over-engagement
- Action chaining delays prevent spam detection
- Human-like scrolling, clicking, and pausing
- Verification of action success where possible
- Fallback handling for failed interactions

### Configuration Points
- Entry point weights
- Persona engagement probabilities
- LLM integration toggles and probabilities
- Time limits and delays
- Engagement limits and caps

## Advanced Features

### LLM Integration
- **Reply Generation**: Uses LLM for contextual replies when `llm_enabled` and `reply_prob` triggers action
- **Quote Commentary**: Generates quote tweet text via LLM when `llm_enabled` and `quote_prob` triggers action
- **Fallback**: Template-based responses when LLM fails or disabled
- **Context Extraction**: Pulls tweet author, text, and replies for LLM prompts

### Smart Decision System
- **Rule-Based Filtering**: Skips tweets based on content analysis (V3 feature)
- **Engagement Levels**: None/Low/Medium/High scoring for appropriate interaction
- **Reason Logging**: Explains why tweets are skipped or engaged

### Behavioral Realism
- **Persona Profiles**: Different engagement patterns (Casual, Enthusiastic, etc.)
- **Timing Variation**: Randomized pauses with human-like distributions
- **Action Chaining**: Prevents rapid successive actions on same tweet
- **Viewport Awareness**: Only engages visible, scrollable content

## Configuration Schema

### Complete Payload Example
```json
{
  "duration_ms": 120000,
  "candidate_count": 5,
  "thread_depth": 5,
  "max_actions_per_scan": 3,
  "weights": {
    "like_prob": 0.4,
    "retweet_prob": 0.15,
    "follow_prob": 0.05,
    "reply_prob": 0.02,
    "thread_dive_prob": 0.25,
    "quote_prob": 0.15,
    "bookmark_prob": 0.05
  },
  "llm_enabled": true,
  "smart_decision_enabled": false,
  "profile": "Casual"
}
```

### Key Parameters
- `duration_ms`: Total runtime (default: 120000 = 2min)
- `candidate_count`: Tweets to consider per scan (default: 5)
- `max_actions_per_scan`: Action budget per scan iteration (default: 3)
- `weights`: Probability multipliers for each engagement type (controls when actions trigger)
- `llm_enabled`: Enable LLM-powered replies and quotes (uses weights for triggering)
- `profile`: Behavior preset ("Casual", "Enthusiastic", etc.)

## Error Handling and Recovery

### Common Failure Points
- **Navigation Failures**: Entry point loading, home navigation
- **Element Not Found**: Tweet links, buttons, selectors outdated
- **Modal Timeouts**: Retweet confirmation, reply composition
- **Rate Limiting**: Twitter/X API or UI restrictions

### Recovery Strategies
- **Fallback URLs**: Multiple home navigation attempts
- **Selector Updates**: JS queries adapt to UI changes
- **Timeout Handling**: Configurable waits with graceful degradation
- **Verification Checks**: Post-action state validation

### Logging and Debugging
- **Action Tracking**: Detailed logs for each engagement attempt
- **Counter Metrics**: Success/failure rates per action type
- **Performance Stats**: Timing data for optimization
- **Error Context**: Full stack traces for troubleshooting

## Performance Considerations

### Optimization Points
- **Batch Processing**: Multiple candidates per scan
- **Lazy Loading**: Scroll-triggered content discovery
- **Caching**: Reuse extracted tweet data
- **Async Operations**: Concurrent API calls where possible

### Resource Usage
- **Memory**: Tweet data storage and cleanup
- **Network**: API calls for LLM, navigation requests
- **CPU**: Sentiment analysis, text processing
- **Time**: Human-like delays vs. execution speed balance

## Testing and Validation

### Unit Tests
- Payload parsing validation
- Sentiment analysis accuracy
- Persona weight calculations
- Limit enforcement logic

### Integration Tests
- End-to-end engagement flows
- UI selector stability
- Error recovery scenarios
- Performance benchmarks

### Manual Testing Checklist
- [ ] Entry point navigation works
- [ ] Candidate scanning finds valid tweets
- [ ] Dive opens tweet details correctly
- [ ] All engagement types execute successfully
- [ ] Home navigation reliable
- [ ] Limits prevent over-engagement
- [ ] LLM features work when enabled
- [ ] Error handling doesn't crash task

## Future Enhancements

### Potential Improvements
- **Machine Learning**: Adaptive engagement based on success rates
- **Multi-threading**: Parallel engagement on multiple tabs
- **Advanced Sentiment**: Emotion detection beyond positive/neutral/negative
- **Conversation Tracking**: Follow-up replies in threads
- **Content Analysis**: Topic-based engagement preferences
- **A/B Testing**: Compare different persona effectiveness
- **Analytics Dashboard**: Real-time engagement metrics

### Technical Debt
- Selector brittleness (Twitter UI changes)
- Hardcoded timeouts and delays
- Limited error recovery automation
- Basic sentiment analysis implementation

## Element Configuration

### Selector Centralization
All Twitter/X UI selectors are centralized as constants at the top of `twitteractivity.rs` for easy maintenance:
- `HOME_LOGO_SELECTOR`: Navigation home button
- `TWEET_LINK_SELECTOR`: Tweet links for diving
- `TWEET_DETAIL_SELECTOR` & fallbacks: Modal/dialog detection
- `RETWEET_BUTTON_SELECTOR`: Retweet menu trigger
- `RETWEET_CONFIRM_SELECTOR`: Retweet confirmation button
- `LIKE_BUTTON_SELECTOR`: Like/heart button

### Selector Strategy
- Simple CSS selectors preferred over complex JS for reliability
- Fallback selectors for UI variations
- Data-testid attributes used where stable
- ARIA labels for accessibility-based selection

## API Click Improvement Plan

### Core Reliability Enhancements

#### 1. Element Detection & Waiting
- **Smart Waiting**: Wait for element stability, not just presence
- **Visual Confirmation**: Verify element is actually clickable (not obscured)
- **Dynamic Selector Fallback**: Try multiple selector strategies automatically
- **Intersection Observer**: Detect when element enters viewport properly

#### 2. Cursor Movement Optimization
- **Adaptive Speed**: Adjust cursor speed based on distance and context
- **Collision Avoidance**: Detect and avoid UI elements during movement
- **Path Optimization**: Choose most natural cursor paths
- **Momentum Simulation**: Add slight overshoots and corrections

#### 3. Timing & Human Behavior
- **Context-Aware Delays**: Different timing for different element types
- **Attention Simulation**: Occasional "distractions" with cursor pauses
- **Fatigue Simulation**: Slightly slower movements over time
- **Learning Adaptation**: Adjust timing based on success rates

#### 4. Error Recovery & Fallbacks
- **Multi-Level Fallbacks**: Element click → coordinate click → scroll retry
- **State Recovery**: Restore cursor position after failed attempts
- **Progressive Retries**: Increase delays between retry attempts
- **Failure Analytics**: Track and learn from click failure patterns

### Advanced Features

#### 5. Visual & Accessibility Support
- **Screen Reader Awareness**: Detect and adapt to accessibility features
- **High Contrast Mode**: Adjust behavior for accessibility users
- **Cursor Overlay Enhancement**: Better visual feedback during automation
- **Element Highlighting**: Optional visual debugging mode

#### 6. Performance Optimizations
- **Element Caching**: Cache recently found elements to reduce DOM queries
- **Batch Operations**: Queue multiple clicks for coordinated execution
- **Lazy Evaluation**: Defer expensive checks until needed
- **Memory Management**: Clean up event listeners and cached data

#### 7. Gesture & Interaction Support
- **Multi-Click Patterns**: Double-click, triple-click support
- **Drag Operations**: Smooth drag-and-drop interactions
- **Right-Click Menus**: Context menu interaction support
- **Touch Simulation**: Mobile/touch device gesture support

#### 8. Monitoring & Analytics
- **Click Success Metrics**: Track success rates by element type
- **Performance Profiling**: Measure click timing and reliability
- **Failure Pattern Analysis**: Identify problematic selectors/pages
- **A/B Testing Framework**: Compare different click strategies

### Implementation Roadmap

#### Phase 1: Core Reliability (Week 1-2)
- [ ] Implement smart element waiting with stability checks
- [ ] Add visual confirmation for clickable elements
- [ ] Enhance cursor movement with collision avoidance
- [ ] Improve error recovery with progressive backoff

#### Phase 2: Human Behavior Enhancement (Week 3-4)
- [ ] Add context-aware timing adjustments
- [ ] Implement attention simulation and fatigue effects
- [ ] Create adaptive speed algorithms
- [ ] Add learning adaptation based on success metrics

#### Phase 3: Advanced Features (Week 5-6)
- [ ] Add accessibility awareness and support
- [ ] Implement gesture support (drag, multi-click)
- [ ] Add comprehensive monitoring and analytics
- [ ] Create performance profiling system

#### Phase 4: Optimization & Polish (Week 7-8)
- [ ] Implement element caching and batch operations
- [ ] Add visual debugging and overlay enhancements
- [ ] Comprehensive testing across different sites
- [ ] Documentation and API stabilization

## API Usage Guidelines

### Navigation Methods
- `api.navigate(url, timeout)`: Initial page loads only
- `api.click(selector)`: Element-based navigation and interactions
- `api.back()`: Browser history navigation (equivalent to back button)
- `goto_home()`: Home navigation with logo click and verification

### When to Use Each Method
- **First visit to any URL**: `api.navigate()`
- **Clicking UI elements**: `api.click()` with centralized selectors
- **Returning to previous page**: `api.back()` (when applicable)
- **Ensuring home state**: `goto_home()` (robust fallback)

### Modal vs Page Navigation
- Twitter/X tweet details open as modals (same URL)
- Back navigation may not work for modal closures
- Element clicking preferred for modal interactions
- Page navigation reserved for cross-page transitions

## Loop Flow Diagram

```
Entry Point Selection → api.navigate() → Reading Simulation → Home Feed
       ↑                                                           ↓
       └────────────────── api.click() or api.back() ←─────────────┘
                                   ↓
Home Feed Scanning ←───────────────┘
     ↓
Candidate Found?
     ↓
   Yes → api.click() → Read Thread → Perform Engagements → api.back()
     ↓
    No
     ↓
Continue Scanning
```