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
| `demo-keyboard` | Keyboard interaction demo | - |
| `demo-mouse` | Mouse movement demo | - |
| `demoqa` | Demo text box automation | [demoqa.md](demoqa.md) |
| `pageview` | Human-like page browsing | [pageview.md](pageview.md) |
| `task-example` | Example task template | - |
| `twitteractivity` | Full Twitter/X engagement with smart decisions | [twitteractivity.md](twitteractivity.md) |
| `twitterdive` | Thread diving and reading | - |
| `twitterfollow` | Profile following | [twitterfollow.md](twitterfollow.md) |
| `twitterintent` | Intent-based actions (like, follow) | - |
| `twitterlike` | Like specific tweets | - |
| `twitterquote` | Quote tweets with LLM | - |
| `twitterreply` | Tweet replies with LLM | [twitterreply.md](twitterreply.md) |
| `twitterretweet` | Retweet specific tweets | - |
| `twittertest` | Twitter automation smoke tests | - |

## Task Syntax

```
taskname                      # No parameters
taskname=value                # Shorthand URL/value
taskname=url=https://...      # Explicit parameter
taskname.js                   # .js extension auto-stripped
```

Parameters are passed as `serde_json::Value` to the task's `run()` function.

## Creating New Tasks

See [Tutorial: Building First Task](../TUTORIAL_BUILDING_FIRST_TASK.md) for detailed instructions.
