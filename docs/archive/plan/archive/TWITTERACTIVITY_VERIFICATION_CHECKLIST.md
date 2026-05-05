# TwitterActivity Verification Checklist

**Status:** Complete

This checklist tracks each interaction performed by the Rust twitteractivity task for verification against the Node.js reference implementation.

## Phase 1: Navigation & Authentication

- [x] **Entry Point Selection**
  - [x] Select weighted entry point (59% home, 32% other pages, 5% exploratory)
  - [x] Use `select_entry_point()` function with random selection based on weights
  - [x] Entry points: home, explore, bookmarks, notifications, trending, etc.

- [x] **Navigate to Entry Point**
  - [x] Use `api.navigate(entry_url, 30000)` to navigate to selected entry
  - [x] Wait 2 seconds after navigation
  - [x] Log entry point selection

- [x] **Check if on Home Feed**
  - [x] Use `is_on_home_feed(api)` to check current URL
  - [x] Returns true if on x.com/home or twitter.com/home

- [x] **Simulate Reading (if not on home)**
  - [x] Calculate random scroll duration (10-20 seconds)
  - [x] Scroll with random amounts (200-600px) for the duration
  - [x] Use `api.scroll_read()` with profile settings
  - [x] Log reading simulation

- [x] **Navigate to Home (after reading)**
  - [x] Use `goto_home(api)` to navigate back to home
  - [x] Wait 500ms after navigation

- [x] **Login Verification**
  - [x] Use `verify_login(api)` to check if user is logged in
  - [x] Log login status
  - [x] Continue even if not logged in (with warning)

- [x] **Popup Dismissal**
  - [x] Dismiss cookie banner using `dismiss_cookie_banner(api)`
  - [x] Dismiss signup nag using `dismiss_signup_nag(api)`
  - [x] Close active popup using `close_active_popup(api)`
  - [x] Log each dismissal result

## Phase 2: Feed Scanning

- [x] **Continuous Scrolling**
  - [x] Use `api.scroll_read()` with profile settings
  - [x] Scroll at regular intervals based on `scroll_pause_ms`
  - [x] Use profile scroll amount, smooth setting, and back scroll

- [x] **Feed Population Check**
  - [x] Use `ensure_feed_populated(api)` to check if feed has content
  - [x] Log warning if feed appears empty

- [x] **Candidate Identification**
  - [x] Use `identify_engagement_candidates(api)` to scan for tweets
  - [x] Extract tweet id, text, position, replies
  - [x] Log candidate count and scan duration

- [x] **Engagement Button Caching**
  - [x] Use `get_tweet_engagement_buttons(api)` to cache button positions
  - [x] Store in `buttons` variable for reuse
  - [x] Use cached buttons for engagement actions

- [x] **Candidate Selection**
  - [x] Take first N candidates based on `candidate_count`
  - [x] Process candidates in order

## Phase 3: Engagement Actions

### Per-Candidate Processing

- [x] **Sentiment Analysis**
  - [x] Use `analyze_tweet_sentiment(tweet)` to analyze tweet
  - [x] Modulate persona weights based on sentiment
  - [x] Negative: 0.3 multiplier, Positive: 1.3 multiplier, Neutral: 1.0

- [x] **Smart Decision Check (if enabled)**
  - [x] Extract tweet text and replies
  - [x] Use `decide_engagement(tweet_text, replies)` for rule-based decision
  - [x] Skip engagement if decision level is None
  - [x] Log decision reason and score

### Like Action

- [x] **Action Chaining Prevention**
  - [x] Check `action_tracker.can_perform_action(tweet_id, "like")`
  - [x] Enforce minimum delay between different action types (3000ms)
  - [x] Skip if cooldown active

- [x] **Limit Check**
  - [x] Check `limits.can_like(&counters)`
  - [x] Skip if like limit reached

- [x] **Button Position Lookup**
  - [x] Use `find_like_button_near(&buttons, pos.0, pos.1)` to find button
  - [x] Get button coordinates from cached positions

- [x] **Like Execution**
  - [x] Use `like_at_position(api, btn_pos.0, btn_pos.1)` to click like
  - [x] Wait for button state change verification
  - [x] Increment like counter if successful
  - [x] Record action in `action_tracker`
  - [x] Use `clustered_engagement_pause(api)` for human-like delay
  - [x] Log like action

### Retweet/Quote Action

- [x] **Action Chaining Prevention**
  - [x] Check `action_tracker.can_perform_action(tweet_id, "retweet")`
  - [x] Skip if cooldown active

