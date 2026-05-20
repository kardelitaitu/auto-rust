last audited 21-05-26 by Codex

# Twitter Reply Task

Navigates to one tweet URL, extracts local context, generates a reply, and attempts to post it.

## Quick Start

```bash
cargo run twitterreply=url=https://x.com/user/status/123
cargo run 'twitterreply={"url":"https://x.com/user/status/123"}'
```

## Runtime Summary

- Default task budget is `45000` ms with timing variance.
- The task scrolls down for a random `10-20` seconds, then scrolls back up for `5-10` seconds before extracting context.
- It extracts the main tweet plus up to `5` visible replies.
- Reply generation currently always uses `UnifiedLLMProcessor`.
- If no generated reply is available, the task falls back to `Interesting perspective! Thanks for sharing.`.
- Posting uses the tweet composer and retries submit up to `3` times.

## Payload Parameters

| Parameter | Type | Notes |
|---|---|---|
| `url` | string | Preferred tweet URL input |
| `value` | string | Alternate tweet URL input |
| `default_url` | string | Fallback tweet URL input |

The current runtime does not support a manual `text` override parameter.

## Reply Sanitization

Before typing, the generated reply is normalized by:

- trimming surrounding whitespace
- removing a leading or trailing double quote
- dropping one trailing period
- truncating to `280` characters with ellipsis when needed

## Flow

1. Read the tweet URL from payload.
2. Navigate to the tweet page.
3. Scroll down, then back up, to simulate reading behavior.
4. Extract the main tweet author and text.
5. Extract up to `5` visible replies for context.
6. Generate candidate replies through `UnifiedLLMProcessor`.
7. Sanitize the first generated reply, or use the static fallback text.
8. Open the reply composer, type the reply, and attempt to post it.

## Notes

- This task uses the local DOM on the current tweet page. It does not perform a deeper thread-dive loop like `twitteractivity`.
- The runtime does not currently gate LLM usage behind a task-level config flag in this path.

## Related Tasks

- [`twitteractivity`](twitteractivity.md)
