# Auto-AI Documentation

Welcome to the Auto-AI documentation. Here you'll find everything you need to get started and master the framework.

## Quick Links

| Guide                                 | Description                           |
| ------------------------------------- | ------------------------------------- |
| [Getting Started](getting-started.md) | Install and run your first automation |
| [Developer Guide](development.md)     | Internal dev workflows and debugging  |
| [API Reference](api.md)               | Complete API documentation            |
| [Architecture](architecture.md)       | System design and core concepts       |
| [Configuration](configuration.md)     | Settings and environment variables    |
| [Tasks](tasks.md)                     | Built-in automation tasks             |
| [Contributing](../CONTRIBUTING.md)    | How to contribute to this project     |

## What is Auto-AI?

Auto-AI is a multi-browser automation framework that orchestrates browser automation across multiple anti-detect browser profiles using Playwright's CDP (Chrome DevTools Protocol). It uses AI for decision-making and includes human-like behavior patterns to reduce detection risk.

## Key Features

- **Multi-Browser Support** - Works with ixBrowser, MoreLogin, Dolphin, Brave, Chrome, Edge, Vivaldi
- **AI-Powered Decision Making** - Local Ollama/Docker LLMs and cloud OpenRouter integration
- **Human-Like Behavior** - Mouse movements, keystroke dynamics, scrolling patterns
- **Session Isolation** - AsyncLocalStorage-based context isolation
- **Error Recovery** - Automatic retry strategies and self-healing prompts

## Common Tasks

```bash
# Page navigation
node main.js pageview=example.com

# Twitter follow
node main.js twitterFollow=https://twitter.com/user

# Run strategy game agent
node agent-main.js owb play
```

## Project Structure

```
auto-ai/
├── api/              # Core API (context, orchestrator, session manager)
├── api/agent/        # AI agent stack (perception, reasoning, action)
├── api/interactions/ # User actions (clicks, typing, scrolling)
├── api/behaviors/    # Humanization behaviors
├── api/utils/        # Utilities (config, logging, fingerprint)
├── connectors/       # Browser discovery adapters
├── tasks/            # Runnable automation scripts
├── config/           # Configuration files
└── docs/             # Documentation
```

## Need Help?

- Check [Troubleshooting](troubleshooting.md) for common issues
- Review [API Reference](api.md) for detailed method documentation
- See [.agents/](../.agents/) for agent-specific documentation
