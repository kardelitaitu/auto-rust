# PageView Task

Navigates to web pages and simulates human-like browsing behavior.

## Quick Start

```bash
# Navigate to URL
cargo run pageview=www.reddit.com

# With custom duration (2 minutes)
cargo run pageview=url=https://example.com,duration_ms=120000

# With custom scroll behavior
cargo run pageview=url=https://example.com,scroll_read_amount=400,scroll_read_pauses=3
```

## Payload Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `url` | string | required | Target URL |
| `value` | string | legacy alias | Alternate target URL input |
| `duration_ms` | u64 | 120000 | Task duration in milliseconds |
| `initial_pause_ms` | u64 | 1000 | Initial delay before actions |
| `selector_wait_ms` | u64 | 6000 | Wait for visible content |
| `cursor_interval_min_ms` | u64 | from profile | Min cursor move interval |
| `cursor_interval_max_ms` | u64 | from profile | Max cursor move interval |
| `scroll_interval_min_ms` | u64 | from profile | Min scroll interval |
| `scroll_interval_max_ms` | u64 | from profile | Max scroll interval |
| `scroll_read_pauses` | u32 | 2 | Pauses during scroll read |
| `scroll_read_amount` | i32 | 650 | Scroll amount per burst |
| `scroll_read_variable_speed` | bool | true | Variable scroll speed |
| `scroll_read_back_scroll` | bool | false | Scroll back to top when done |

## Behavior

- Human-like scroll patterns with pauses
- Random cursor movements between scrolls
- Respects persona timing profiles
- Variable speed scrolling for realism

## Examples

```bash
# Quick visit (1 minute)
cargo run pageview=example.com,duration_ms=60000

# Deep reading with many pauses
cargo run pageview=example.com,scroll_read_pauses=5,scroll_read_amount=300

# Return to top when done
cargo run pageview=example.com,scroll_read_back_scroll=true
```
