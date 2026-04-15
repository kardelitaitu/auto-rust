# Retweet Task Documentation

## Overview

The retweet task (`retweet.js`) is an API-enhanced automation task that navigates to a tweet, retweets it, and simulates human-like behavior patterns. It uses weighted random branch selection to vary automation patterns and avoid detection.

## Configuration

### Branch Probabilities

| #   | Branch                                 | Weight | Time    | Purpose                                                                                  |
| --- | -------------------------------------- | ------ | ------- | ---------------------------------------------------------------------------------------- |
| 1   | `retweetBranch1_directRetweet`         | 20%    | 3-7 min | Browse tweet (10-20s) → Retweet → Home reading (2-5 min)                                 |
| 2   | `retweetBranch2_profileVisitRetweet`   | 15%    | 3-7 min | Profile visit (20-90s) → Back → Tweet read (20-90s) → Retweet → Home (2-5 min)           |
| 3   | `retweetBranch3_homeReadRetweet`       | 12%    | 2-4 min | Home browse (2-4 min) → Tweet read (2-5s) → Retweet                                      |
| 4   | `retweetBranch4_homeProfileRetweet`    | 8%     | 3-6 min | Home browse (2-4 min) → Profile (5-15s) → Back → Tweet (5-10s) → Retweet                 |
| 5   | `retweetBranch5_threadReader`          | 10%    | 4-8 min | Home (2-4 min) → Tweet (1-2 min) → Thread (1-2 min) → Back → Retweet                     |
| 6   | `retweetBranch6_likeRetweet`           | 10%    | 4-6 min | Home (2-3 min) → Tweet (1-2 min) → Like → Retweet                                        |
| 7   | `retweetBranch7_notificationCheck`     | 8%     | 4-7 min | Home (1-2 min) → Notifications (1-2 min) → Tweet (1-2 min) → Retweet                     |
| 8   | `retweetBranch8_overscrollReturn`      | 7%     | 4-6 min | Home (2-3 min) → Tweet (1-2 min) → Overscroll → Return → Retweet                         |
| 9   | `retweetBranch9_searchToRetweet`       | 7%     | 4-7 min | Home (1-2 min) → Search (1-2 min) → Tweet (1-2 min) → Retweet                            |
| 10  | `retweetBranch10_impulseRetweet`       | 6%     | 3-4 min | Home (2-3 min) → Quick retweet (1-3s)                                                    |
| 11  | `retweetBranch11_multiPhaseEngagement` | 5%     | 5-8 min | Home (1-2 min) → Notifications (1-2 min) → Tweet (1-2 min) → Retweet → Profile (1-2 min) |
| 12  | `retweetBranch12_misclickRecovery`     | 2%     | 4-6 min | Home (2-3 min) → Tweet (1-2 min) → Misclick → Escape → Retweet                           |

## Workflow

The retweet task follows these steps to ensure a human-like and robust interaction:

1.  **Validation**: Verifies the target URL is valid for x.com or twitter.com.
2.  **Navigation**: Direct navigation to the tweet URL.
3.  **Visual Verification**: Wait for the retweet button to become visible.
4.  **Human Behavior**: Simulates reading by scrolling through the content.
5.  **Execution**: Performs the retweet action using `api.retweet()` wrapped in `api.recover()` for maximum reliability.
6.  **Confirmation**: Verifies the action was successful by checking for the unretweet state.

### Debug Override

Force a specific branch via CLI:

```bash
node main.js retweet=https://x.com/user/status/123 debugBranch=retweetBranch1_directRetweet
```

Or edit `DEBUG_FORCE_BRANCH` flags at the top of `retweet.js`.

---

## Branch 1: Direct Retweet + Home Feed Reading

**Weight:** 20% | **Time:** 3-7 min | **Purpose:** Simulates users who visit a specific tweet, retweet it, then browse their home feed.

### Flow

```
tweet page → browse (10-20s) → retweet → home (2-5 min)
```

1. **Locate tweet and retweet button**
    - `article[data-testid="tweet"]` + `[data-testid="retweet"]`
2. **Browse page** (10-20 seconds)
    - Random scroll down (150-400px)
    - Random pauses (400-2500ms)
    - 20% chance to scroll back up (30-250px)
