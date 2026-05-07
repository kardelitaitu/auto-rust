# Plan

## What Is the Solution
**Modularize Providers**: Refactor the factory and client logic using a Strategy pattern. 

1. **Define Trait**: Create `src/llm/provider.rs` defining the `ProviderClient` trait:
   - `async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String>;`
   - `async fn health_check(&self) -> bool;`
2. **Modularize Providers**: Move existing provider-specific logic from `client.rs` into dedicated modules:
   - `src/llm/providers/ollama.rs` (Implement `ProviderClient`)
   - `src/llm/providers/openrouter.rs` (Implement `ProviderClient`)
3. **Dispatch Router**: Refactor `create_llm_client_from_config` in `src/llm/client.rs` to initialize and return a `Box<dyn ProviderClient>`.

## Verification & Testing
- **Wiremock Preservation**: All existing `wiremock` integration tests currently in `src/llm/client.rs` must be migrated to the new provider modules or a shared test file, ensuring the fallback, timeout, and error-handling logic remains fully covered.
- **Regression Testing**: Execute the existing `wiremock` suite before and after refactoring to ensure 100% test parity.

# internal api outline

- `src/llm/provider.rs`: `ProviderClient` trait definition.
- `src/llm/providers/`: Directory for provider-specific implementations.

# decisions

- Strategy Pattern: Use a trait-based factory rather than a monolithic match-statement.
- Polymorphism: Return `Box<dyn ProviderClient>` to hide implementation details from the `Llm` wrapper.
- Error Handling: Keep existing `anyhow::Result` error boundaries to ensure no logic regressions.

