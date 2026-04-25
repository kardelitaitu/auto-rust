# Twitter Follow Task

Navigate to Twitter/X profiles and follow users with human-like behavior.

## Quick Start

```bash
# Follow from profile URL
cargo run twitterfollow=url=https://x.com/username

# Follow from tweet URL (navigates to profile)
cargo run twitterfollow=url=https://x.com/user/status/123

# Follow with username directly
cargo run 'twitterfollow={"username":"username"}'
```

## Features

- 🎯 **Smart URL Detection**: Handles profile URLs, tweet URLs, usernames
- ✅ **Already Following Check**: Prevents duplicate follows
- ⏳ **Pending State Handling**: Waits for follow confirmation
- 🔄 **Retry Logic**: Page reload on failure (up to 7 attempts)
- 🛡️ **Rate Limit Detection**: Skips actions when rate-limited
- 👻 **Popup Dismissal**: Handles login/signup modals

## Rate Limit Signals Detected

- "Rate limit"
- "Too many attempts"
- "Try again later"
- "You have been rate limited"
- "Temporary restriction"
- "Something went wrong"
- "Unable to follow"

## Payload Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `url` | string | Profile URL or tweet URL |
| `username` | string | Twitter username (alternative to URL) |

## How It Works

1. Parses URL or username to determine target profile
2. Navigates to profile page
3. Dismisses any popups/modals
4. Checks if already following
5. Clicks follow button with human-like timing
6. Waits for confirmation
7. Retries on transient failures (max 7 attempts)

## Error Handling

| Scenario | Behavior |
|----------|----------|
| Already following | Gracefully skips |
| Rate limited | Logs warning, skips action |
| Follow button not found | Retries with page reload |
| Account suspended | Reports failure |

## Example Use Cases

```bash
# Follow single user
cargo run twitterfollow=url=https://x.com/rustlang

# Follow from tweet (extracts author)
cargo run twitterfollow=url=https://x.com/user/status/123456

# Follow by username only
cargo run 'twitterfollow={"username":"rustlang"}'
```