3. **Focus on retweet button** using `api.scroll.focus2()`
4. **Execute retweet** via `retweetWithAPI()`
5. **Navigate to home** via `api.twitter.home()`
6. **Read home feed** (2-5 minutes)
    - Random scroll amounts (200-600px)
    - 1-4 second pauses

### Behavior

- Fastest initial engagement
- Extended home reading after retweet simulates "checking what else is happening"
- Most common branch (20% weight)

---

## Branch 2: Profile Visit + Retweet

**Weight:** 15% | **Time:** 3-7 min | **Purpose:** Simulates users who check the author's profile before retweeting.

### Flow

```
tweet → click profile → read profile (20-90s) → back → read tweet (20-90s) → retweet → home (2-5 min)
```

1. **Extract profile** from tweet URL (`x.com/{username}/status/{id}`)
2. **Click profile link** on tweet (`a[href="/{username}"]`)
    - Falls back to direct navigation if link not visible
3. **Read profile** (20-90 seconds)
    - Scroll 1-3 times (200-600px)
    - Random pauses (500-2000ms)
4. **Navigate back** using `api.back()`
5. **Read tweet page** (20-90 seconds)
    - Scroll 1-2 times (200-400px)
    - Random pauses (500-1500ms)
6. **Focus on retweet button** using `api.scroll.focus2()`
7. **Execute retweet** via `retweetWithAPI()`
8. **Navigate to home** for extended reading (2-5 min)

### Behavior

- Verifies author credibility before retweeting
- Two-phase reading (profile + tweet page)
- Falls back to direct retweet if URL format invalid

---

## Branch 3: Home Browse + Read Then Retweet

**Weight:** 12% | **Time:** 2-4 min | **Purpose:** Simulates users who browse their home feed, then engage with a specific tweet.

### Flow

```
home (2-4 min) → tweet → hover & read (2-5s) → retweet
```

1. **Navigate to home** via `api.twitter.home()`
2. **Browse home feed** (2-4 minutes)
3. **Navigate to target tweet**
4. **Focus on tweet** using `api.scroll.focus()`
5. **Simulate reading** (2-5 seconds)
    - `api.hover()` over tweet element
    - Mouse wiggle movements (5px magnitude)
6. **Focus on retweet button** using `api.scroll.focus2()`
7. **Execute retweet** via `retweetWithAPI()`

### Behavior

- Home browsing creates "found this while scrolling" pattern
- Includes hover and mouse wiggles during reading
- No home reading after retweet (quick engagement)

---

## Branch 4: Home Browse + Profile Visit + Retweet

**Weight:** 8% | **Time:** 3-6 min | **Purpose:** Simulates users who browse home, check author profile, then retweet.

### Flow

```
home (2-4 min) → tweet → click profile → read profile (5-15s) → back → read tweet (5-10s) → retweet
```

1. **Navigate to home** via `api.twitter.home()`
2. **Browse home feed** (2-4 minutes)
3. **Navigate to target tweet**
4. **Extract and click profile** (`a[href="/{username}"]`)
    - Falls back to direct navigation
5. **Read profile** (5-15 seconds)
    - Scroll 1-3 times (200-600px)
6. **Navigate back** using `api.back()`
7. **Read tweet page** (5-10 seconds)
    - Scroll 1-2 times (200-400px)
8. **Focus on retweet button** using `api.scroll.focus2()`
9. **Execute retweet** via `retweetWithAPI()`

### Behavior

- Most complex flow with multiple navigation phases
- Combines home browsing with profile verification
- Shorter profile read time than Branch 2

---

## Branch 5: Home Browse + Thread Reader

**Weight:** 10% | **Time:** 4-8 min | **Purpose:** Simulates users who read full thread context before retweeting.

### Flow

```
home (2-4 min) → tweet (1-2 min) → "Show thread" → read thread (1-2 min) → back → retweet
```

1. **Navigate to home** via `api.twitter.home()`
2. **Browse home feed** (2-4 minutes)
3. **Navigate to target tweet**
4. **Read tweet page** (1-2 minutes)
    - Scroll through tweet content
5. **Find and click thread button**
    - Looks for `[data-testid="tweet-text-show-show-thread"]`
    - Falls back to `a[href*="/status/"]` link
6. **Read thread** (1-2 minutes)
    - Scroll through thread replies
7. **Navigate back** to original tweet
8. **Focus on retweet button** using `api.scroll.focus2()`
9. **Execute retweet** via `retweetWithAPI()`

