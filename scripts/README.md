# Scripts

Utility scripts for the auto-rust project. Run from the project root.

## Quality Tools

| Script | Purpose |
|--------|---------|
| `miri.ps1` | Dynamic analysis — detect undefined behavior via cargo-miri (requires nightly) |
| `coverage.ps1` | Coverage-guided gap analysis via cargo-tarpaulin (in root) |
| `mutants.ps1` | Mutation testing via cargo-mutants (in root) |

## CI Gates

| Script | Purpose |
|--------|---------|
| `check.ps1` | Full CI suite: spec-lint, build, fmt, clippy, tests (in root) |
| `check-fast.ps1` | Scoped fast check for iteration (changed files only, in root) |

## Spec Tools

| Script | Purpose |
|--------|---------|
| `spec-lint.ps1` | Validate spec package structure (in root) |
| `spec-stash.ps1` | Create named git stash checkpoint (in root) |
| `spec-restore.ps1` | Restore from named git stash checkpoint (in root) |
| `spec-archive.ps1` | Archive completed spec packages (in root) |

## Test Runners

| Script | Purpose |
|--------|---------|
| `run-twitter-tests.ps1` | Twitter module TDD helper (red-green-refactor) |
| `run-integration-tests.ps1` | Integration test orchestrator |

## Build & Docs

| Script | Purpose |
|--------|---------|
| `benchmark-builds.ps1` | Build time measurement |
| `docs.ps1` | Generate cargo doc documentation |
| `performance.ps1` | Performance profiling |

## Automation Scripts

| Script | Purpose |
|--------|---------|
| `setup-windows.bat` | Windows environment setup |
| `bacon-auto-loop.bat` | Bacon pipeline auto-loop |

## Migration/Wiring Scripts (Python)

One-shot migration scripts from the `duration_ms` and delegation refactors.
Kept for reference — no longer needed to run.

| Script | Purpose |
|--------|---------|
| `fix_dead_code.py` | Dead code removal automation |
| `fix_mul_duration_usage.py` | Duration multiplication fix |
| `fix_click.py` | Click interaction fix |
| `fix_default_feed_scan.py` | Default feed scan fix |
| `fix_focus_delegation.py` | Focus delegation fix |
| `fix_probability_assignments.py` | Probability assignment fix |
| `fix_policy_tests.py` | Policy test fix |
| `fix_nav_delegations.py` | Nav delegation fix (pass 1) |
| `fix_nav_delegations2.py` | Nav delegation fix (pass 2) |
| `fix_nav_delegations3.py` | Nav delegation fix (pass 3) |
| `fix_remaining_duration_ms.py` | Remaining duration_ms fix |
| `fix_task_context_duration_ms.py` | Task context duration_ms fix |
| `fix_validate_config_duration_ms.py` | Validate config duration_ms fix |
| `wire_policy_max_duration_ms.py` | Policy max duration wiring |
| `wire_native_interaction_duration_ms.py` | Native interaction duration wiring |
| `wire_duration_ms_orchestrator.py` | Orchestrator duration wiring |
| `wire_duration_ms_discovery_feed.py` | Discovery feed duration wiring |
| `show_dsl_coverage.py` | DSL coverage analysis |
