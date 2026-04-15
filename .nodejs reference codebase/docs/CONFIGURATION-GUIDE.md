# Configuration Guide

Complete guide to configuring Auto-AI for your environment.

---

## Configuration Layers

Auto-AI uses a layered configuration system:

```
1. .env                    → Environment variables (API keys, endpoints)
2. config/settings.json    → Automation settings (LLM, humanization, limits)
3. config/browserAPI.json  → Browser discovery ports
4. config/timeouts.json    → Timeout values
```

**Priority**: Environment variables override settings.json values.

---

## 1. Environment Variables (.env)

### AI Provider Configuration

Choose **ONE** option:

#### Option A: Local LLM (Ollama) - Free

```env
LOCAL_LLM_ENDPOINT=http://localhost:11434
LOCAL_LLM_MODEL=hermes3:8b
```

**Setup:**
```bash
# Install Ollama
# Visit https://ollama.ai and download

# Pull a model
ollama pull hermes3:8b

# Start Ollama server
ollama serve
```

**Recommended Models:**
| Model | Size | Speed | Quality | Use Case |
|-------|------|-------|---------|----------|
| `hermes3:8b` | 8B | Fast | Good | Simple tasks |
| `llama3:8b` | 8B | Fast | Good | Navigation, clicks |
| `mistral:7b` | 7B | Fast | Good | Basic automation |
| `gemma3:4b` | 4B | Very Fast | Fair | Quick tasks |

#### Option B: Cloud LLM (OpenRouter) - Paid

```env
OPENROUTER_API_KEY=sk-or-v1-your_key_here
OPENROUTER_DEFAULT_MODEL=anthropic/claude-3.5-sonnet
```

**Setup:**
1. Visit https://openrouter.ai/keys
2. Create API key
3. Add credits (pay-per-use)

**Recommended Models:**
| Model | Speed | Quality | Cost | Use Case |
|-------|-------|---------|------|----------|
| `anthropic/claude-3.5-sonnet` | Fast | Excellent | $$ | Complex decisions |
| `openai/gpt-4-turbo` | Fast | Excellent | $$ | Analysis |
| `meta-llama/llama-3.3-70b-instruct:free` | Medium | Good | Free | General tasks |

### System Settings

```env
# Environment: development, production, test
NODE_ENV=development

# Logging: debug, info, warn, error
LOG_LEVEL=info
```

### Optional Overrides

```env
# Humanization (overrides settings.json)
# HUMANIZE_MOUSE=true
# HUMANIZE_KEYBOARD=true
# PERSONA=casual

# Browser ports
# IXBROWSER_PORT=53200
# BRAVE_PORT=9222

# Advanced
# TASK_TIMEOUT=30000
# MAX_CONCURRENT_SESSIONS=3
```

---

## 2. Automation Settings (config/settings.json)

### LLM Configuration

```json
{
    "llm": {
        "local": {
            "enabled": true,
            "provider": "ollama",
            "endpoint": "http://localhost:11434",
            "model": "hermes3:8b",
            "timeout": 30000
        },
        "cloud": {
            "enabled": false,
            "provider": "openrouter",
            "timeout": 120000,
            "retryAttempts": 1,
            "retryDelay": 3000
        },
        "fallback": {
            "enabled": true,
            "maxRetries": 2,
            "retryDelay": 2000
        }
    }
}
```

**When to enable cloud fallback:**
- Local LLM gives poor quality responses
- Complex decision-making needed
- Error recovery required

### Humanization Settings

```json
{
    "humanization": {
        "mouse": {
            "minDuration": 300,
            "maxDuration": 1500,
            "baseSpeed": 2.0,
            "jitterRange": 2
        },
        "keystroke": {
            "baseDelay": 120,
            "stdDev": 40,
            "punctuationPause": 300,
            "spacePause": 150
        },
        "idle": {
            "wiggleFrequency": 2000,
            "wiggleMagnitude": 5,
            "enabled": true
        }
    }
}
```

**Tuning for stealth:**
- Increase `minDuration`/`maxDuration` for slower, more careful movements
- Increase `stdDev` for more variable timing
- Keep `idle.enabled: true` to simulate presence

### Twitter Automation Settings

```json
{
    "twitter": {
        "session": {
            "minSeconds": 300,
            "maxSeconds": 540,
            "randomProfile": true
        },
        "engagement": {
            "maxReplies": 3,
            "maxRetweets": 1,
            "maxLikes": 5,
            "maxFollows": 2
        },
        "timing": {
            "warmupMin": 1000,
            "warmupMax": 3000,
            "scrollMin": 300,
            "scrollMax": 700,
            "readMin": 5000,
            "readMax": 15000,
            "globalScrollMultiplier": 2.0
        }
    }
}
```

