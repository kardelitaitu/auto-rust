# Twitter Follow Like Retweet Task Documentation

This document explains the workflow and logic of the `twitterFollowLikeRetweet.js` task. This task extends the standard follow behavior by adding engagement actions (Like & Retweet) on the source tweet, simulating a "Super Fan" interaction pattern.

## 1. Configuration

The task is configured via dynamic payload (via `main.js` CLI args) or fallback constants:

- **`targetUrl`**: The specific "entry point" tweet.
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
                                               [Follow Action]
                                            (Check/Click/Verify)
                                                         |
                                                         v
                                              [Back to Tweet]
                                           (CDP History Navigation)
                                                         |
                                                         v
                                         [Retweet Action (Hardened)]
                                       (Golden Zone -> Retry Loop -> Verify)
                                                         |
                                                         v
                                          [Like Action (Hardened)]
                                       (Golden Zone -> Retry Loop -> Verify)
                                                         |
                                                         v
                                                [Cool-down Phase]
                                                (Home Feed Read)
                                                         |
                                                         v
                                                      [End]
```

## 3. Initialization & Navigation

Matches standard `twitterFollow` logic:

1.  **Agent Setup**: Loads `TwitterAgent` with profile-specific headers/fingerprints.
2.  **Anti-Sybil Referrers**: Generates traffic sources (Google, t.co, etc.) to mask direct entry.
3.  **Reading Phase**: Simulates human reading (scrolling, micro-pauses) on the tweet before interacting.

## 4. Profile Interaction (The "Follow")

1.  **Discovery**: Locates the author's name/avatar.
2.  **Navigation**: Clicks through to the profile.
3.  **Follow**: Checks if already following. If not, performs a robust `humanClick` on the Follow button and verifies the text changes to "Following" or the button becomes "Unfollow".

## 5. Engagement Actions (Reliability Hardened)

After following, the agent navigates back to the original tweet to perform engagements.

### A. "Golden Zone" Scrolling

Before clicking any engagement button (Retweet/Like), the agent uses the `goldenZone()` helper.

- **Problem**: Sticky headers or footers often obscure buttons at the very top or bottom of the viewport.
- **Solution**: The element is automatically scrolled to **30% of the viewport height** (approx. 1/3 down the screen). This ensures a clear, unobstructed click path.

### B. Atomic Retry Loops

Interactions are wrapped in a 3-attempt retry loop to handle network flics or UI lag.

1.  **Check**: verified if already Retweeted/Liked (checking for Green/Pink states or "Un-" buttons).
2.  **Attempt**: If not, it attempts the action.
3.  **Retry**: If the menu (for Retweet) doesn't appear or the click doesn't register, it waits and retries up to 3 times.

### C. Visual State Verification

The task uses a `verifyVisualState()` helper (optional probe) and strict selector checks to confirm success.

- **Retweet**: Verifies the "Retweet Confirm" menu appears, clicks it, then verifies the button changes to "Unretweet".
- **Like**: Verifies the button changes to "Unlike" and (optionally) checks the heartbeat icon color (Pink `rgb(249, 24, 128)`).

## 6. Cooldown & Exit

1.  **Return Home**: Navigates back to the persistent "Home" feed.
2.  **Cool-down Reading**: Spends 1-2 minutes reading the timeline.
3.  **Session Check**: Performs a final `checkLoginState()` to warn if the session was invalidated during the task.
