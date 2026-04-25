# CookieBot Task

Manages browser cookies and consent dialogs. Visits URLs from `data/cookiebot.txt`.

## Quick Start

```bash
cargo run cookiebot
```

## Data File

Create `data/cookiebot.txt`:

```
https://example1.com
https://example2.com
# This is a comment
https://example3.com
```

## How It Works

1. Reads URLs from `data/cookiebot.txt` (one per line, `#` for comments)
2. Navigates to each URL with human-like timing
3. Handles cookie/consent dialogs automatically
4. Uses `cookiebot` resource-blocking behavior internally

## Use Cases

- Pre-warming browser sessions with cookies
- Managing consent dialogs before main tasks
- Building browsing history for realistic profiles
