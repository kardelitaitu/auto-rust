# Twitter Intent Task

## Overview

The `twitterintent` task handles Twitter/X intent URLs for automated actions including follow, like, post, quote, and retweet. It automatically detects the intent type from the URL and performs the appropriate action with human-like timing.

## Task Sequence

### 1. URL Extraction
- Extracts intent URL from payload (supports `url`, `value`, or fallback to any field containing `x.com` or `twitter.com`)

### 2. Intent Type Detection
Automatically detects intent type from URL path:
- `/intent/follow` → Follow intent
- `/intent/like` → Like intent
- `/intent/tweet` (with `url=` parameter) → Quote intent
- `/intent/tweet` (without `url=` parameter) → Post intent
- `/intent/retweet` → Retweet intent

### 3. Navigation
- Navigates to intent URL using `api.navigate()` (includes trampoline/referrer)
- Waits 2 seconds for page load

### 4. Click Verification
- Checks if confirm button is visible before clicking
- If not visible, returns failure (may already be clicked)
- Random 4-8 second pause before clicking (human-like delay)
- Clicks the button using `api.click()` (human-like cursor movement)
- Waits 1 second for action to process
- Verifies success by checking if button disappears

### 5. Post-Action Pause
- Random 5-10 second pause after intent action (applies to both success and failure)
- Logs the actual pause duration

### 6. Return to Previous Page
- Uses JavaScript `window.history.back()` to return to previous page

## Intent Types and Selectors

| Intent Type | URL Pattern | Confirm Selector | Button Text |
|-------------|-------------|------------------|-------------|
| **Follow** | `/intent/follow?screen_name={username}` | `[data-testid="confirmationSheetConfirm"]` | Follow @{username} |
| **Like** | `/intent/like?tweet_id={tweetId}` | `[data-testid="confirmationSheetConfirm"]` | Like |
| **Post** | `/intent/tweet?text={encodedText}` | `[data-testid="tweetButton"]` | Post |
| **Quote** | `/intent/tweet?url={url}&text={encodedText}` | `[data-testid="tweetButton"]` | Reply |
| **Retweet** | `/intent/retweet?tweet_id={tweetId}` | `[data-testid="confirmationSheetConfirm"]` | Repost |

## Intent URL Examples

### Follow
```
https://x.com/intent/follow?screen_name=elonmusk
```

### Like
```
https://x.com/intent/like?tweet_id=2047854858305405321
```

### Post
```
https://x.com/intent/tweet?text=Hello%20world
```

### Quote
```
https://x.com/intent/tweet?url=https://x.com/user/status/123&text=Great%20tweet
```

### Quote with Reply (Multi-line)
```
https://x.com/intent/tweet?text=this+is+example+reply%0Athis+is+second+line&in_reply_to=2047854858305405321
```
- `%0A` represents line break

### Retweet
```
https://x.com/intent/retweet?tweet_id=2047854858305405321
```

## Usage

```bash
# Follow a user
cargo run -- twitterintent=https://x.com/intent/follow?screen_name=elonmusk

# Like a tweet
cargo run -- twitterintent=https://x.com/intent/like?tweet_id=123456789

# Post a tweet
cargo run -- twitterintent=https://x.com/intent/tweet?text=Hello%20world

# Quote a tweet
cargo run -- twitterintent=https://x.com/intent/tweet?url=https://x.com/user/status/123&text=Great

# Retweet
cargo run -- twitterintent=https://x.com/intent/retweet?tweet_id=123456789
```

## Timing Summary

| Step | Duration |
|------|----------|
| Post-navigation wait | 2 seconds (fixed) |
| Pre-click pause | 4-8 seconds (random) |
| Post-click processing | 1 second (fixed) |
| Post-action pause | 5-10 seconds (random) |

## Trampoline/Referrer

The task uses the existing trampoline functionality through `api.navigate()`, which:
- Randomly selects a referrer from a list (Google, Bing, Reddit, X.com, etc.)
- Adds human-like pause before navigation
- Helps avoid detection

## Verification Logic

Click verification checks if the confirm button disappears after clicking:
- **Success:** Button disappears → Returns `true`
- **Failure:** Button still visible → Returns `false` (may already be clicked or action already performed)

## Logging

The task logs:
- Intent URL and detected intent type
- Selector being clicked
- Click outcome
- Pause durations (pre-click and post-action)
- Verification result (success/failure)
- Navigation actions
