# Browser Discovery / Session Assembly

Status: `done`

Owner: `spec-agent`
Implementer: `implementation-agent`

## Summary

Split browser discovery and session assembly into explicit connector, factory, and pool boundaries so startup becomes easier to reason about, easier to test, and less tightly coupled to `browser.rs`.

## Scope

- In scope:
  - connector abstraction for discovery and connect flows
  - session factory ownership of `Session` construction
  - session pool management for retry and parallel discovery
  - browser capability detection attached to sessions
  - preserving current browser filter behavior and discovery semantics
- Out of scope:
  - CLI parsing and task registry work
  - runtime shutdown work
  - task execution or interaction refactors
  - new browser driver support beyond the existing discovery sources

## Files

- `spec.yaml`
- `baseline.md`
- `internal-api-outline.md`
- `plan.md`
- `validation-checklist.md`
- `ci-commands.md`
- `decisions.md`
- `quality-rules.md`
- `implementation-notes.md`

## Next Step

Hand this package to the implementer agent after the CLI spec is available.

# Baseline

## Current State

- `src/browser.rs` owns browser discovery, browser filter matching, connection, and retry loops.
- `discover_browsers_with_filters` currently walks configured profiles, RoxyBrowser discovery, and local discovery in one flow.
- `connect_to_browser` constructs `Session` directly in `browser.rs`.
- `Session::new` currently owns the session bootstrap behavior in `src/session/mod.rs`.
- `BrowserConfig` already carries connection timeout, retry timing, profile list, RoxyBrowser config, native interaction, and session worker settings.

## Known Gaps

- Discovery strategy and session assembly are coupled in one file.
- There is no connector trait or session factory boundary yet.
- Session capability data is not modeled as a first-class property on the session.
- Parallel discovery and retry policy are still embedded in `browser.rs`, which makes testing the startup path more difficult.

## Why This Matters

Browser startup is one of the most failure-prone parts of the system. Breaking discovery and assembly into clearer boundaries should make startup more reliable and much easier to test without changing the rest of the automation flow.

