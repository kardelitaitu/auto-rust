# Plan

## What Is the Solution
**Modularize Providers**: Refactor the factory into a true Strategy or Builder pattern. Each provider (e.g., `openai.rs`, `anthropic.rs`) should implement a `ProviderFactory` trait responsible for parsing its specific config subset and returning the configured client. `create_llm_client_from_config` should merely be a dispatch router (< 50 lines).

# internal api outline

Implementation details to be defined during active development.

# decisions

Implementation details to be defined during active development.

