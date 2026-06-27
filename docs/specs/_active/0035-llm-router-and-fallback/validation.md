last audited 2026-06-27 by antigravity – ✅ PASS

## Acceptance Criteria

1. **Configurable Chain**: The LLM client loads `LLM_FALLBACK_ENABLED` and `LLM_ROUTING_CHAIN` from environment variables, configuring a list of model targets.
2. **Sequential Failover**: `chat_with_fallback` attempts each configured target sequentially upon failure, returning immediately on the first success.
3. **Fallback Disabling**: If `LLM_FALLBACK_ENABLED=false`, the client halts after the first target fails and returns the error without calling fallbacks.
4. **Ollama Pool**: If `OLLAMA_URL` contains comma-separated URLs, Ollama chat calls rotate between them round-robin.
5. **CI Health**: `.\check.ps1` and `.\check-fast.ps1` compile and pass without errors.

## Test Commands
- `cargo test llm::client`
- `.\check-fast.ps1`
- `.\check.ps1`
