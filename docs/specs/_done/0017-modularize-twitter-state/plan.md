# Plan

## What Is the Solution

Create `src/utils/twitter/state/` directory, extract 8 structs into 4 submodules:

```
src/utils/twitter/state/
  mod.rs      — module declarations, pub re-exports (≤50 lines)
  types.rs    — TaskValidationError, SentimentTemplates, CandidateContext, CandidateResult (≤250 lines)
  session.rs  — SessionState, RateLimitBackoff + impls (≤300 lines)
  tracking.rs — TweetActionTracker + impls (≤200 lines)
  config.rs   — TaskConfig + impls + from_payload (≤250 lines)
```

### Extraction mapping

| Type/Helper | Lines (est.) | → Target |
|---|---|---|
| `TaskValidationError` + Display + Error impl | ~20 | `types.rs` |
| `SentimentTemplates` + Default impl | ~60 | `types.rs` |
| `CandidateContext`, `CandidateResult` | ~15 | `types.rs` |
| `TaskConfig` + `from_payload()` | ~80 | `config.rs` |
| `read_u64()`, `read_u32()`, `value_kind()` | ~50 | `config.rs` |
| `decision_llm_api_key()` | ~5 | `config.rs` |
| `TweetActionTracker` + impl | ~45 | `tracking.rs` |
| `SessionState` + impl | ~80 | `session.rs` |
| `RateLimitBackoff` + impl | ~90 | `session.rs` |

### Test distribution

| Test module | Tests what | → Target submodule |
|---|---|---|
| `display_tests` | `TaskValidationError` Display | `types.rs` |
| `read_u64_tests` | `read_u64` | `config.rs` |
| `read_u32_tests` | `read_u32` | `config.rs` |
| `payload_tests` | `TaskConfig::from_payload()` | `config.rs` |
| `tdd_tests` (SessionState) | `SessionState` methods | `session.rs` |
| `tdd_tests` (RateLimitBackoff) | `RateLimitBackoff` | `session.rs` |
| `tdd_tests` (TweetActionTracker) | `TweetActionTracker` cooldown | `tracking.rs` |
| `gap_tests` (SentimentTemplates) | Default template contents | `types.rs` |
| `gap_tests` (SessionState) | `is_action_allowed`, action_summary | `session.rs` |
| `gap_tests` (TweetActionTracker) | `record_action`, `can_perform_action` | `tracking.rs` |
| `gap_tests` (RateLimitBackoff) | `calculate_delay`, exponential growth | `session.rs` |
| `gap_tests` (TaskConfig) | `from_payload` edge cases | `config.rs` |
| `gap_tests` (read_u64/read_u32) | Validation edge cases | `config.rs` |
| `test_support` | Test helper functions | stays in `twitteractivity_state.rs` (shared by all) |

### Wire `state/mod.rs` with re-exports

```rust
mod config;
mod session;
mod tracking;
mod types;

pub use config::TaskConfig;
pub use session::{RateLimitBackoff, SessionState};
pub use tracking::TweetActionTracker;
pub use types::{CandidateContext, CandidateResult, SentimentTemplates, TaskValidationError};
```

### Update `twitteractivity_state.rs`

Replace all extracted bodies with a re-export shim (≤50 lines):
```rust
// Re-exports — types moved to state/ submodules
mod state;
pub use state::{
    CandidateContext, CandidateResult, RateLimitBackoff, SentimentTemplates,
    SessionState, TaskConfig, TaskValidationError, TweetActionTracker,
    read_u32, read_u64,
};

// Shared test helpers stay here
#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test_support { ... }
```

### Wire `state/mod.rs`

```rust
mod config;
mod session;
mod tracking;
mod types;

pub use config::{read_u32, read_u64, TaskConfig};
pub use session::{RateLimitBackoff, SessionState};
pub use tracking::TweetActionTracker;
pub use types::{CandidateContext, CandidateResult, SentimentTemplates, TaskValidationError};
```

### Add `pub mod state;` to `src/utils/twitter/mod.rs`

### Note: `CandidateContext` cross-module dependency

`CandidateContext` (in `types.rs`) imports `PersonaWeights` from `decision/`, `TaskConfig` from `config.rs`,
`EngagementLimits` from `session.rs`, and `TweetActionTracker` from `tracking.rs`.
It also references `TaskContext` from the public runtime API.
The `types.rs` submodule must include `use crate::utils::twitter::{decision::PersonaWeights, ...}`
to resolve these intra-crate imports.

So submodules can import from sibling paths like `crate::utils::twitter::state::*`.

### Verify

```bash
cargo check -p auto-rust
cargo test --lib
cargo clippy --all-targets --all-features
```

### Files changed

| File | Action | Target lines |
|------|--------|-------------|
| `src/utils/twitter/twitteractivity_state.rs` | Shrink to re-export shim | ≤50 |
| `src/utils/twitter/state/mod.rs` | New | ≤50 |
| `src/utils/twitter/state/types.rs` | New | ≤250 |
| `src/utils/twitter/state/session.rs` | New | ≤300 |
| `src/utils/twitter/state/tracking.rs` | New | ≤200 |
| `src/utils/twitter/state/config.rs` | New | ≤250 |
