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
