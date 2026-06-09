last audited 21-05-26 by Codex

# Twitter Follow Task

Navigates to a profile or tweet author and follows the target if the account is not already followed.

## Quick Start

```bash
cargo run twitterfollow=url=https://x.com/username
cargo run twitterfollow=url=https://x.com/user/status/123
cargo run 'twitterfollow={"username":"username"}'
```

## Runtime Summary

- Default task budget is `45000` ms with timing variance.
- The task accepts a profile URL, a tweet URL, a `username` field, or a generic `value` field.
- Tweet URLs trigger a tweet-to-profile flow before follow verification begins.
- The task checks for an already-following state before attempting any follow click.
- A humanized `8-15` second pause happens before the follow action phase.
- The retry path performs up to `5` in-page attempts, then allows one reload recovery transition.
- The reload transition does not reset the attempt counter, so it is a narrow recovery path rather than a fresh second attempt budget.

## Soft Error Signals

The runtime checks page text for these signals before retrying:

- `rate limit`
- `too many attempts`
- `try again later`
- `you have been rate limited`
- `temporary restriction`
- `something went wrong`
- `unable to follow`

## Payload Parameters

| Parameter | Type | Notes |
|---|---|---|
| `url` | string | Profile URL or tweet URL |
| `username` | string | Direct username input |
| `value` | string | Alternate direct username input |

## Flow

1. Normalize the incoming target.
2. If the target is a tweet URL, navigate to the tweet and extract the author path.
3. Otherwise navigate straight to the profile URL.
4. Verify that the current profile matches the expected username.
5. If already following, exit without clicking.
6. Dismiss overlays, check soft-error signals, and look for follow-button candidates.
7. Click follow and poll for the post-click following state.

## Notes

- Pending-follow detection is part of the verification path, not a separate locator strategy.
- The reload recovery branch is intentionally conservative and should be kept in sync with the unit test in `twitterfollow.rs`.

## Related Tasks

- [`twitteractivity`](twitteractivity.md)
- `twitterintent`