- [x] **Limit Check**
  - [x] Check `limits.can_retweet(&counters)`
  - [x] Skip if retweet limit reached

- [x] **Quote Decision**
  - [x] Check if LLM enabled and quote probability met
  - [x] Use `llm_quote_probability` for probability

- [x] **Quote Tweet (if selected)**
  - [x] Use `extract_tweet_context(api)` to get tweet details
  - [x] Use `generate_quote_commentary(api, author, text, replies)`
  - [x] Use `quote_tweet(api, commentary)` to post
  - [x] Increment quote counter
  - [x] Record action in `action_tracker`
  - [x] Use `clustered_reply_pause(api)`
  - [x] Log quote action

- [x] **Regular Retweet (if not quote)**
  - [x] Use `retweet_tweet(api)` to retweet
  - [x] Increment retweet counter
  - [x] Record action in `action_tracker`
  - [x] Use `clustered_engagement_pause(api)`
  - [x] Log retweet action

### Follow Action

- [x] **Action Chaining Prevention**
  - [x] Check `action_tracker.can_perform_action(tweet_id, "follow")`
  - [x] Skip if cooldown active

- [x] **Limit Check**
  - [x] Check `limits.can_follow(&counters)`
  - [x] Skip if follow limit reached

- [x] **Follow Execution**
  - [x] Use `follow_from_tweet(api)` to follow user
  - [x] Increment follow counter if successful
  - [x] Record action in `action_tracker`
  - [x] Use `clustered_engagement_pause(api)`
  - [x] Log follow action

### Reply Action

- [x] **Action Chaining Prevention**
  - [x] Check `action_tracker.can_perform_action(tweet_id, "reply")`
  - [x] Skip if cooldown active

- [x] **Limit Check**
  - [x] Check `limits.can_reply(&counters)`
  - [x] Skip if reply limit reached

- [x] **Reply Text Generation**
  - [x] If LLM enabled: use `extract_tweet_context(api)` and `generate_reply(api, author, text, replies)`
  - [x] On LLM failure: fallback to `generate_reply_text(sentiment, counters.replies)`
  - [x] If LLM disabled: use `generate_reply_text(sentiment, counters.replies)`

- [x] **Reply Execution**
  - [x] Use `reply_to_tweet(api, &reply_text)` to post reply
  - [x] Increment reply counter if successful
  - [x] Record action in `action_tracker`
  - [x] Use `clustered_reply_pause(api)`
  - [x] Log reply action

### Thread Dive Action

- [x] **Action Chaining Prevention**
  - [x] Check `action_tracker.can_perform_action(tweet_id, "dive")`
  - [x] Skip if cooldown active

- [x] **Limit Check**
  - [x] Check `limits.can_dive(&counters)`
  - [x] Skip if thread dive limit reached

- [x] **Dive Execution**
  - [x] Use `dive_into_thread(api, pos.0, pos.1)` to click tweet
  - [x] Use `read_full_thread(api, thread_depth)` to scroll through thread
  - [x] Use `scroll_pause(api)` after reading
  - [x] Increment thread dive counter
  - [x] Record action in `action_tracker`

- [x] **Navigate Back to Home**
  - [x] Use `goto_home(api)` to return to home feed
  - [x] Use `scroll_pause(api)` after navigation
  - [x] Log navigation back to home

## Phase 4: Completion

- [x] **Time Check**
  - [x] Check remaining time: `deadline.saturating_duration_since(Instant::now())`
  - [x] Break if remaining time < 500ms

- [x] **Final Summary**
  - [x] Log engagement counters (likes, retweets, follows, replies, thread dives)
  - [x] Log remaining limits
  - [x] Log total duration
  - [x] Return success

## Task-API Usage Verification

- [x] **Navigation**
  - [x] `api.navigate(entry_url, timeout)` - used in navigate_and_read
  - [x] `goto_home(api)` - uses api.navigate internally

- [x] **Scrolling**
  - [x] `api.scroll_read(pauses, amount, smooth, back_scroll)` - used in navigate_and_read and main loop
  - [x] Uses profile scroll settings

- [x] **Clicking**
  - [x] Engagement actions use coordinate-based clicking via helper functions
  - [x] `like_at_position`, `retweet_tweet`, `follow_from_tweet`, etc.
  - [x] Uses `api.click_at(x, y)` internally

## Notes

- All interactions use task-api methods
- Human-like delays applied throughout
- Error handling with Result types
- Logging at appropriate levels
- Action chaining prevention enforced
- Engagement limits respected
