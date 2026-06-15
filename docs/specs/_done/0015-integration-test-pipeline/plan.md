# Integration Test Pipeline with Browser WebSocket Support

## Baseline

### What I Find
1. **25 `#[ignore]` integration tests** across 3 files cannot run in CI or normal dev workflows because they require a real browser CDP WebSocket connection
2. **Duplicated code**: `connect_test_browser()` is copy-pasted identically in `navigation_integration.rs:23-55` and `task_context_integration.rs:19-49`
3. **No automation script**: Developers must manually start a browser, set env vars, and run tests
4. **orchestrator_integration.rs** has 5 additional ignored tests that use `discover_browsers()` — a different entry point that still requires real browser instances
5. **query.rs** already has a delegation-based test infrastructure (string table tests) but the browser round-trip tests remain uncovered in CI

### What I Claim
Adding a shared test helper and a one-command PowerShell script will enable developers to run all integration tests locally with zero manual setup, and provide a clear path for CI integration.

### What Is the Proof
1. `connect_test_browser()` is 30 lines duplicated across 2 files — extracted once, it reduces maintenance surface
2. No existing script automates browser launch + test run — creating one closes the gap between "ignored" and "usable"
3. The orchestrator tests already use `#[ignore]` as a skip mechanism — a script that conditionally enables them is the only missing piece

## What Is the Solution

### Step 1: Extract shared helper
Move `connect_test_browser()` into `tests/common/mod.rs` as a public function. Update both test files to import from `common`.

### Step 2: Create orchestration script
Create `scripts/run-integration-tests.ps1` that:
1. Detects available browsers (Chrome, Brave, Edge)
2. Launches one with `--headless --remote-debugging-port=9222`
3. Sets `TASK_API_TEST_WS` env var
4. Runs `cargo test --test navigation_integration -- --ignored --test-threads=1`
5. Runs `cargo test --test task_context_integration -- --ignored --test-threads=1`
6. Runs `cargo test --test orchestrator_integration -- --ignored --test-threads=1`
7. Kills the browser process

### Step 3: Fix orchestrator_integration.rs
Replace `get_available_sessions()` (which calls `discover_browsers()`) with the same `connect_test_browser()` approach. The orchestrator tests need an `Orchestrator`, which requires a config — but they don't actually need multiple sessions. A single browser connection is sufficient.

### Step 4: Add documentation
Add a section to `docs/CONTRIBUTING.md` describing how to run integration tests.

## API Changes
No production API changes. Only test infrastructure changes.

## Design Decisions and Risks
- **Why a script instead of CI config?** CI-agnostic — works locally and can be adapted to any CI system.
- **Why `--test-threads=1`?** A single browser page can only handle one navigation at a time.
- **Why headless?** CI environments typically lack a display server.
- **What about macOS/Linux?** The script is PowerShell-only for now. Add a shell script in a follow-up.
- Confidence: **High**

## Validation
- `scripts/run-integration-tests.ps1` runs all 25+ tests successfully
- `cargo test` (non-ignored) still passes all 3466+ tests
- `cargo clippy` shows no new warnings on test code
