# Frequently Asked Questions (FAQ)

Common questions and answers about Auto-AI.

---

## Getting Started

### How do I install Auto-AI?

```bash
git clone https://github.com/kardelitaitu/auto-ai.git
cd auto-ai
pnpm install
copy .env.example .env
# Edit .env with your settings
```

See [QUICKSTART.md](QUICKSTART.md) for the full guide.

---

### What browsers are supported?

**Anti-detect browsers:**
- ixBrowser, MoreLogin, Dolphin, AdsPower, RoxyBrowser
- Undetectable, MultiLogin, GoLogin, Incogniton
- Kameleo, OctoBrowser, NSTBrowser, HideMyAcc, AntBrowser

**Standard browsers:**
- Chrome, Brave, Edge, Vivaldi (with remote debugging)

---

### Do I need an API key?

**No**, if you use local LLM (Ollama):
```env
LOCAL_LLM_ENDPOINT=http://localhost:11434
```

**Yes**, for cloud LLM (OpenRouter):
```env
OPENROUTER_API_KEY=sk-or-v1-your_key_here
```

---

## Configuration

### How do I configure Ollama?

1. Install Ollama from https://ollama.ai
2. Pull a model: `ollama pull hermes3:8b`
3. Add to `.env`:
```env
LOCAL_LLM_ENDPOINT=http://localhost:11434
LOCAL_LLM_MODEL=hermes3:8b
```

---

### How do I change the browser port?

Edit `config/browserAPI.json`:
```json
{
    "ixbrowser": { "port": 53200 },
    "brave": { "port": 9222 }
}
```

---

### How do I adjust humanization settings?

Edit `config/settings.json`:
```json
{
    "humanization": {
        "mouse": {
            "minDuration": 500,
            "maxDuration": 2000
        },
        "keystroke": {
            "baseDelay": 150
        }
    }
}
```

---

## Usage

### How do I run a task?

```bash
# Basic page view
node main.js pageview=https://example.com

# Twitter automation
node main.js twitterFollow=https://twitter.com/username

# Game agent
node agent-main.js owb play
```

---

### How do I chain multiple tasks?

```bash
node main.js pageview=url then like=tweetUrl then retweet=tweetUrl
```

---

### How do I create a custom task?

1. Create `tasks/my-task.js`:
```javascript
export default async function(page, payload) {
    await api.withPage(page, async () => {
        await api.init(page);
        // Your automation logic
    });
}
```

2. Run it:
```bash
node main.js my-task=parameter
```

---

## Troubleshooting

### "No browsers discovered"

**Cause:** Browser not running with remote debugging.

**Fix:**
- ixBrowser: Enable Remote Debugging in Settings
- Chrome/Brave: Launch with `--remote-debugging-port=9222`

---

### "Connection timeout"

**Cause:** Firewall or wrong port.

**Fix:**
1. Check firewall allows the port
2. Verify port in `config/browserAPI.json`
3. Try a different browser

---

### "LLM not responding"

**Local LLM:**
```bash
# Check Ollama is running
curl http://localhost:11434/api/tags

# Start Ollama
ollama serve
```

**Cloud LLM:**
```bash
# Verify API key
curl -H "Authorization: Bearer $OPENROUTER_API_KEY" \
     https://openrouter.ai/api/v1/models
```

---

### "Session crashed"

**Cause:** Browser closed or disconnected.

**Fix:**
1. Keep browser window open
2. Increase timeout in `config/timeouts.json`
3. Check logs for details

---

### Tests failing randomly

**Cause:** AsyncLocalStorage isolation issue.

**Fix:** Ensure `pool: 'forks'` in `config/vitest.config.js`:
```javascript
export default {
    test: {
        pool: 'forks'
    }
}
```

---

## Performance

### How can I make automation faster?

1. Use local LLM for simple tasks
2. Reduce humanization delays:
```json
{
    "humanization": {
        "mouse": { "minDuration": 100 }
    }
}
```
3. Increase concurrent sessions

---

### How can I make automation more stealthy?

1. Increase humanization delays:
```json
{
    "humanization": {
        "mouse": { "minDuration": 500, "maxDuration": 2000 },
        "keystroke": { "baseDelay": 200 }
    }
}
```
2. Use stealth persona
3. Enable idle behavior

---

## Development

### How do I run tests?

```bash
# Fast unit tests
pnpm run test:bun:unit

# All tests
pnpm run test:bun:all

# With coverage
pnpm run test:bun:coverage
```

---

### How do I contribute?

1. Fork the repository
2. Create feature branch: `git checkout -b feat/my-feature`
3. Make changes
4. Run tests: `pnpm run test:bun:all`
5. Commit: `pnpm commit "Add my feature"`
6. Push and create PR

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

---

### Where are tests located?

```
api/tests/
├── unit/           # Unit tests
├── integration/    # Integration tests
└── edge-cases/     # Edge case tests
```

---

## Advanced

### How do I use the API programmatically?

```javascript
import { api } from './api/index.js';

await api.withPage(page, async () => {
    await api.init(page, { persona: 'casual' });
    await api.goto('https://example.com');
    await api.click('.button');
});
```

See [API-CHEATSHEET.md](API-CHEATSHEET.md) for more patterns.

---

### How do I add a new browser connector?

1. Extend `connectors/baseDiscover.js`
2. Implement `discover()` method
3. Add to `config/browserAPI.json`

See existing connectors for examples.

---

### How do I customize the LLM prompt?

Edit prompts in `prompts/` directory or modify `api/agent/llmClient.js`.

---

## Getting Help

### Where can I find more documentation?

- [QUICKSTART.md](QUICKSTART.md) - Quick start guide
- [docs/](docs/) - Full documentation
- [AGENTS.md](AGENTS.md) - Agent development
- [API-CHEATSHEET.md](API-CHEATSHEET.md) - API reference

---

### What if my question isn't answered here?

1. Check [docs/troubleshooting.md](docs/troubleshooting.md)
2. Search [GitHub Issues](https://github.com/kardelitaitu/auto-ai/issues)
3. Open a new issue with details

---

### How do I report a bug?

1. Search existing issues first
2. Open new issue with:
   - Description
   - Steps to reproduce
   - Expected behavior
   - Actual behavior
   - Environment (OS, Node.js version, browser)
   - Logs (if applicable)
