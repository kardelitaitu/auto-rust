# Proposed Folder Structure

This document explains the recommended folder structure for this Rust browser automation codebase.

## Goals

The structure should:

- keep framework code separate from task code
- make domain-specific code easy to find
- keep `utils/` small and truly generic
- isolate experimental or advanced subsystems
- allow tasks to grow into self-contained modules
- keep the public API small and intentional

## Recommended Structure

```text
src/
  main.rs
  lib.rs

  app/
    cli.rs
    config.rs
    orchestrator.rs
    metrics.rs
    logger.rs
    tracing.rs

  runtime/
    mod.rs
    execution.rs
    task_context.rs

  browser/
    mod.rs
    discovery.rs
    connection.rs
    profiles.rs

  session/
    mod.rs
    cleanup.rs
    health.rs

  capabilities/
    mod.rs
    mouse.rs
    keyboard.rs
    navigation.rs
    scroll.rs
    clipboard.rs
    timing.rs

  tasks/
    mod.rs

    cookiebot/
      mod.rs
      task.rs
      selectors.rs

    pageview/
      mod.rs
      task.rs

    demoqa/
      mod.rs
      task.rs

    twitter/
      mod.rs
      activity.rs
      follow.rs
      like.rs
      reply.rs
      retweet.rs
      quote.rs
      dive.rs
      intent.rs
      selectors.rs
      decision.rs
      sentiment.rs
      llm.rs

  validation/
    mod.rs
    task.rs
    registry.rs

  state/
    mod.rs
    overlay.rs
    clipboard.rs

  internal/
    mod.rs
    circuit_breaker.rs

  features/
    adaptive/
      mod.rs
      action_sequencer.rs
      learning_engine.rs
      predictive_scorer.rs
      self_healing.rs
      monitoring_dashboard.rs
      performance_analytics.rs
      thread_analyzer.rs

  llm/
    mod.rs
    client.rs
    models.rs
    reply_engine.rs
    reply_strategies.rs
    unified_processor.rs
    unified_action_processor.rs
    sentiment_aware_processor.rs

  utils/
    mod.rs
    text.rs
    timing.rs
    geometry.rs
    mouse.rs
    keyboard.rs
    scroll.rs
    navigation.rs
    zoom.rs
    page_size.rs
    native_input.rs
```

## Why This Works

### 1. `app/` for top-level wiring

This folder holds the application shell:

- CLI parsing
- config loading
- logging and tracing setup
- orchestration and metrics

These are important, but they are not domain features themselves.

### 2. `runtime/` for execution mechanics

This folder owns the execution layer:

- `TaskContext`
- task execution flow
- shutdown-aware runtime behavior

This keeps runtime logic separate from CLI and task definitions.

### 3. `tasks/` for self-contained automation tasks

Each task should live in its own folder once it grows beyond a trivial file.

That makes it easier to keep together:

- task logic
- selectors
- task-specific helpers
- task-specific tests
- task documentation

### 4. `browser/` and `session/` for lifecycle concerns

Browser discovery and session management are core infrastructure pieces.

Separating them makes it clearer where to find:

- browser connection logic
- discovery and profile handling
- health and cleanup behavior

### 5. `features/` for larger subsystems

Bigger cross-cutting systems, like adaptive learning or self-healing, should not be mixed with core task execution.

A `features/` namespace makes experimental or advanced capabilities easy to identify.

### 6. `utils/` only for generic helpers

The `utils/` folder should stay narrow.

Good candidates:

- text handling
- math/geometry helpers
- timing utilities
- low-level input helpers

If something is specific to Twitter, browser session management, or a task, it should usually live closer to that domain instead of `utils/`.

## Practical Rules

- Prefer **domain folders** over a large generic helper folder
- Put **task code** under `tasks/`
- Put **browser/session concerns** in their own modules
- Keep **experimental code** isolated from core runtime code
- Expose only the **minimum public API** from `lib.rs`
- Use **vertical slices** for large features like Twitter

## Suggested Migration Priority

If this structure is adopted, I would migrate in this order:

1. `runtime/`, `browser/`, `session/`
2. `tasks/`
3. `validation/` and `state/`
4. `llm/` and `features/adaptive/`
5. cleanup of `utils/`

That keeps the most important boundaries clear first.

## Summary

The best folder structure for this project is one that is:

- easy to navigate
- domain-oriented
- scalable as tasks grow
- conservative about shared utilities
- clear about what is core vs experimental

For this codebase, a structure centered around `app/`, `runtime/`, `tasks/`, `browser/`, `session/`, and `features/` is a strong fit.
