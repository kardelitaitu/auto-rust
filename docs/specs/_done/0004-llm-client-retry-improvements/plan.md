# Plan

## What Is the Solution

### Step 1: Replace etry_delay() with exponential-backoff + jitter in acon-pipeline/src/llm/client.rs

Change lines 26-32 from:

`ust
fn retry_delay(attempt: usize) -> Duration {
    match attempt {
        1 => Duration::from_secs(10),
        2 => Duration::from_secs(30),
        _ => Duration::from_secs(60),
    }
}
`

To:

`ust
/// Delay with exponential backoff and jitter.
/// Base delay = BASE_MS * MULTIPLIER^(attempt-1), capped at MAX_DELAY_MS.
/// Jitter randomly varies the delay by ±JITTER_FRACTION of the computed value
/// to prevent thundering-herd when multiple clients retry simultaneously.
fn retry_delay(attempt: usize) -> Duration {
    const BASE_MS: u64 = 1_000;
    const MULTIPLIER: f64 = 2.0;
    const MAX_DELAY_MS: u64 = 60_000;
    const JITTER_FRACTION: f64 = 0.25;

    let base = (BASE_MS as f64) * MULTIPLIER.powi(attempt as i32 - 1);
    let clamped = base.min(MAX_DELAY_MS as f64);
    let jitter_range = clamped * JITTER_FRACTION;
    // Deterministic pseudo-jitter to keep fn pure (no RNG dependency)
    let jitter = (attempt as f64 * 137.508).fract() * jitter_range - (jitter_range / 2.0);
    Duration::from_millis((clamped + jitter) as u64)
}
`

Using deterministic pseudo-jitter (golden-angle-based) instead of and to avoid adding a dependency for a simple jitter. The golden angle (137.508°) provides well-distributed pseudo-random values from the attempt counter.

### Step 2: Add Retry-After header parsing

In the retry loop, after receiving a 429 response, check for the Retry-After header before using the computed delay:

`ust
if let Some(retry_after) = response.headers().get("retry-after") {
    if let Ok(value) = retry_after.to_str() {
        // Retry-After can be a delay in seconds (integer)
        if let Ok(seconds) = value.parse::<u64>() {
            let server_delay = Duration::from_secs(seconds);
            let computed = retry_delay(attempt);
            // Use the longer of the two (server knows best)
            let delay = std::cmp::max(server_delay, computed);
            warn!(... server requested {seconds}s ...);
            sleep(delay).await;
            continue;
        }
    }
}
// Fall back to computed delay
let delay = retry_delay(attempt);
sleep(delay).await;
`

### Step 3: Apply same changes to src/llm/client.rs

The NVIDIA retry path in src/llm/client.rs has the same fixed-stepped pattern. Apply steps 1-2 identically.

For the OpenRouter path (lines 260-395), add the computed delay before falling back to the next model:

`ust
if response.status() == StatusCode::TOO_MANY_REQUESTS {
    warn!("OpenRouter 429 on model {model}; retry delay before fallback");
    let delay = retry_delay(1); // First-attempt delay before fallback
    sleep(delay).await;
    continue; // Try next model
}
`

### Step 4: Update documentation

Update .bacon/workflow.md to document:
- The retry behavior: up to 3 attempts with exponential backoff + jitter
- Retry-After header is respected when present
- Environment variable knobs (future: BACON_LLM_RETRY_BASE_MS, etc.)

### Step 5: Add unit tests

Add tests covering:
1. etry_delay produces increasing delays for attempts 1, 2, 3
2. Retry delay never exceeds MAX_DELAY_MS
3. Jitter varies within ±JITTER_FRACTION of base
4. Retry-After header is preferred when present (longer delay wins)
5. Retry-After invalid value falls back to computed delay
6. OpenRouter 429 triggers delay before fallback

### Files affected

| File | Change |
|---|---|
| acon-pipeline/src/llm/client.rs | Replace etry_delay(), add Retry-After parsing |
| src/llm/client.rs | Same changes + OpenRouter delay |
| .bacon/workflow.md | Document retry behavior |
| acon-pipeline/tests/ (implied) | New retry delay tests |

### Acceptance criteria

1. etry_delay(1) < retry_delay(2) < retry_delay(3) (monotonic increases, subject to jitter distribution)
2. etry_delay(n) <= 60_000 for any n
3. Retry-After header value overrides computed delay when server-specified delay is longer
4. All existing tests pass (cargo nextest run --all-features)
5. check.ps1 passes (spec-lint, build, fmt, clippy, nextest)
