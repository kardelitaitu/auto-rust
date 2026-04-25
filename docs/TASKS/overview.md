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
| `pageview` | Human-like page browsing | [pageview.md](pageview.md) |
| `demoqa` | Demo text box automation | [demoqa.md](demoqa.md) |
| `twitteractivity` | Full Twitter/X engagement | [twitteractivity.md](twitteractivity.md) |
| `twitterfollow` | Profile following | [twitterfollow.md](twitterfollow.md) |
| `twitterreply` | Tweet replies with LLM | [twitterreply.md](twitterreply.md) |

## Task Syntax

```
taskname                      # No parameters
taskname=value                # Shorthand URL/value
taskname=url=https://...      # Explicit parameter
taskname.js                   # .js extension auto-stripped
```

Parameters are passed as `serde_json::Value` to the task's `run()` function.

## Creating New Tasks

See [Task Authoring Guide](../TASK_AUTHORING_GUIDE.md) for detailed instructions.
