# Baseline

## What I Find

### 1. Session pool uses linear backoff with no jitter

`session/pool.rs:187-204`

The `discover_and_connect` retry loop sleeps `1000ms`, `2000ms`, `3000ms` linearly.
No jitter is applied. In a multi-session scenario (parallel execution), all
retries synchronize — a classic thundering-herd pattern.

### 2. LLM client (auto-rust crate) has its own fallback retry logic

`llm/client.rs` contains OpenRouter fallback logic with its own retry
configuration, timeout handling, and model failover. This is independent
from the session pool retry and the bacon-pipeline LLM retry (0014).

### 3. API client has a configurable retry policy with wiremock tests

`api/client.rs` defines a `RetryPolicy` struct and uses it in HTTP calls.
It has 3 wiremock tests (`test_api_client_get_success`,
`test_api_client_get_with_key_auth_header`,
`test_api_client_retry_on_500_then_success`). This is a third independent
implementation.

### 4. Twitter utility has its own retry_with_backoff

`utils/twitter/twitteractivity_retry` defines `RetryConfig` and
`retry_with_backoff` with domain-specific Aggressive and Default presets.

### 5. Bacon-pipeline already proves the pattern works

`0014-llm-client-retry-improvements` added exponential backoff with jitter
to the bacon-pipeline LLM client. That spec explicitly listed "Extract shared
retry library from Twitter utilities (separate follow-up)" as a non-goal,
confirming this consolidation was always intended as the next step.

## What I Claim

A shared `src/utils/retry.rs` module with `RetryConfig`,
`ExponentialBackoff` iterator, and `retry_with_backoff` adapter
will:

- Eliminate duplicated retry logic (3+ implementations -> 1)
- Make retry behavior consistent across session, API, and LLM boundaries
- Add jitter to session pool discovery, reducing thundering-herd risk
- Provide a single, well-tested retry primitive for future consumers

## What Is the Proof

1. **3 retry implementations exist** in the non-bacon-pipeline crate:
   `session/pool.rs` (linear), `api/client.rs` (custom policy),
   `utils/twitter/twitteractivity_retry` (custom presets) — each
   with a different API surface and test coverage level.

2. **Session pool backoff has no jitter** — `pool.rs` uses
   `1000 * attempt` ms directly with no randomization. This is
   the worst-case thundering-herd scenario when multiple sessions
   retry simultaneously after a browser restart.

3. **The bacon-pipeline 0014 spec explicitly deferred this work**:
   Its non-goals include "Extract shared retry library from Twitter
   utilities (separate follow-up)", confirming the team's intent
   to consolidate retry logic outside the pipeline crate.

4. **Exponential backoff + jitter is a proven pattern in the repo**:
   Bacon-pipeline's LLM client already uses it (from 0014).
   Porting the same pattern to a shared module is low-risk,
   well-understood work.
