# Quick Start Guide

Get Auto-AI running in 5 minutes.

---

## Prerequisites

Before you begin, ensure you have:

- [ ] **Node.js 18+** - [Download](https://nodejs.org/)
- [ ] **pnpm 10+** - Run: `npm install -g pnpm`
- [ ] **A browser** with remote debugging enabled (ixBrowser, Chrome, Brave, etc.)

---

## Step 1: Install (2 minutes)

```bash
# Clone and install
git clone https://github.com/kardelitaitu/auto-ai.git
cd auto-ai
pnpm install
```

---

## Step 2: Configure (1 minute)

### A. Environment Variables

```bash
# Copy the example environment file
copy .env.example .env
```

Edit `.env` and choose ONE option:

**Option A: Local LLM (Free)**
```env
LOCAL_LLM_ENDPOINT=http://localhost:11434
LOCAL_LLM_MODEL=hermes3:8b
```
> Requires Ollama: Run `ollama serve` in another terminal

**Option B: Cloud LLM (Better reasoning)**
```env
OPENROUTER_API_KEY=sk-or-v1-your_key_here
OPENROUTER_DEFAULT_MODEL=anthropic/claude-3.5-sonnet
```
> Get API key: https://openrouter.ai/keys

### B. Browser Configuration

Ensure your browser is running with remote debugging:

**ixBrowser:**
- Open Settings → Enable Remote Debugging → Port 53200

**Chrome/Brave:**
```bash
# Launch with remote debugging flag
chrome.exe --remote-debugging-port=9222
```

---

## Step 3: Run Your First Automation (1 minute)

### Basic Page View

```bash
# Navigate to a website
node main.js pageview=https://example.com
```

### Chained Actions

```bash
# Multiple actions in sequence
node main.js pageview=https://example.com then twitterFollow=https://twitter.com/username
```

### Game Agent (OWB)

```bash
# Start autonomous game agent
node agent-main.js owb play
```

---

## What Just Happened?

1. **Browser Discovery**: Auto-AI found your running browser via CDP
2. **Session Created**: Isolated session with unique context
3. **Task Executed**: Ran the automation with human-like behavior
4. **Cleanup**: Session closed cleanly

---

## Next Steps

### Learn More

| Topic | Guide |
|-------|-------|
| Full configuration | [docs/CONFIGURATION-GUIDE.md](docs/CONFIGURATION-GUIDE.md) |
| Available tasks | [docs/tasks.md](docs/tasks.md) |
| API reference | [docs/api.md](docs/api.md) |
| Agent modes | [AGENTS.md](AGENTS.md) |

### Common Commands

```bash
# Run tests
pnpm run test:bun:unit

# Lint code
pnpm run lint

# Format code
pnpm run format

# Start dashboard
pnpm run dashboard
```

### Troubleshooting

**Issue**: `No browsers discovered`
- **Fix**: Ensure browser is running with remote debugging enabled

**Issue**: `Connection timeout`
- **Fix**: Check firewall settings and port configuration

**Issue**: `LLM not responding`
- **Fix**: Verify Ollama is running (`ollama serve`) or API key is valid

See [docs/troubleshooting.md](docs/troubleshooting.md) for more solutions.

---

## Quick Reference

### Environment Variables

```env
# Local LLM
LOCAL_LLM_ENDPOINT=http://localhost:11434
LOCAL_LLM_MODEL=hermes3:8b

# Cloud LLM
OPENROUTER_API_KEY=sk-or-v1-xxx

# System
NODE_ENV=development
LOG_LEVEL=info
```

### Task Examples

```bash
# Page navigation
node main.js pageview=https://example.com

# Twitter automation
node main.js twitterFollow=https://twitter.com/username
node main.js like tweetUrl="https://twitter.com/user/status/123"

# Game agent
node agent-main.js owb play
node agent-main.js owb rush
```

### Configuration Files

| File | Purpose |
|------|---------|
| `.env` | Environment variables |
| `config/settings.json` | Automation settings |
| `config/browserAPI.json` | Browser ports |
| `config/timeouts.json` | Timeout values |

---

## Getting Help

- **Documentation**: [docs/](docs/) directory
- **Issues**: [GitHub Issues](https://github.com/kardelitaitu/auto-ai/issues)
- **Agent Guide**: [AGENTS.md](AGENTS.md)
- **Contributing**: [CONTRIBUTING.md](CONTRIBUTING.md)

---

**You're all set! 🚀**

Start automating with `node main.js <task>=<url>`
