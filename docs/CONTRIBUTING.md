# Contributing Guide

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
cd rust-orchestrator

# Build
cargo build --all-features

# Run tests
cargo test

# Check lints
cargo clippy --all-targets --all-features
```

## Making Changes

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

### Adding a New Task

1. Create file in `src/task/my_task.rs`
2. Implement `run(api: &TaskContext, payload: Value) -> Result<()>`
3. Register in `src/task/mod.rs`
4. Add documentation in `docs/TASKS/my_task.md`

See [Task Authoring Guide](TASK_AUTHORING_GUIDE.md) for full details.

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
src/
├── api/          # API client
├── browser.rs    # Browser management
├── cli.rs        # Command line interface
├── config.rs     # Configuration loader
├── capabilities/ # Task-facing actions (mouse, keyboard, scroll)
├── internal/     # Framework helpers
├── task/         # Automation tasks
├── utils/        # Low-level utilities
└── ...
```

## Getting Help

- Check [README.md](../README.md) for usage
- Review [Task Authoring Guide](TASK_AUTHORING_GUIDE.md) for task development
- See [API Reference](API_REFERENCE.md) for API details
- Open an issue for bugs or feature requests

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
