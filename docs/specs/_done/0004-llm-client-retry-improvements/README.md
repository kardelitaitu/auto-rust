# Add Exponential Backoff, Jitter, and Retry-After Support to LLM Clients

Status: done

Owner: spec-agent
Implementer: spec-agent

## Summary

The bacon-pipeline and main crate LLM clients both use fixed stepped retry delays (10s/30s/60s) that ignore the server's Retry-After header and lack backoff/jitter. The Twitter subsystem already has a proven exponential-backoff-with-jitter implementation. This proposal upgrades the LLM clients to use exponential backoff with jitter and respect Retry-After headers, improving resilience under API rate-limits and transient failures.

## Scope

- **acon-pipeline/src/llm/client.rs**: Replace etry_delay() fixed-step function with exponential backoff + jitter; add Retry-After header parsing.
- **src/llm/client.rs**: Same changes as bacon-pipeline client (NVIDIA path); OpenRouter path should also apply delay before fallback switch.
- **.bacon/workflow.md**: Document the new retry behavior and env-var knobs.
- **Tests**: All existing retry tests (bacon-pipeline and src/) must pass unchanged. New unit tests for backoff calculation, jitter bounds, and Retry-After parsing will be added.

## Next Steps

1. Implement in acon-pipeline/src/llm/client.rs
2. Implement in src/llm/client.rs
3. Update docs in .bacon/workflow.md
4. Add unit tests for delay calculation and Retry-After parsing
5. Run check.ps1 (spec-lint, build, fmt, clippy, nextest)
