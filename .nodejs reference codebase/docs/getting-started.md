# Getting Started

This guide will help you set up and run your first automation with Auto-AI.

## Prerequisites

| Requirement | Version | Notes                                   |
| ----------- | ------- | --------------------------------------- |
| Node.js     | 18+     | LTS recommended                         |
| pnpm        | 8+      | Or npm/yarn                             |
| Browser     | -       | ixBrowser, Brave, Chrome, Edge, Vivaldi |
| Docker      | 24+     | For local LLM (optional)                |

## Installation

### 1. Clone and Install

```bash
git clone https://github.com/kardelitaitu/auto-ai.git
cd auto-ai
pnpm install
```

### 2. Configure Environment

Copy the example environment file:

```bash
copy .env-example .env
```

Edit `.env` with your API keys:

```env
# OpenRouter (recommended for cloud AI)
OPENROUTER_API_KEY=your_key_here

# Local LLM (optional, faster for simple tasks)
LOCAL_LLM_ENDPOINT=http://localhost:11434/api/generate
LOCAL_LLM_MODEL=llama3
```

### 3. Start Your Browser

Start a browser with remote debugging enabled:

**ixBrowser**: Enable "Remote Debugging" in settings (port 8855)

**Brave/Chrome/Edge**:

```bash
# Windows
start "" "C:\Program Files\BraveSoftware\Brave-Browser\Application\browser.exe" --remote-debugging-port=9222

# Or use the helper script
ix-open.bat
```

## Running Your First Task

### Basic Navigation

```bash
node main.js pageview=example.com
```

### Twitter Automation

```bash
# Follow a user
node main.js twitterFollow=https://twitter.com/username

# Like a tweet
node main.js like tweetUrl="https://twitter.com/user/status/123"
```

### Game Agent (OWB)

```bash
# Auto-play mode
node agent-main.js owb play

# Rush strategy
node agent-main.js owb rush
```

## Quick Reference

| Command                            | Description         |
| ---------------------------------- | ------------------- |
| `node main.js pageview=<url>`      | Navigate to URL     |
| `node main.js twitterFollow=<url>` | Follow Twitter user |
| `node main.js like tweetUrl=<url>` | Like a tweet        |
| `node agent-main.js owb play`      | Run game agent      |
| `pnpm test:unit`                   | Run unit tests      |

## Troubleshooting

### Browser Not Found

Make sure your browser is running with remote debugging enabled. Check the logs:

```bash
node main.js pageview=example.com 2>&1 | findstr /i browser
```

### LLM Connection Failed

- For local LLM: Ensure Docker is running with Ollama
- For cloud LLM: Verify `OPENROUTER_API_KEY` in `.env`

### Port Already in Use

Check which process is using the port:

```bash
# Windows
netstat -ano | findstr :9222
```

## Next Steps

- [API Reference](api.md) - Learn the full API
- [Configuration](configuration.md) - Customize settings
- [Architecture](architecture.md) - Understand the system
