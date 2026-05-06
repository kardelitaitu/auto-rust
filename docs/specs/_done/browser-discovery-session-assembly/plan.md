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