**Adjusting engagement limits:**
- Lower values = safer, less activity
- Higher values = more activity, higher detection risk
- Recommended: Start low, increase gradually

### System Settings

```json
{
    "system": {
        "environment": "development",
        "logLevel": "info",
        "sessionTimeoutMs": 1800000,
        "cleanupIntervalMs": 300000
    }
}
```

**Production tuning:**
- Set `environment: "production"`
- Set `logLevel: "warn"` (less logging)
- Increase `sessionTimeoutMs` for longer sessions

---

## 3. Browser Configuration (config/browserAPI.json)

```json
{
    "ixbrowser": { "port": 53200 },
    "morelogin": { "port": 6699 },
    "dolphin": { "port": 5050 },
    "brave": { "port": 9222 },
    "chrome": { "port": 9222 }
}
```

**Common Ports:**
| Browser | Default Port |
|---------|--------------|
| ixBrowser | 53200 |
| MoreLogin | 6699 |
| Dolphin | 5050 |
| Chrome/Brave | 9222 |
| Edge | 9223 |
| Vivaldi | 9224 |

---

## 4. Timeout Configuration (config/timeouts.json)

```json
{
    "pageLoad": 30000,
    "elementWait": 5000,
    "actionTimeout": 10000,
    "llmResponse": 30000,
    "browserConnect": 15000
}
```

**When to increase timeouts:**
- Slow network connections
- Large pages with many resources
- Complex LLM responses
- Remote browser instances

---

## Use Case Configurations

### Quick Automation (Fast, less stealthy)

```env
# .env
NODE_ENV=development
LOG_LEVEL=info
```

```json
// settings.json
{
    "humanization": {
        "mouse": { "minDuration": 100, "maxDuration": 500 },
        "keystroke": { "baseDelay": 50 }
    },
    "twitter": {
        "timing": { "globalScrollMultiplier": 1.0 }
    }
}
```

### Stealth Mode (Slower, more human-like)

```env
# .env
NODE_ENV=production
LOG_LEVEL=warn
PERSONA=stealth
```

```json
// settings.json
{
    "humanization": {
        "mouse": { "minDuration": 500, "maxDuration": 2000, "baseSpeed": 1.5 },
        "keystroke": { "baseDelay": 200, "stdDev": 60 },
        "idle": { "enabled": true, "wiggleFrequency": 1500 }
    },
    "twitter": {
        "session": { "minSeconds": 600, "maxSeconds": 900 },
        "timing": { "globalScrollMultiplier": 3.0, "readMin": 10000 }
    }
}
```

### Development/Testing (Fast feedback)

```env
# .env
NODE_ENV=development
LOG_LEVEL=debug
HUMAN_DEBUG=true
```

```json
// settings.json
{
    "llm": {
        "local": { "timeout": 15000 },
        "fallback": { "enabled": false }
    },
    "verification": {
        "preFlightEnabled": false,
        "postFlightEnabled": false
    }
}
```

---

## Configuration Validation

After making changes, verify:

```bash
# Check .env syntax
node -e "require('dotenv').config(); console.log('OK')"

# Validate settings.json
node -e "JSON.parse(require('fs').readFileSync('config/settings.json'))"

# Run a simple test
node main.js pageview=https://example.com
```

---

## Troubleshooting

### LLM Not Connecting

**Local:**
```bash
# Check Ollama is running
curl http://localhost:11434/api/tags

# Start Ollama
ollama serve
```

**Cloud:**
```bash
# Test API key
curl -H "Authorization: Bearer $OPENROUTER_API_KEY" \
     https://openrouter.ai/api/v1/models
```

### Browser Not Found

1. Check browser is running
2. Verify remote debugging is enabled
3. Confirm port matches `browserAPI.json`

### Settings Not Applying

1. Check JSON syntax (use JSONLint)
2. Restart the application
3. Check `.env` isn't overriding values

---

## Related Documentation

- [QUICKSTART.md](../QUICKSTART.md) - Quick setup guide
- [docs/configuration.md](configuration.md) - Extended configuration docs
- [docs/troubleshooting.md](troubleshooting.md) - Troubleshooting guide
- [AGENTS.md](../AGENTS.md) - Agent configuration
