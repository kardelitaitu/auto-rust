# LLM Setup Guide for Auto-Rust

## Quick Start (Recommended: Ollama - FREE)

### 1. Install Ollama
**Windows:**
```powershell
# Download from: https://ollama.ai/download
# Or via winget:
winget install Ollama.Ollama
```

**After install:** Ollama runs automatically in system tray

### 2. Download Model
```bash
# Open PowerShell or Command Prompt
ollama pull llama3.2:3b

# Verify installation
ollama list
```

Expected output:
```
NAME            ID           SIZE      MODIFIED
llama3.2:3b     365c0bd3c000   2.0 GB    2 days ago
```

### 3. Test Ollama
```bash
ollama run llama3.2:3b "Hello, are you working?"
```

Expected: AI responds with a message

### 4. Configure Auto-Rust
The `.env` file is already configured for Ollama:
```
OLLAMA_ENABLED=true
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_MODEL=llama3.2:3b
```

### 5. Test Integration
```bash
# Build the project
cargo build --all-features

# Run a simple Twitter test (requires browser connected)
cargo run twittertest=url=https://x.com/elonmusk/status/1234567890
```

---

## Alternative: OpenRouter (Paid)

### When to Use:
- Need access to premium models (Claude-3, GPT-4)
- Don't want to run models locally
- Have budget for API costs (~$0.50 per 1000 tweets)

### Setup:
1. **Get API Key:**
   - Go to https://openrouter.ai/
   - Sign up → Keys → Create New Key
   - Copy key (starts with `sk-or-v1-...`)

2. **Add Credits:**
   - Minimum: $5 USD
   - Navigate to Billing → Add Credits

3. **Update `.env`:**
   ```
   OLLAMA_ENABLED=false
   OPENROUTER_ENABLED=true
   OPENROUTER_API_KEY=sk-or-v1-YOUR_KEY_HERE
   OPENROUTER_MODEL=anthropic/claude-3-haiku
   ```

---

## Troubleshooting

### Ollama Not Responding
```bash
# Check if Ollama is running
ollama list

# Restart Ollama service
# Windows: Right-click system tray icon → Quit, then restart from Start menu

# Test the model
ollama run llama3.2:3b "hi"
```

### Model Too Slow
```bash
# Try a smaller model
ollama pull phi3:mini       # Very fast, decent quality

# Update .env:
OLLAMA_MODEL=phi3:mini
```

### Better Quality Needed
```bash
# Try larger models
ollama pull llama3.1:8b     # Better quality
ollama pull mixtral:latest  # Best quality (larger)

# Update .env:
OLLAMA_MODEL=llama3.1:8b
```

### Connection Refused
```bash
# Ollama might not be running on default port
# Check .env:
OLLAMA_BASE_URL=http://localhost:11434

# If you changed Ollama's port, update accordingly
```

### LLM Tests Failing
```bash
# Test LLM directly first:
ollama run llama3.2 "Say hello"

# If that works, test integration:
cargo test --lib llm

# Check logs for details:
RUST_LOG=debug cargo run twittertest=...
```

---

## Model Recommendations

### For Twitter Engagement:

| Model | Size | Speed | Quality | Cost |
|-------|------|-------|---------|------|
| **llama3.2:3b** | 2GB | ⚡⚡⚡ | Good | FREE |
| **phi3:mini** | 1GB | ⚡⚡⚡⚡ | Decent | FREE |
| **mistral:latest** | 4GB | ⚡⚡ | Very Good | FREE |
| **llama3.1:8b** | 8GB | ⚡ | Excellent | FREE |

### For OpenRouter:

| Model | Cost/1K | Quality |
|-------|---------|---------|
| **claude-3-haiku** | $0.25 | Very Good |
| **gpt-3.5-turbo** | $0.50 | Good |
| **llama-3-8b** | $0.05 | Decent |

---

## Performance Tips

1. **Start with llama3.2** - Best balance of speed/quality
2. **Use SSD** - Models load faster
3. **16GB+ RAM** - Prevents swapping
4. **Close other apps** - Frees up RAM for Ollama
5. **Test with small batches first** - Verify quality before scaling

---

## Next Steps

1. ✅ Install Ollama
2. ✅ Pull llama3.2 model
3. ✅ Test with `ollama run`
4. ✅ Run `cargo run twittertest` to verify integration
5. ✅ Start live testing with real tweets!

## Support

- Ollama Docs: https://ollama.ai/docs
- Auto-Rust Issues: GitHub repository
- Model Discussions: Ollama Discord
