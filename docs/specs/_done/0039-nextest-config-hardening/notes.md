## Implementation Notes

- Updated `.config/nextest.toml` to harden the CI nextest profile with retries, immediate-final failure output, and a terminating slow-timeout.
- Kept the CI workflow invocation unchanged so it still uses `cargo nextest run --all-features --profile ci`.
- Verified the repo gate still passes after the config change.
