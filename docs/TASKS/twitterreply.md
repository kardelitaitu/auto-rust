# Twitter Reply Task

Extract tweet context and compose human-like replies with optional LLM integration.

## Quick Start

```bash
# Reply to specific tweet
cargo run twitterreply=url=https://x.com/user/status/123
```

## Features

- 📝 **Context Extraction**: Reads tweet + top replies from DOM
- 🎯 **Sentiment Analysis**: Matches reply tone to context
- ⚡ **Quick Generation**: Fast, template-based replies
- 🤖 **LLM-Powered**: Contextual AI-generated replies (when enabled)
- 🔧 **Content Validation**: Sanitizes output for Twitter compliance

## LLM Integration

When LLM is enabled in config, replies are generated using:

- Tweet author and text as context
- Up to 5 top replies for conversation understanding
- Configurable provider (Ollama or OpenRouter)
- Automatic fallback to template on LLM failure

### Enable LLM

```toml
[twitter_activity.llm]
enabled = true
provider = "ollama"
model = "llama3.2:latest"
```

## Content Validation

LLM output is validated to ensure:

- Maximum 280 characters
- No @mentions (unless in original tweet)
- No #hashtags
- No emojis
- No banned AI-sounding words

## Payload Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `url` | string | Tweet URL to reply to |
| `text` | string | Manual reply text (bypasses LLM) |

## How It Works

1. Navigates to tweet URL
2. Extracts tweet text and author
3. Reads up to 5 top replies for context
4. Performs sentiment analysis
5. Generates reply (LLM if enabled, otherwise template)
6. Validates content against Twitter rules
7. Types reply with human-like timing
8. Submits reply

## Reply Generation Modes

| Mode | When Used | Speed |
|------|-----------|-------|
| Template | LLM disabled or failed | Fast (<100ms) |
| LLM | Enabled and available | Medium (1-3s) |
| Manual | `text` parameter provided | Fast |

## Examples

```bash
# Auto-generate reply via LLM
cargo run twitterreply=url=https://x.com/user/status/123

# Use manual text (no LLM)
cargo run 'twitterreply={"url":"https://x.com/user/status/123","text":"Great point!"}'
```

## Related Tasks

- [`twitteractivity`](twitteractivity.md) - Full engagement with optional replies
