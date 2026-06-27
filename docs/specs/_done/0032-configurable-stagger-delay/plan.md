last audited 26-06-26 by antigravity

# Plan: Configurable Stagger Delay

## Baseline
- `task_stagger_delay_ms` is defined in `OrchestratorConfig` as a `u64` representing the staggering delay in milliseconds.
- Currently, it defaults to `500` (in `defaults.rs` and `mod.rs.backup`) or `2000` (in `load_code_config` fallback).
- There is no option to override this delay via an environment variable / `.env` config.

## Proposed Changes

### 1. Update `src/config/env.rs`
In the function `apply_env_overrides(mut config: Config) -> Result<Config>`, add support for `TASK_STAGGER_DELAY_MS` key:
```rust
    if let Ok(stagger) = env::var("TASK_STAGGER_DELAY_MS") {
        config.orchestrator.task_stagger_delay_ms = stagger
            .parse()
            .unwrap_or(config.orchestrator.task_stagger_delay_ms);
    }
```
Place this check inside `apply_env_overrides` alongside other orchestrator settings (e.g. after `MAX_RETRIES` or `TASK_TIMEOUT_MS` override).

### 2. Add Unit Test to `src/config/tests.rs`
Add `test_task_stagger_delay_env_override()` to test suite verifying correct behavior:
- Storing existing environment keys.
- Setting `TASK_STAGGER_DELAY_MS` to a valid value (e.g., `4500`) and verifying it overrides the default value.
- Setting it to an invalid value (e.g., `"not-a-number"`) and verifying it falls back to the default.
- Cleaning up the environment variables.

## Rationale
- Stagger delay is vital when launching 500+ browser sessions to prevent overwhelming the local proxy or network interfaces.
- Exposing it via environment variables allows dynamic fine-tuning on different servers without rebuilding the code.
