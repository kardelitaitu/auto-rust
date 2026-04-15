# Troubleshooting

Solutions for common issues with Auto-AI.

## Browser Issues

### Browser Not Found

**Symptoms:**

```
Error: No browsers discovered
```

**Solutions:**

1. Ensure browser is running with remote debugging enabled
2. Check the browser port matches config
3. Restart the browser

```bash
# ixBrowser: Enable Remote Debugging in Settings → Port 8855
# Chrome/Brave: Start with --remote-debugging-port=9222
```

### Connection Timeout

**Symptoms:**

```
Error: Connection timeout after 30000ms
```

**Solutions:**

1. Check firewall settings
2. Verify the port is correct
3. Try a different browser

## LLM Issues

### Local LLM Not Connecting

**Symptoms:**

```
Error: connect ECONNREFUSED 127.0.0.1:11434
```

**Solutions:**

1. Start Ollama: `ollama serve`
2. Pull a model: `ollama pull llama3`
3. Check Docker is running

```bash
# Start Ollama
ollama serve

# In another terminal, pull model
ollama pull llama3
```

### OpenRouter API Error

**Symptoms:**

```
Error: 401 - Invalid API key
```

**Solutions:**

1. Verify `OPENROUTER_API_KEY` in `.env`
2. Check API key has credits
3. Try a different model

### LLM Response Timeout

**Symptoms:**

```
Error: LLM request timeout
```

**Solutions:**

1. Increase timeout in `config/timeouts.json`
2. Use faster local LLM for simple tasks
3. Check network connection

## Session Issues

### Session Crashed

**Symptoms:**

```
Error: Session crashed
```

**Solutions:**

1. Check browser is still running
2. Increase timeout settings
3. Review logs for error details

### Context Isolation Error

**Symptoms:**

```
Error: No context available
```

**Solutions:**

1. Always use `api.withPage()` for operations
2. Don't access page outside callback

```javascript
// Correct
await api.withPage(async (page) => {
    await page.goto('https://example.com');
});

// Incorrect - no context!
await page.goto('https://example.com');
```

## Humanization Issues

### Detection by Website

**Symptoms:**

```
Website detects automation
```

**Solutions:**

1. Enable all humanization features
2. Use a different persona
3. Check for canvas fingerprinting

```bash
# In .env
HUMANIZE_MOUSE=true
HUMANIZE_KEYBOARD=true
HUMANIZE_SCROLL=true
PERSONA=stealth
```

## Installation Issues

### Node Module Build Failed

**Symptoms:**

```
Error: better-sqlite3 build failed
```

**Solutions:**

1. Install build tools: `npm install --global windows-build-tools`
2. Or install Visual Studio Build Tools
3. Then rebuild: `npm rebuild better-sqlite3`

### pnpm Install Fails

**Symptoms:**

```
Error: pnpm install failed
```

**Solutions:**

1. Clear cache: `pnpm store prune`
2. Delete node_modules: `rm -rf node_modules`
3. Retry: `pnpm install --force`

## Performance Issues

### Slow Execution

**Symptoms:**

```
Tasks taking too long
```

**Solutions:**

1. Use local LLM for simple tasks
2. Reduce humanization delays
3. Increase concurrent sessions

### Memory Leaks

**Symptoms:**

```
Memory usage growing indefinitely
```

**Solutions:**

1. Run tests to identify issues
2. Check for unclosed pages
3. Review session cleanup

## Getting Help

- Check [GitHub Issues](https://github.com/kardelitaitu/auto-ai/issues)
- Review [API Reference](api.md)
- Check [.agents/](../.agents/) for agent debugging
