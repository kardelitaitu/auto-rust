# Auto-AI

![Coverage](./coverage-badge.svg)
![Node.js](https://img.shields.io/badge/node-%3E%3D18-brightgreen?logo=node.js&logoColor=white)
![pnpm](https://img.shields.io/badge/pnpm-%3E%3D10.33-yellow?logo=pnpm&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-blue?logo=openaccess&logoColor=white)
![Version](https://img.shields.io/badge/version-0.0.30-orange)
![Dependencies](https://img.shields.io/badge/dependencies-up%20to%20date-brightgreen)

Agentic orchestration framework for discovering and automating pre-existing browser instances via CDP. Uses AI for decision-making with human-like behavior patterns to reduce detection risk.

## Table of Contents

- [Features](#features)
- [Demo](#demo)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Usage](#usage)
    - [Command Line](#command-line)
    - [Programmatic API](#programmatic-api)
- [Documentation](#documentation)
- [Requirements](#requirements)
- [Testing](#testing)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)
- [Changelog](#changelog)
- [Acknowledgments](#acknowledgments)
- [License](#license)

## Features

- **Multi-Browser Support** - ixBrowser, MoreLogin, Dolphin, Brave, Chrome, Edge, Vivaldi
- **AI-Powered** - Local Ollama/Docker LLMs + cloud OpenRouter integration
- **Human-Like Behavior** - Mouse movements, keystroke dynamics, scrolling patterns
- **Session Isolation** - AsyncLocalStorage-based context isolation
- **Error Recovery** - Automatic retry strategies and self-healing prompts
- **Game Agent** - OWB strategy automation with multiple play styles

## Demo

> Record a terminal session or browser automation and save as `demo.gif` in the root directory.

```bash
# Example automation output
$ node main.js pageview=example.com then twitterFollow=url

[AutoAI] Starting task sequence...
[AutoAI] Navigating to example.com...
[AutoAI] Simulating human reading pattern...
[AutoAI] Task complete!
```

## Quick Start

### 1. Install

```bash
git clone https://github.com/kardelitaitu/auto-ai.git
cd auto-ai
pnpm install
```

### 2. Configure

```bash
copy .env-example .env
# Edit .env with your API keys (see Configuration section)
```

### 3. Run

```bash
# Start browser with remote debugging
node main.js pageview=example.com
```

## Configuration

### Environment Variables (.env)

```env
# AI Provider (optional - for intelligent decisions)
OPENROUTER_API_KEY=sk-or-v1-xxxxx
OLLAMA_HOST=http://localhost:11434

# Browser Settings
HUMAN_DEBUG=false
NODE_ENV=production

# Logging
LOG_LEVEL=info
```

### Browser Configuration (config/browserAPI.json)

```json
{
    "ixbrowser": { "port": 18800 },
    "morelogin": { "port": 18800 },
    "dolphin": { "port": 8888 },
    "brave": { "port": 9222 },
    "chrome": { "port": 9222 }
}
```

### Automation Settings (config/settings.json)

```json
{
    "twitter": {
        "engagement": {
            "maxLikes": 5,
            "maxFollows": 1,
            "maxRetweets": 1
        }
    }
}
```

## Usage

### Command Line

```bash
# Page navigation
node main.js pageview=example.com

# Twitter automation
node main.js twitterFollow=https://twitter.com/user
node main.js like tweetUrl="https://twitter.com/user/status/123"
node main.js retweet tweetUrl="https://twitter.com/user/status/123"

# Chained tasks
node main.js pageview=example.com then twitterFollow=url then like tweetUrl=url

# Game agent
node agent-main.js owb play
node agent-main.js owb rush
node agent-main.js owb turtle
```

### Programmatic API

```javascript
import { api } from './api/index.js';

// Basic navigation and interaction
await api.withPage(async (page) => {
    await api.navigate('https://example.com');
    await api.wait(1000);
    await api.click('#button');
    await api.type('input[name="search"]', 'query');
    await api.scroll.down(500);
});

// Twitter automation
import { twitterFollow, likeTweet } from './tasks/';

// Using the agent
import { AgentRunner } from './api/agent/';
const agent = new AgentRunner({ strategy: 'balanced' });
await agent.run();
```

## Documentation

| Guide                                      | Description                       |
| ------------------------------------------ | --------------------------------- |
| [Getting Started](docs/getting-started.md) | Installation and first automation |
| [API Reference](docs/api.md)               | Complete API documentation        |
| [Architecture](docs/architecture.md)       | System design and concepts        |
| [Configuration](docs/configuration.md)     | Settings and environment          |
| [Tasks](docs/tasks.md)                     | Built-in automation tasks         |
| [Troubleshooting](docs/troubleshooting.md) | Common issues and solutions       |

## Requirements

- **Node.js** >= 18
- **pnpm** >= 10.33
- **Browser** with remote debugging (ixBrowser, Brave, Chrome, etc.)
- **Docker** (optional, for local LLM via Ollama)

## Testing

```bash
# Fast tests (Bun)
pnpm run test:bun:unit       # Unit tests
pnpm run test:bun:integration # Integration tests
pnpm run test:bun:all        # All tests

# With coverage
pnpm run coverage:full       # Run tests + generate badge
```

## Troubleshooting

### Browser Connection Issues

```bash
# Ensure browser has remote debugging enabled
# Brave: brave.exe --remote-debugging-port=9222
# Chrome: chrome.exe --remote-debugging-port=9222
```

### Port Conflicts

```bash
# Check if ports are in use
netstat -an | findstr "18800 9222"

# Kill process on port
npx kill-port 9222
```

### LLM Connection

```bash
# Verify Ollama is running
curl http://localhost:11434/api/tags

# Test OpenRouter API key
curl -H "Authorization: Bearer $OPENROUTER_API_KEY" \
     https://openrouter.ai/api/v1/models
```

## Project Structure

```
auto-ai/
├── api/              # Core API
│   ├── core/         # Context, orchestrator, session manager
│   ├── agent/        # AI agent stack
│   ├── interactions/ # Click, type, scroll
│   ├── behaviors/    # Humanization
│   └── utils/        # Config, logging, fingerprint
├── connectors/       # Browser discovery adapters
├── tasks/            # Automation scripts
├── config/           # Configuration files
└── docs/             # Documentation
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`pnpm commit "Add amazing feature"`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Changelog

See [patchnotes.md](patchnotes.md) for a list of changes in each release.

## Acknowledgments

- [Playwright](https://playwright.dev/) - Browser automation
- [Vitest](https://vitest.dev/) - Testing framework
- [Ollama](https://ollama.ai/) - Local LLM support
- [OpenRouter](https://openrouter.ai/) - Cloud AI API

## License

MIT - see [LICENSE](LICENSE) for details.
