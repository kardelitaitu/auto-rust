# Plan

## Step 1: Baseline and contract

- Confirm the current discovery flow in `browser.rs` and the current `Session::new` boundary.
- Re-read the browser filter and session startup tests already in the codebase.
- Lock the promise that filter semantics stay the same.

## Step 2: Connector and capability boundaries

- Introduce a connector trait for browser discovery and connection sources.
- Add a capability model that travels with the session.
- Keep connector-specific details out of `browser.rs`.

## Step 3: Factory and pool manager

- Move session construction into a dedicated factory.
- Add a pool manager for parallel discovery and retry coordination.
- Preserve the current priority order of discovery sources.

## Step 4: Tests and regression checks

- Add unit tests for connector matching and capability mapping.
- Add tests for session factory construction and pool retry behavior.
- Keep one narrow integration test if a real startup path must be verified.

## Step 5: Verification

- Run `.
check-fast.ps1` during implementation.
- Run `.
check.ps1` before handoff.
- Move the spec to `_done/` only after the full gate passes.

# Internal API Outline

## Ownership Boundaries

- `src/browser.rs`
  - Keep top-level discovery orchestration and backward-compatible entry points.
  - Delegate source-specific work to connectors and factories.

- `src/session/connector.rs`
  - Own the `BrowserConnector` trait and the connector implementations.
  - Expose a small discovery/connect interface.

- `src/session/factory.rs`
  - Own `SessionFactory` and `BrowserCapabilities`.
  - Build sessions from connector output and config data.

- `src/session/pool.rs`
  - Own `SessionPoolManager` and parallel discovery/retry behavior.
  - Keep retry and merge semantics explicit.

- `src/session/mod.rs`
  - Extend `Session` with capability data and keep its constructor boundary thin.

- `src/config/mod.rs`
  - Keep browser profile, RoxyBrowser, and worker settings as the input source.

## API Shape Rules

- Keep discovery filters stable.
- Make connector selection explicit enough to test without real browsers.
- Preserve current timeout and retry semantics unless a test proves a safer equivalent.
- Attach capabilities to the session so downstream code can branch without re-discovering environment facts.

# Decisions

- Keep `browser.rs` as the orchestration entry point.
- Use explicit connector and factory boundaries instead of one large discovery function.
- Attach browser capabilities to `Session` rather than rediscovering them at call sites.
- Preserve filter and retry semantics by default.
- Prefer deterministic tests with mocks over browser-heavy timing tests.