### Behavior

- Falls back to direct retweet if no thread found
- Full context reading before sharing
- Longest branch (up to 8 minutes)

---

## Branch 6: Home Browse + Like + Retweet

**Weight:** 10% | **Time:** 4-6 min | **Purpose:** Simulates multi-engagement users who like before retweeting.

### Flow

```
home (2-3 min) → tweet (1-2 min) → like → wait → retweet
```

1. **Navigate to home** via `api.twitter.home()`
2. **Browse home feed** (2-3 minutes)
3. **Navigate to target tweet**
4. **Read tweet** (1-2 minutes)
    - Scroll and pause pattern
5. **Click like button** (`[data-testid="like"]`)
    - Focus first, then click
    - Wait 1-2 seconds after liking
6. **Focus on retweet button** using `api.scroll.focus2()`
7. **Execute retweet** via `retweetWithAPI()`

### Behavior

- Two-action engagement pattern
- Pause between like and retweet
- Skips like if button not found (already liked)

---

## Branch 7: Home + Notification Check + Retweet

**Weight:** 8% | **Time:** 4-7 min | **Purpose:** Simulates users who check notifications before engaging.

### Flow

```
home (1-2 min) → notifications (1-2 min) → tweet (1-2 min) → retweet
```

1. **Navigate to home** via `api.twitter.home()`
2. **Browse home feed** (1-2 minutes)
3. **Navigate to notifications** (`x.com/notifications`)
4. **Browse notifications** (1-2 minutes)
    - Scroll through notifications
    - Random pauses (1-3 seconds)
5. **Navigate to target tweet**
6. **Read tweet** (1-2 minutes)
    - Scroll and pause pattern
7. **Focus on retweet button** using `api.scroll.focus2()`
8. **Execute retweet** via `retweetWithAPI()`

### Behavior

- Adds realism by checking notifications first
- Natural daily habit simulation

---

## Branch 8: Home Browse + Overscroll & Return

**Weight:** 7% | **Time:** 4-6 min | **Purpose:** Simulates users who accidentally scroll past and come back.

### Flow

```
home (2-3 min) → tweet (1-2 min) → overscroll → pause → scroll back → retweet
```

1. **Navigate to home** via `api.twitter.home()`
2. **Browse home feed** (2-3 minutes)
3. **Navigate to target tweet**
4. **Read tweet** (1-2 minutes)
    - Scroll and pause pattern
5. **Overscroll past tweet** (2-3 scrolls down, 300-600px each)
6. **Pause to "realize"** (2-4 seconds)
7. **Scroll back to tweet** using `api.scroll.focus2()`
8. **Focus on retweet button** using `api.scroll.focus2()`
9. **Execute retweet** via `retweetWithAPI()`

### Behavior

- Simulates natural scrolling mistakes
- Pause adds realism to "realization" moment
- Double focus2: once for tweet, once for button

---

## Branch 9: Home Browse + Search to Retweet

**Weight:** 7% | **Time:** 4-7 min | **Purpose:** Simulates users who search before finding content to retweet.

### Flow

```
home (1-2 min) → search page → type query → browse results (1-2 min) → tweet (1-2 min) → retweet
```

1. **Navigate to home** via `api.twitter.home()`
2. **Browse home feed** (1-2 minutes)
3. **Navigate to search** (`x.com/search`)
4. **Type search query** (keyword extracted from URL, fallback: "tech")
5. **Browse search results** (1-2 minutes)
    - Scroll and pause pattern
6. **Navigate to target tweet**
7. **Read tweet** (1-2 minutes)
    - Scroll and pause pattern
8. **Focus on retweet button** using `api.scroll.focus2()`
9. **Execute retweet** via `retweetWithAPI()`

### Behavior

- Search keyword extracted from tweet URL (first path segment)
- Falls back to generic "tech" if extraction fails
- Simulates topic-focused browsing

---

## Branch 10: Home Browse + Impulse Retweet

**Weight:** 6% | **Time:** 3-4 min | **Purpose:** Simulates quick, impulse reactions.

### Flow

```
home (2-3 min) → tweet → quick pause (1-3s) → retweet
```

