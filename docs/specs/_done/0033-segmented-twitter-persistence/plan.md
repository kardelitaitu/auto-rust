last audited 2026-06-27 by antigravity

# Segment Twitter persistence state files by browser profile name

## Baseline
Currently, `TwitterPersistenceState` in `src/utils/twitter/twitteractivity_persistence.rs` loads from and saves to a single hardcoded path: `~/.config/auto-rust/twitter-state.json`, using the lock file `~/.config/auto-rust/twitter-state.json.lock`.
With 500+ concurrent browser sessions, multiple sessions attempting to update this file simultaneously causes lock contention, resulting in timeouts (5s limit).

## Implementation Steps
1. Modify `src/utils/twitter/twitteractivity_persistence.rs`:
   - Update `state_path(profile_name: &str)` and `lock_path(profile_name: &str)` to accept a profile name.
   - If `profile_name` is empty, default to `"default"`.
   - Sanitize `profile_name` to remove any invalid path characters (like `..`, `/`, `\`, etc.) using a simple filter.
   - Update `load(profile_name: &str)`, `save(&self, profile_name: &str)`, and `update_async<F>(profile_name: &str, f: F)` to accept `profile_name`.
   - Update all existing unit tests in `src/utils/twitter/twitteractivity_persistence.rs` to pass a mock profile name like `"test_profile"` or `"test_save_load"`.
2. Modify `src/task/twitteractivity.rs`:
   - In `run_inner`, extract the profile name: `let profile_name = api.behavior_profile().name.as_str();`
   - Pass `profile_name` to `TwitterPersistenceState::load` and `TwitterPersistenceState::update_async`.

## API Changes
No public crate-level API changes, but internal persistence method signatures are updated to require a `profile_name: &str`.

## Validation
- Run unit tests in `twitteractivity_persistence.rs`.
- Run `.\check-fast.ps1` and `.\check.ps1`.

## Design Decisions and Risks
- **Sanitization**: Profile names are usually alphanumeric, but just in case, we will filter character inputs to alphanumeric, dashes, and underscores.
- **Confidence Level**: High.
