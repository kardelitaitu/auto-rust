# Twitter Follow Task Documentation

This document explains the workflow and logic of the `twitterFollow.js` task. This task is designed to simulate a highly realistic user journey: discovering a specific tweet, reading it, clicking through to the author's profile, and eventually following them.

## 1. Configuration

The task is configured via constants at the top of the file:

- **`TARGET_TWEET_URL`**: The specific "entry point" tweet.
- **`MANUAL_REFERRER`**: Optional override to simulate traffic coming from a specific external site (e.g., Reddit), bypassing the dynamic referrer engine 20% of the time.

## 2. Logic Flow Diagram

```text
[Start Task] --> [Init Agent & Profile] --> [Warm-up Jitter]
                                                  |
                                                  v
                                            [Login Check]
                                            /           \
                                          Fail         Pass
                                           |             |
                                         [End]    [Referrer Engine]
                                                  (Manual or Dynamic)
                                                         |
                                                         v
                                              [Navigate to Tweet]
                                                         |
                                                         v
                                            [Read Tweet Thread]
                                           (Entropy-based Reading)
                                                         |
                                                         v
                                               [Click Profile]
                                              (humanClick + curve)
                                                         |
                                                         v
                                              [Read Profile Bio]
                                                         |
                                                         v
                                            [Check Follow Status]
                                            /                   \
                                    [Already Following]      [Not Following]
                                            |                      |
                                            |               [robustFollow()]
                                            |               (See 99.99% Flow)
                                            |                      |
                                            +-----------+----------+
                                                        |
                                                        v
                                                [Cool-down Phase]
                                                (Home Feed Read)
                                                        |
                                                        v
                                                      [End]
```

### 99.99% Follow Flow (robustFollow)

```text
                    [Wait 5-10s] (reading profile)
                           |
                           v
              ┌──────────────────────────┐
              │    PRE-FLIGHT CHECKS     │
              │  • Dismiss overlays      │
              │  • Scroll to Golden Zone │
              │  • Check actionability   │
              └──────────────────────────┘
                           |
                           v
              ┌──────────────────────────┐
              │   6-LAYER CLICK CHAIN    │
              │ 1. Ghost Click           │
              │ 2. Native Click          │
              │ 3. Force Click           │
              │ 4. JS Dispatch           │
              │ 5. MouseEvent            │
              │ 6. Keyboard (Enter)      │
              │ (300-800ms between each) │
              └──────────────────────────┘
                           |
                           v
              ┌──────────────────────────┐
              │   POLL VERIFICATION      │
              │ Check every 500ms × 10   │
              │ for Unfollow button or   │
              │ "Following" text         │
              └──────────────────────────┘
                           |
                    Success? ──────> [Done ✅]
                           |
                           No
                           |
              ┌──────────────────────────┐
              │   EXPONENTIAL BACKOFF    │
              │ 1.5s + 1s×attempt        │
              │ ± 500ms jitter           │
              └──────────────────────────┘
                           |
                           v
                    Retry (up to 5×)
                           |
                    All failed?
                           |
              ┌──────────────────────────┐
              │      PAGE RELOAD         │
              │ Wait 2-5s → Reload       │
              │ → 2 more attempts        │
              └──────────────────────────┘
                           |
                           v
                    Still failed?
                           |
                     [Fail ❌]
```

## 3. Initialization Phase

1.  **Agent Setup**: Loads the `TwitterAgent`, applying a specific or random `Profile`.
2.  **Theme Enforcement**: Sets the browser color scheme (Dark/Light/Dim) to match the profile's preference.
3.  **Humanization Patch**: Injects low-level browser overrides (WebGL, Audio API, etc.) to mask automation fingerprints.
4.  **Warm-up Jitter**: Waits for a random duration (2-8s) after browser launch. This decouples the "browser started" timestamp from the "website request" timestamp, a common bot signal.

## 4. Login Verification (The "3-Strike" Rule)

Before navigating to the target, the agent checks if it is logged in.

- If not already on X.com, it visits the Home page.
- It attempts to detect login state up to **3 times**.
- **Fail-Safe**: If it detects the user is logged out (Login buttons visible, "Oops" text) 3 times in a row, the task **aborts immediately**. This prevents "zombie" sessions from trying to run against a login wall.

## 5. Navigation (Anti-Sybil Referrers)

The task uses the `ReferrerEngine` to generate a realistic navigation context.

- **Dynamic Referrers**: It mimics coming from search engines (Google, Bing) or social media (Reddit, Discord, t.co) rather than a direct, robotic "type-in".
- **Headers**: Sets specific `Referer` and `Sec-Fetch-*` headers to match the simulated source.
- **Wait for Content**: Explicitly waits for the `article[data-testid="tweet"]` element to ensure the page has loaded (hydrated) before acting.

## 6. The "Reading" Phase (Tweet)

Instead of a simple "scroll down", the agent uses the `simulateReading()` engine.

- **Behavior**: It slowly scrolls through the thread, simulating reading speed (Gaussian distribution).
- **Fidgets**: Includes random mouse movements, text highlighting, or micro-pauses that a real human would make while reading a thread.
- **Duration**: 5-10 seconds.

## 7. Profile Dive

1.  **Discovery**: Locates the author's name/avatar link within the tweet structure.
2.  **Aggressive Scroll (`humanClick`)**:
    - The agent forces the profile link to scroll to the **center of the viewport**.
    - It waits a fraction of a second (visual reaction time).
    - It uses the **Ghost Cursor** (Bezier curve movement) to click the link.
    - If the element moves (layout shift), the cursor tracks it.
3.  **Profile Reading**: Once on the profile, it simulates reading the bio and pinned tweets for roughly 15 seconds.

## 8. The Follow Action (99.99% Success Rate)

The task uses `agent.robustFollow()` with a 6-layer click strategy:

### Pre-Flight Checks

1. **Initial Delay**: Wait 5-10s (simulates "reading the profile")
2. **Dismiss Overlays**: Press Escape to clear toasts/modals
3. **Golden Zone Scroll**: Position button at 30% viewport height
4. **Actionability Check**: Verify button is not covered by overlay

### 6-Layer Click Fallback

| Layer | Method       | Description                      |
| ----- | ------------ | -------------------------------- |
| 1     | Ghost Click  | Human-like Bezier curve movement |
| 2     | Native Click | Standard Playwright `click()`    |
| 3     | Force Click  | `click({ force: true })`         |
| 4     | JS Dispatch  | `element.click()` via evaluate   |
| 5     | MouseEvent   | Dispatch `MouseEvent('click')`   |
| 6     | Keyboard     | Focus + Enter key                |

**Delay**: 300-800ms between each layer.

### Verification

- **Polling**: Check every 500ms for 5s (10 polls)
- **Checks**: Unfollow button, button text, aria-label

### Retry & Reload

- **5 attempts** with exponential backoff (1.5s + 1s×attempt ± 500ms)
- **Page reload** after all 5 fail
- **2 post-reload attempts**
- **Maximum**: 7 total attempts

## 9. Cooldown & Exit

1.  **Return Home**: Navigates back to the persistent "Home" feed.
2.  **Cool-down Reading**: Spends 1-2 minutes reading the timeline. This "washes" the targeted action with generic, low-frequency organic behavior, masking the intent of the session.
3.  **Cleanup**: Closes the page and reports metrics.
