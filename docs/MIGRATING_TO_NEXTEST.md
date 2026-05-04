# Migrating to cargo-nextest

## Todo Checklist for Implementation

- [x] Install cargo-nextest in GitHub Actions workflow
- [x] Replace `cargo test` with `cargo nextest run` in CI
- [x] Create optional `Nextest.toml` configuration file
- [x] Verify test parity between vanilla test and nextest
- [x] Update contributing documentation (if applicable)
- [x] Monitor CI performance improvements

## Why Migrate to cargo-nextest?

### Limitations of Vanilla `cargo test`
- Executes tests sequentially per test binary (limited parallelism)
- No intelligent test scheduling or prioritization
- Minimal diagnostics for flaky tests
- No built-in retry mechanism
- Scales poorly with large test suites (>1000 tests)

### Advantages of cargo-nextest
- **Significant Speed Improvements**: 2-10x faster test execution via intelligent parallelization
- **Smart Test Scheduling**: Prioritizes likely-to-fail tests and avoids CPU oversubscription
- **Enhanced Diagnostics**: Clear test lists, timing summaries, leak detection
- **Flaky Test Handling**: Automatic retry mechanisms and isolation
- **CI Optimizations**: Test sharding, JUnit/XML reporting, fail-fast modes
- **Backward Compatibility**: Runs identical test binaries with zero code changes

## Implementation Details

### GitHub Actions Integration
Replace the test step in `.github/workflows/ci.yml`:

```yaml
- name: Install cargo-nextest
  run: cargo install cargo-nextest

- name: Run tests with nextest
  run: cargo nextest run --release
```

### Configuration (Optional)
Create `Nextest.toml` for project-specific settings:

```toml
[profile.default]
# Increase timeout for integration tests
timeout = 90

# Retry flaky tests up to 3 times
retries = 3

# Fail fast in CI to save resources
fail-fast = true

# Report slowest tests for optimization
slowest = { threshold = "2s", num = 10 }

[profile.ci]
# Override for CI-specific settings
timeout = 60
retries = 1
```

### Verification Strategy
1. Run locally to ensure parity:
   ```bash
   cargo test          # vanilla
   cargo nextest run   # nextest
   ```
2. Confirm identical test pass/fail results
3. Monitor CI for reduced execution time
4. Check for any timing-dependent test failures (reveals pre-existing flakiness)

## Expected Outcomes
- **Short-term**: 30-70% reduction in CI test execution time
- **Medium-term**: Better visibility into test performance and flakiness
- **Long-term**: Foundation for advanced testing strategies (sharding, parallelization across jobs)

## Risks and Mitigations
| Risk | Mitigation |
|------|------------|
| Timing-dependent test failures | These reveal real flakiness; investigate and fix underlying issues |
| Configuration complexity | Start with defaults; add configuration only as needed |
| Toolchain dependency | Minimal impact - single binary installation with caching |
| Output format differences | Team adapts quickly to clearer, more actionable nextest output |

## Recommendation
Given our current test suite (~1840 tests) and clean CI state, migration provides immediate speed benefits with negligible risk. The backward-compatible nature ensures zero test code changes while improving developer feedback loops.

---
*Last updated: $(date +%Y-%m-%d)*