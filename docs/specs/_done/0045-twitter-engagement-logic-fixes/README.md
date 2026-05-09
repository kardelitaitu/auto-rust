# Twitter Engagement Logic Fixes

Status: `done`

Owner: `spec-agent`
Implementer: `pending`

## Summary

The core engagement loop in `src/utils/twitter/twitteractivity_engagement.rs` contains three critical logical flaws. First, the depth-first reply engagement bypasses the `dry_run_actions` flag, causing real mutations during dry runs. Second, the action selection logic truncates to a single action using an ordered list, causing retweets to "starve" replies and quotes, preventing the bot from performing natural multi-actions (e.g., Like + Reply). Finally, there is unreachable "dead code" in the Like execution path. This spec fixes these logical errors to ensure safe, multi-action, and accurate engagement.

## Scope

- In scope:
  - Modifying `engage_replies` to respect `task_config.dry_run_actions`.
  - Refactoring `process_candidate` to allow executing multiple selected actions on a single tweet.
  - Ensuring that if *any* selected action requires a thread dive, the dive occurs exactly once before actions are executed.
  - Removing the unreachable `did_dive` check inside the `like` action block.
- Out of scope:
  - Changing the actual probability algorithms or limits tracking.
  - Changing DOM selectors or interaction primitives.

## Files

- `spec.yaml`
- `plan.md`
- `validation.md`
- `notes.md`

## Rules

- Keep the spec short.
- Run `spec-lint.ps1` before handoff.
- Use `.\check-fast.ps1` while iterating.
- Use the archive helper `.\spec-archive.ps1` to move to `_done/`.

## Next Step

Wait for the implementer agent to execute the logic fixes.
