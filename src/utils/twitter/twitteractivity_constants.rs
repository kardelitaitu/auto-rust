//! Constants for Twitter activity task.
//! Contains timing constants and other configuration values.

/// Default feed scan duration budget (ms): 5 minutes.
pub const DEFAULT_TWITTERACTIVITY_DURATION_MS: u64 = 300_000;

/// Minimum delay between feed candidate scans (ms).
pub const MIN_CANDIDATE_SCAN_INTERVAL_MS: u64 = 2500;

/// Minimum delay between actions on same tweet (ms).
pub const MIN_ACTION_CHAIN_DELAY_MS: u64 = 3000;

/// Maximum consecutive scroll failures before stopping the feed loop (default for config).
/// The runtime value is now driven by `TwitterActivityConfig::max_consecutive_scroll_failures`.
pub const MAX_CONSECUTIVE_SCROLL_FAILURES: u32 = 3;

/// Maximum consecutive empty candidate scans before stopping the feed loop (default for config).
/// The runtime value is now driven by `TwitterActivityConfig::max_consecutive_empty_scans`.
pub const MAX_CONSECUTIVE_EMPTY_SCANS: u32 = 3;
