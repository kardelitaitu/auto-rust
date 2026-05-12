# Plan

### Step 1: Fix post-dive candidate scan reset
**File:** `engagement.rs`
- After `goto_home` succeeds (~line 847), reset `next_candidate_scan = now + candidate_scan_interval`.

### Step 2: Bounded sleep in main loop
**File:** `twitteractivity.rs`
- Sleep in max-1s increments with `session.is_expired()` checks between them.

### Step 3: Decouple should_dive from non-like actions
**File:** `engagement.rs`
- Remove `actions_to_do.retain(|a| a == "like")` gate at line 329.

### Step 4: Fix PersonaStrategy multiplier
**File:** `persona.rs`
- Unify: `base * interest_multiplier` for all levels (clamped 0-2).

### Step 5: Remove dead actions_taken
**Files:** `twitteractivity.rs`, `engagement.rs`, `state.rs`
- Remove from `process_candidate()` signature and `CandidateResult`.

### Step 6: Fix cookie banner selectors
**File:** `popup.rs`
- Replace `:contains()` with JS text-matching loop over buttons.

### Step 7: Seed select_entry_point
**File:** `navigation.rs`
- Accept `seed: u64`, use `StdRng::seed_from_u64(seed)`.

### Step 8: Fix dive pause
**File:** `engagement.rs`
- Use `max(60s, dive_elapsed * 2)` instead of constant 300s.

### Step 9: Lazy regex
**File:** `llm_validation.rs`
- Use `std::sync::OnceLock<regex::Regex>`.

### Step 10: Verify
- `.\check-fast.ps1` and `.\check.ps1`.
- Verify simulation distribution matches expected.
