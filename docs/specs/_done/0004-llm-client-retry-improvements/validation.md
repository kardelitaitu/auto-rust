# Validation

## Acceptance Criteria

- [x] LLM retry delay uses exponential backoff with configurable base and multiplier
- [x] Jitter factor is applied to retry delays to avoid thundering-herd
- [x] Retry-After response header is parsed and respected when present
- [x] All existing retry tests continue to pass without modification
- [x] LLM retry behavior is documented in .bacon/workflow.md
