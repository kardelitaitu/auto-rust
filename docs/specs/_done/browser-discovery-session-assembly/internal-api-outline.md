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
