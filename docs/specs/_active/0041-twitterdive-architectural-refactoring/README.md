# TwitterDive Architectural Refactoring

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary

The `twitterdive` task currently bypasses core `TaskContext` APIs, using raw JavaScript for scrolling and external modules for pauses. It also relies on flawed DOM logic for end-of-thread detection that fails in virtualized lists. This spec refactors `twitterdive.rs` to use internal capabilities (`scroll::human_scroll`, `api.pause_human()`) and robust scroll-delta checks to ensure human-like behavior and accurate metric tracking without changing the core business goal.

## Scope

- In scope:
  - Replacing raw JS `window.scrollBy` with `crate::capabilities::scroll` methods.
  - Replacing bespoke/uniform pauses with `api.pause_human()`.
  - Fixing `check_end_of_thread` to use scroll deltas instead of node counts.
  - Fixing `tweets_read` to track unique items instead of scroll attempts.
  - Cleaning up imports to use `crate::prelude::*`.
- Out of scope:
  - Changing the overall duration budget or payload schema.
  - Adding new engagement features (like/retweet) to the dive task itself.
  - Modifying the underlying scroll capabilities implementation.

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

Wait for implementer to pick up the task and execute the refactoring plan.
