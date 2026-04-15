# Configuration

Learn how to configure Auto-AI for your needs.

## Environment Variables

### LLM Configuration

| Variable              | Description                     | Default                               |
| --------------------- | ------------------------------- | ------------------------------------- |
| `OPENROUTER_API_KEY`  | OpenRouter API key for cloud AI | -                                     |
| `OPENROUTER_BASE_URL` | OpenRouter base URL             | `https://openrouter.ai/api/v1`        |
| `OPENROUTER_MODEL`    | Model to use                    | `anthropic/claude-3.5-sonnet`         |
| `LOCAL_LLM_ENDPOINT`  | Local LLM endpoint              | `http://localhost:11434/api/generate` |
| `LOCAL_LLM_MODEL`     | Local model name                | `llama3`                              |
| `USE_LOCAL_LLM`       | Prefer local over cloud         | `true`                                |

### Browser Configuration

| Variable          | Default | Description             |
| ----------------- | ------- | ----------------------- |
| `BROWSER_PORT`    | `9222`  | Default CDP port        |
| `BROWSER_TIMEOUT` | `30000` | Connection timeout (ms) |
| `IXBROWSER_PORT`  | `8855`  | ixBrowser port          |
| `MORELOGIN_PORT`  | `35000` | MoreLogin port          |

### Humanization

| Variable            | Default   | Description                   |
| ------------------- | --------- | ----------------------------- |
| `HUMANIZE_MOUSE`    | `true`    | Enable human-like mouse       |
| `HUMANIZE_KEYBOARD` | `true`    | Enable keystroke dynamics     |
| `HUMANIZE_SCROLL`   | `true`    | Enable random scroll patterns |
| `PERSONA`           | `default` | Behavior persona name         |

### Timeouts

| Variable         | Default | Description         |
| ---------------- | ------- | ------------------- |
| `PAGE_TIMEOUT`   | `30000` | Page load timeout   |
| `ACTION_TIMEOUT` | `10000` | Action timeout      |
| `IDLE_DELAY_MIN` | `1000`  | Min idle delay (ms) |
| `IDLE_DELAY_MAX` | `5000`  | Max idle delay (ms) |

## Configuration Files

### `config/settings.json`

Main settings file:

```json
{
    "llm": {
        "provider": "openrouter",
        "model": "anthropic/claude-3.5-sonnet",
        "temperature": 0.7,
        "maxTokens": 4096
    },
    "humanization": {
        "enableMouse": true,
        "enableKeyboard": true,
        "enableScroll": true,
        "persona": "default"
    },
    "timeouts": {
        "page": 30000,
        "action": 10000,
        "idleMin": 1000,
        "idleMax": 5000
    }
}
```

### `config/browserAPI.json`

Browser discovery ports:

```json
{
    "ixBrowser": 8855,
    "moreLogin": 35000,
    "dolphan": 18500,
    "undetectable": 9222,
    "roxybrowser": 19992
}
```

### `config/timeouts.json`

Timeout values:

```json
{
    "page": 30000,
    "action": 10000,
    "navigation": 30000,
    "wait": 5000
}
```

## Persona Configuration

Personas define behavior patterns. Set via `config/settings.json`:

```json
{
    "humanization": {
        "persona": "default" // or "aggressive", "stealth", "casual"
    }
}
```

## Loading Configuration

```javascript
import { config } from './api/utils/config.js';

// Load all settings
const settings = config.load();

// Get specific section
const llmConfig = config.get('llm');
```

## Examples

### Use Only Local LLM

```bash
USE_LOCAL_LLM=true
LOCAL_LLM_ENDPOINT=http://localhost:11434/api/generate
LOCAL_LLM_MODEL=llama3
```

### Custom Browser Ports

```bash
BROWSER_PORT=9223
IXBROWSER_PORT=8856
```

### Disable Humanization

```bash
HUMANIZE_MOUSE=false
HUMANIZE_KEYBOARD=false
HUMANIZE_SCROLL=false
```
