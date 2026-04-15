# Auto-AI User Guide

Complete guide for using Auto-AI browser automation framework.

---

## Table of Contents

1. [Getting Started](#getting-started)
2. [Installation](#installation)
3. [Configuration](#configuration)
4. [Running Tasks](#running-tasks)
5. [API Overview](#api-overview)
6. [Common Patterns](#common-patterns)
7. [Troubleshooting](#troubleshooting)
8. [Advanced Topics](#advanced-topics)

---

## Getting Started

Auto-AI is a multi-browser automation framework that uses AI for decision-making with human-like behavior patterns.

**What you can do:**
- Automate Twitter/X engagement (follow, like, retweet, reply)
- Run autonomous game agents (OWB strategy game)
- Create custom browser automation tasks
- Orchestrate multiple browser sessions simultaneously

**Quick example:**
```bash
# Navigate to a website
node main.js pageview=https://example.com

# Follow someone on Twitter
node main.js twitterFollow=https://twitter.com/username

# Run game agent
node agent-main.js owb play
```

---

## Installation

### Prerequisites

| Requirement | Version | Purpose |
|-------------|---------|---------|
| Node.js | 18+ | Runtime |
| pnpm | 10+ | Package manager |
| Browser | Any CDP-enabled | Automation target |

### Step 1: Clone and Install

```bash
git clone https://github.com/kardelitaitu/auto-ai.git
cd auto-ai
pnpm install
```

### Step 2: Configure Environment

```bash
copy .env.example .env
```

Edit `.env`:

```env
# Option A: Local LLM (free, requires Ollama)
LOCAL_LLM_ENDPOINT=http://localhost:11434
LOCAL_LLM_MODEL=hermes3:8b

# Option B: Cloud LLM (paid, better reasoning)
OPENROUTER_API_KEY=sk-or-v1-your_key_here
```

### Step 3: Start Browser

**ixBrowser:**
- Open Settings → Enable Remote Debugging → Port 53200

**Chrome/Brave:**
```bash
chrome.exe --remote-debugging-port=9222
```

---

## Configuration

### Configuration Files

| File | Purpose |
|------|---------|
| `.env` | Environment variables |
| `config/settings.json` | Automation settings |
| `config/browserAPI.json` | Browser ports |
| `config/timeouts.json` | Timeout values |

### Key Settings

**LLM Configuration:**
```json
{
  "llm": {
    "local": {
      "enabled": true,
      "model": "hermes3:8b"
    },
    "cloud": {
      "enabled": false,
      "provider": "openrouter"
    }
  }
}
```

**Humanization:**
```json
{
  "humanization": {
    "mouse": { "enabled": true },
    "keystroke": { "enabled": true },
    "idle": { "enabled": true }
  }
}
```

See [docs/CONFIGURATION-GUIDE.md](CONFIGURATION-GUIDE.md) for complete details.

---

## Running Tasks

### Built-in Tasks

| Task | Command | Description |
|------|---------|-------------|
| Page View | `node main.js pageview=<url>` | Navigate to URL |
| Twitter Follow | `node main.js twitterFollow=<url>` | Follow user |
| Like Tweet | `node main.js like tweetUrl=<url>` | Like a tweet |
| Retweet | `node main.js retweet tweetUrl=<url>` | Retweet |
| Reply | `node main.js reply tweetUrl=<url>` | Reply to tweet |
| Game Agent | `node agent-main.js owb play` | Auto-play game |

### Task Chaining

```bash
# Multiple actions in sequence
node main.js pageview=url then twitterFollow=url then like=tweetUrl
```

### Game Agent Modes

```bash
node agent-main.js owb play      # Auto-play
node agent-main.js owb rush      # Fast attack
node agent-main.js owb turtle    # Defensive
node agent-main.js owb economy   # Resource focus
```

---

## API Overview

### Basic Pattern

```javascript
import { api } from './api/index.js';

await api.withPage(page, async () => {
    await api.init(page, { persona: 'casual' });
    await api.goto('https://example.com');
    await api.click('.button');
});
```

### Key Methods

| Category | Methods |
|----------|---------|
| Navigation | `api.goto()`, `api.goBack()`, `api.reload()` |
| Interaction | `api.click()`, `api.type()`, `api.press()` |
| Scroll | `api.scroll.down()`, `api.scroll.toTop()` |
| Wait | `api.wait()`, `api.wait.forElement()` |
| Query | `api.find()`, `api.findAll()`, `api.exists()` |
| Screenshot | `api.screenshot.full()`, `api.screenshot.save()` |

See [API-CHEATSHEET.md](../API-CHEATSHEET.md) for quick reference.
See [docs/api.md](api.md) for complete reference.

---

## Common Patterns

### Wait for Element Then Click

```javascript
await api.goto('https://example.com');
await api.wait.forElement('.loaded');
await api.click('.action-btn');
```

### Fill Form

```javascript
await api.type('input[name="email"]', 'user@example.com');
await api.type('input[name="password"]', 'secret');
await api.press('Enter');
```

### Scroll and Read

```javascript
await api.scroll.read();  // Natural reading pattern
await api.wait(3000);     // Simulate reading time
```

### Take Screenshot

```javascript
await api.screenshot.full();
await api.screenshot.save('output.png');
```

See [docs/RECIPES.md](RECIPES.md) for more patterns.

---

## Troubleshooting

### Browser Not Found

**Problem:** `Error: No browsers discovered`

**Solution:**
1. Ensure browser is running
2. Enable remote debugging: `--remote-debugging-port=9222`
3. Check port in `config/browserAPI.json`

### LLM Not Responding

**Problem:** `Error: LLM request timeout`

**Solution:**
- Local: `ollama serve` must be running
- Cloud: Verify `OPENROUTER_API_KEY` in `.env`

### Element Not Found

**Problem:** `Error: Element not found: .selector`

**Solution:**
```javascript
// Wait for element first
await api.wait.forElement('.selector');
await api.click('.selector');

// Or check if exists
if (await api.exists('.selector')) {
    await api.click('.selector');
}
```

See [docs/troubleshooting.md](troubleshooting.md) for complete solutions.
See [FAQ.md](../FAQ.md) for common questions.

---

## Advanced Topics

### Custom Tasks

Create `tasks/my-task.js`:

```javascript
export default async function(page, payload) {
    await api.withPage(page, async () => {
        await api.init(page);
        await api.goto(payload.url);
        // Your automation logic
    });
}
```

Run: `node main.js my-task=url`

### Personas

```javascript
await api.init(page, { persona: 'stealth' });
```

Available: `casual`, `focused`, `careful`, `stealth`

### Session Management

```javascript
// Multiple isolated sessions
await api.withPage(page1, async () => { /* ... */ });
await api.withPage(page2, async () => { /* ... */ });
```

### Agent Development

See [`.agents/`](../.agents/) for agent development guides.

---

## Related Documentation

| Document | Purpose |
|----------|---------|
| [QUICKSTART.md](../QUICKSTART.md) | 5-minute setup |
| [API-CHEATSHEET.md](../API-CHEATSHEET.md) | Quick reference |
| [docs/api.md](api.md) | Full API reference |
| [docs/RECIPES.md](RECIPES.md) | Automation patterns |
| [docs/troubleshooting.md](troubleshooting.md) | Problem solutions |
| [FAQ.md](../FAQ.md) | Common questions |

---

*Last updated: 2026-03-31*
