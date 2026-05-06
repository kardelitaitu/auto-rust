# LLM Client Factory Modularization

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary
This is a textbook violation of the Open/Closed Principle and a massive "God Function". Having a single factory function span over a thousand lines implies it is hardcoding the initialization, routing, and error handling for every single supported LLM provider inline. Every time a new provider is added, this function grows.

## Scope
- **In scope**: Refactoring described in the plan.
- **Out of scope**: Changing core logic.

## Next Step
Begin implementation according to plan.md.

# Baseline

## What I Find
The `src/llm/client.rs` file is 1,379 lines long. Astoundingly, a single function named `create_llm_client_from_config` is **1,092 lines long**.

## What I Claim
This is a textbook violation of the Open/Closed Principle and a massive "God Function". Having a single factory function span over a thousand lines implies it is hardcoding the initialization, routing, and error handling for every single supported LLM provider inline. Every time a new provider is added, this function grows.

## What Is the Proof
1. Context execution analysis confirms `create_llm_client_from_config` has an exact length of 1,092 lines.
2. The file length correlates directly to this single function, making unit testing provider initialization in isolation impossible.

