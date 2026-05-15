# Documentation Summary

last audited 08-05-26 by Kilo

This directory contains the documentation for the Rust Orchestrator project.

## Documents

- [README.md](../README.md) - Main project documentation with features, installation, and usage
- [docs/specs/README.md](./specs/README.md) - Two-agent spec workflow and handoff contract
- [TUTORIAL_BUILDING_FIRST_TASK.md](./TUTORIAL_BUILDING_FIRST_TASK.md) - Guide for authoring browser automation tasks
- [TASK_RUNNER_PREPARATION.md](./_archive/TASK_RUNNER_PREPARATION.md) - Registry foundation and task-system preparation plan (archived)
- [TASK_RUNNER_DSL_BUILD.md](./_archive/TASK_RUNNER_DSL_BUILD.md) - Future DSL build plan that depends on the registry foundation (archived)

## API Documentation

Generate HTML documentation using `cargo doc`:

```bash
# Generate documentation
cargo doc --all-features

# Open in browser
cargo doc --open
```

## Quick Links

- [Installation Guide](../README.md#installation)
- [Quick Start](../README.md#quick-start)
- [Available Tasks](../README.md#available-tasks)
- [Configuration](../README.md#configuration)
- [Task Authoring](./TUTORIAL_BUILDING_FIRST_TASK.md)
