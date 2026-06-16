# Contributing Guide

last audited 13-05-26 by Buffy

Thank you for contributing to the Rust Orchestrator!

## Development Setup

### Prerequisites

- Rust 1.70+ ([Install via rustup](https://rustup.rs/))
- Brave browser with remote debugging enabled (for local testing)
- Optional: RoxyBrowser API access

### Build

```bash
# Clone
git clone <repository-url>
cd auto

# Build
cargo build --all-features

# Run tests
cargo test

# Check lints
cargo clippy --all-targets --all-features
```

## Making Changes

### Bacon Pipeline (Recommended)

The Bacon gated-LLM pipeline automates code changes through 4 stages:
1. **Observer** — scans for approved specs or generates improvement ideas
2. **Strategist** — creates a spec package with plan and validation criteria
3. **Coder** — applies SEARCH/REPLACE blocks and verifies with `check-fast.ps1`
4. **Auditor** — reviews semantic correctness and compliance

Spec packages use a streamlined **3-file format**:
- `spec.yaml` — metadata (title, status, owner, acceptance criteria)
- `plan.md` — implementation steps and scope
- `validation.md` — acceptance criteria and audit results

Manual spec creation for non-trivial work:
1. Create a spec directory in `docs/specs/_active/<initiative>/`
2. Write `spec.yaml`, `plan.md`, and (optionally) `validation.md`
3. Run the pipeline with `bacon --spec <number>` or the full 4-stage flow
4. Move the folder to `_done/` only after `./check-fast.ps1` passes

### Code Style

- Follow Rust best practices and idioms
- Run `cargo fmt` before committing
- Address all `cargo clippy` warnings
- Add documentation comments (`///`) for public APIs

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_shutdown_channel_signal

# With output
cargo test -- --nocapture
```

### Integration Tests

Browser-backed integration tests require a running Chromium-based browser with CDP:
```powershell
.\scripts\run-integration-tests.ps1
```
This launches a headless Chrome/Brave/Edge on port 9222, runs the ignored
integration tests, and cleans up.

Use `-TestFilter` to run a subset:
```powershell
.\scripts\run-integration-tests.ps1 -TestFilter query
```

Orchestrator integration tests need configured browser profiles (not just a raw
CDP port). Run them separately:
```powershell
.\scripts\run-integration-tests.ps1 -IncludeOrchestrator
```

Non-ignored unit tests always pass without a browser:
```bash
cargo test --lib
```

### Adding a New Task

1. Create file in `src/task/my_task.rs`
2. Implement `run(api: &TaskContext, payload: Value) -> Result<()>`
3. Register in `src/task/mod.rs`
4. Add documentation in `docs/TASKS/my_task.md`

See [docs/TASKS/overview.md](TASKS/overview.md) for full details.

## Pull Request Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Refactoring

## Testing
- [ ] `cargo test` passes
- [ ] `cargo clippy --all-targets --all-features` is clean
- [ ] `cargo fmt` run
- [ ] New tests added for new functionality

## Checklist
- [ ] Code follows existing style
- [ ] Documentation updated (if needed)
- [ ] No breaking changes (or clearly documented)
```

## Commit Message Style

```
feat: add new twitterquote task
fix: handle rate limit in twitterfollow
docs: update api reference for nativeclick
refactor: extract common retry logic
test: add integration test for graceful shutdown
```

## Project Structure

```
crates/
├── bacon-pipeline/  # Shared pipeline types, traits & agent implementations

src/
├── adaptive/        # Adaptive learning module
├── api/             # API client
├── benchmarks/      # Performance benchmarks
├── bin/             # CLI binary entry points
├── capabilities/    # Task-facing actions (mouse, keyboard, scroll)
├── cli/             # Command line interface
├── config/          # Configuration loader
├── internal/        # Framework helpers
├── llm/             # LLM integration
├── metrics.rs       # Metrics collection and logging
├── orchestrator.rs  # Main runtime orchestrator
├── runtime/         # Browser/session/page lifecycle
├── session/         # Session management
├── state/           # Session-scoped handles
├── task/            # Automation tasks
├── tests/           # Built-in test helpers
├── utils/           # Low-level utilities
├── validation/      # Validation utilities
└── ...
```

## Getting Help

- Check [README.md](../README.md) for usage
- Review [docs/TASKS/overview.md](TASKS/overview.md) for task development
- See [API Reference](API_REFERENCE.md) for API details
- Open an issue for bugs or feature requests

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
