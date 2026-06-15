last audited 16-06-26 by opencode
# Task Overview

Tasks are the automation units in the Rust Orchestrator. Each task is a Rust async function that runs browser automation actions through a `TaskContext`.

## Running Tasks

```bash
# Single task
cargo run cookiebot

# Task with parameters
cargo run pageview=url=https://example.com

# Multiple tasks (parallel within group)
cargo run cookiebot pageview=reddit.com

# Sequential groups (then = new group)
cargo run cookiebot pageview=reddit.com then cookiebot
```

## Available Tasks

| Task | Description | Doc |
|------|-------------|-----|
| `cookiebot` | Cookie/consent dialog management | [cookiebot.md](cookiebot.md) |
| `demo-keyboard` | Keyboard interaction demo | text only |
| `demo-mouse` | Mouse movement demo | text only |
| `demoqa` | Demo text box automation | [demoqa.md](demoqa.md) |
| `pageview` | Human-like page browsing | [pageview.md](pageview.md) |
| `task-example` | Example task template | text only |
| `twitteractivity` | Full Twitter/X engagement with smart decisions | [twitteractivity.md](twitteractivity.md) |
| `twitterdive` | Thread diving and reading | text only |
| `twitterfollow` | Profile following | [twitterfollow.md](twitterfollow.md) |
| `twitterintent` | Intent-based actions (like, follow) | text only |
| `twitterlike` | Like specific tweets | text only |
| `twitterquote` | Quote tweets with LLM | text only |
| `twitterreply` | Tweet replies with LLM | [twitterreply.md](twitterreply.md) |
| `twitterretweet` | Retweet specific tweets | text only |
| `twittertest` | Twitter automation smoke tests | text only |

## Task Syntax

```
taskname                      # No parameters
taskname=value                # Shorthand URL/value
taskname=url=https://...      # Explicit parameter
taskname.js                   # .js extension auto-stripped
```

Parameters are passed as `serde_json::Value` to the task's `run()` function.

Entries marked `text only` do not have a dedicated task doc page yet.

## Creating New Tasks

See [docs/CONTRIBUTING.md](../CONTRIBUTING.md) for the task creation guide including file setup, registration, and documentation requirements.

## Shared Task Rules

- TaskContext rules: [task-context.md](task-context.md)
- DSL rules: [dsl.md](dsl.md)
- Selector rules: [selectors.md](selectors.md)
- Twitter task behavior: [twitteractivity.md](twitteractivity.md)
- Browser browsing behavior: [pageview.md](pageview.md)