1. **Navigate to home** via `api.twitter.home()`
2. **Browse home feed** (2-3 minutes)
3. **Navigate to target tweet**
4. **Brief pause** (1-3 seconds)
5. **Focus on retweet button** using `api.scroll.focus2()`
6. **Short pause** (200-500ms)
7. **Execute retweet** via `retweetWithAPI()`

### Behavior

- Minimal interaction time on tweet page
- Home browsing provides context, then quick decision
- Fastest branch completion after home

---

## Branch 11: Multi-Phase Engagement

**Weight:** 5% | **Time:** 5-8 min | **Purpose:** Simulates thorough user engagement pattern across multiple X.com sections.

### Flow

```
home (1-2 min) → notifications (1-2 min) → tweet (1-2 min) → retweet → profile (1-2 min)
```

1. **Browse home feed** via `api.twitter.home()` (1-2 minutes)
2. **Check notifications** (`x.com/notifications`)
    - Browse for 1-2 minutes
    - Scroll and pause pattern
3. **Navigate to target tweet**
4. **Read tweet** (1-2 minutes)
    - Scroll and pause pattern
5. **Focus on retweet button** using `api.scroll.focus2()`
6. **Execute retweet** via `retweetWithAPI()`
7. **Navigate to author's profile** (if retweet successful)
    - Parse username from tweet URL
    - Browse profile (1-2 minutes)

### Behavior

- Most comprehensive engagement pattern
- Home first, then notifications (natural daily routine)
- Post-retweet profile visit shows "interest in author"
- Longest total duration branch

---

## Branch 12: Home Browse + Misclick Recovery

**Weight:** 2% | **Time:** 4-6 min | **Purpose:** Simulates human error and recovery.

### Flow

```
home (2-3 min) → tweet (1-2 min) → misclick (like/reply) → escape → retweet
```

1. **Navigate to home** via `api.twitter.home()`
2. **Browse home feed** (2-3 minutes)
3. **Navigate to target tweet**
4. **Read tweet** (1-2 minutes)
    - Scroll and pause pattern
5. **Simulate misclick** (50/50 chance: like or reply button)
    - Focus and click wrong button
6. **Press Escape** to close/cancel (via `page.keyboard.press('Escape')`)
7. **Wait for recovery** (1-2 seconds)
8. **Focus on retweet button** using `api.scroll.focus2()`
9. **Execute retweet** via `retweetWithAPI()`

### Behavior

- Simulates clicking wrong button
- Keyboard escape to recover
- Lowest weight (2%) - rare human error pattern
- Falls back gracefully if misclick buttons not found

---

## Common Components

### api.twitter.home()

Navigation to home feed with built-in reading:

```javascript
await api.twitter.home({ readDurationMs: 120000 }); // 2 minutes
```

### api.scroll.focus2()

New scroll function that:

- Uses absolute document coordinates (`offsetTop`)
- Compensates for Twitter's fixed header (55px)
- Multi-step scroll with easeOutCubic easing
- Returns verification metrics

### Retweet Button Selection

```javascript
const retweetBtn = page.locator('[data-testid="retweet"]').first();
await retweetBtn.waitFor({ state: 'visible', timeout: TWEET_VISIBLE_TIMEOUT_MS });
await api.scroll.focus2(retweetBtn);
const result = await retweetWithAPI({ page, tweetElement });
```

### Referrer Engine

All branches use `ReferrerEngine` for natural navigation:

```javascript
const referrerEngine = new ReferrerEngine({ addUTM: true });
const refCtx = referrerEngine.generateContext(targetUrl);
await api.goto(targetUrl, { referer: refCtx.referrer || undefined });
```

**Strategies:** google_search (25%), social (30%), direct (10%), messaging (25%), other (10%)

---

## Timeout

Default timeout: **10 minutes** (600000ms)

This accommodates the longest branches (5-8 minutes) plus:

- Navigation: ~7s per page
- Hydration: 3s × multiple pages
- Scroll to button: 3-5s
- Retweet: ~5s

---

## Related Modules

| Module                       | Purpose                                 |
| ---------------------------- | --------------------------------------- |
| `api/twitter/navigation.js`  | `api.twitter.home()` implementation     |
| `api/actions/retweet.js`     | `retweetWithAPI()` execution            |
| `api/utils/urlReferrer.js`   | `ReferrerEngine` for natural navigation |
| `api/interactions/scroll.js` | `api.scroll.focus2()` implementation    |
