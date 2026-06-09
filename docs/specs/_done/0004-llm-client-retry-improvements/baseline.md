# Baseline

## What I Find

**Finding 1: LLM client retry uses fixed stepped delays with no backoff or jitter.**

In acon-pipeline/src/llm/client.rs (lines 26-32):

`ust
fn retry_delay(attempt: usize) -> Duration {
    match attempt {
        1 => Duration::from_secs(10),
        2 => Duration::from_secs(30),
        _ => Duration::from_secs(60),
    }
}
`

The maximum delay is 60s regardless of how many retries have been attempted. If the API is rate-limiting, a 60s delay is too short; if it is a transient blip, the 10s initial delay is unnecessarily long.

The same pattern is duplicated in src/llm/client.rs for the NVIDIA path.

**Finding 2: No Retry-After header parsing anywhere in the codebase.**

Both LLM clients check for HTTP 429 (Too Many Requests) but ignore the Retry-After response header that tells the client exactly how long to wait. The server provides its preferred delay, but the client ignores it and uses its own fixed timing.

**Finding 3: OpenRouter path does not delay before fallback model switch.**

In src/llm/client.rs (lines 260-395), when a 429 is received on the primary model, the client immediately switches to the next fallback model with zero delay, increasing the chance of an immediate second 429 on the fallback.

**Finding 4: The Twitter subsystem already has a proven exponential-backoff-with-jitter implementation.**

In src/utils/twitter/twitteractivity_retry.rs (lines 20-255):

`ust
fn calculate_delay(attempt: u32, config: &RetryConfig) -> u64 {
    let base = config.base_delay_ms as f64 * config.backoff_multiplier.powi(attempt as i32 - 1);
    let delay = base.min(config.max_delay_ms as f64);
    let jitter = if config.jitter_factor > 0.0 {
        let jitter_range = delay * config.jitter_factor;
        let random_jitter = rand::random::<f64>() * jitter_range;
        random_jitter - (jitter_range / 2.0)
    } else { 0.0 };
    (delay + jitter) as u64
}
`

This pattern handles rate-limits gracefully, avoids thundering-herd (via jitter), and is field-tested in production. The LLM clients should use an equivalent approach.

**Finding 5: No dynamic adaptation to API health.**

The retry logic treats all errors the same — three attempts with fixed timing. There is no circuit breaker, no exponential escalation, and no way to respect server-provided timing guidance.

## What I Claim

Upgrading the LLM client retry logic from fixed stepped delays to exponential-backoff-with-jitter (plus Retry-After header parsing) will:

1. **Reduce total wait time during transient blips** — initial retry can be faster (1-2s) when the server doesn't specify a delay.
2. **Increase success rate under sustained rate-limits** — exponential backoff spreads requests further apart, matching typical rate-limit windows (1-60 seconds).
3. **Prevent thundering-herd cascades** — jitter randomizes retry timing, essential when multiple pipeline stages or parallel specs hit the API simultaneously.
4. **Respect server guidance** — Retry-After parsing means we wait exactly as long as the server requests, minimizing both retries and delay.

## What Is the Proof

1. **acon-pipeline/src/llm/client.rs:26-32** — Fixed etry_delay function with three hardcoded values. No exponential growth. No jitter. No Retry-After path. Confirmed by the etry_delay function definition and the absence of any jitter or Retry-After query in the retry loop (lines 143-210).

2. **src/llm/client.rs** — Duplicate of the same fixed-stepped pattern for NVIDIA calls (confirmed by exploration). OpenRouter path (lines 260-395) has zero delay on 429 before switching models.

3. **src/utils/twitter/twitteractivity_retry.rs:20-255** — Proven exponential-backoff-with-jitter implementation with configurable ase_delay_ms, ackoff_multiplier, max_delay_ms, and jitter_factor. Also includes CircuitBreaker (lines 82-170). This demonstrates the pattern works in this codebase and is ready to be applied to the LLM path.

4. **src/utils/twitter/twitteractivity_state.rs:379-465** — A second retry implementation (RateLimitBackoff with true exponential ase * 2^(hits-1)) further confirms the codebase already knows and uses backoff — just not in the LLM clients.

5. **Global g for Retry-After and etry_after** — Zero matches across the entire codebase. No client ever reads this standard HTTP header.
