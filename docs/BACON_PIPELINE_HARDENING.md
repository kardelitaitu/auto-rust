# Bacon-Pipeline Hardening & Improvement Plan

*Not "write more tests." The pipeline already has strong compile-time guarantees.
The best returns come from hardening the LLM boundary, spec integrity, and recovery paths.*

---

## Layer 1: Types over Tests (highest ROI)

Encode invariants so invalid states are unrepresentable.

- [ ] **Newtype `SpecId`** — `u32` spec numbers parsed from directory names (`0001-add-foo`) are currently raw. Wrap in `struct SpecId(u32)` with `FromStr` and `Display` so every path construction goes through validation.
- [ ] **Newtype `StageName`** — Strategist/Coder resume by parsing stage strings from CLI/config. `Stage::from_name()` returns `Option` but callers differ on handling. Make `StageName` typed so invalid stages are caught at parse time.
- [ ] **Newtype `ConfidenceScore`** — `Confidence` enum exists, but `extract_confidence()` parses free text from LLM output into `Option<Confidence>`. Wrap the parser result so "medium/Medium/unknown" handling is centralized.
- [ ] **`NonZeroU32` for retry counters** — `attempt`, `max_attempts`, `consecutive_refusals` are `u32` but semantically never 0 when active. Use `NonZeroU32` to catch arithmetic bugs.
- [ ] **`enum` for patch outcomes** — `PipelineCtx` currently uses multiple `Option` fields (`patch_path`, `coder_refused`, `needs_human_approval`, `scope_reduction_needed`) that can be inconsistently set. Replace with `enum PatchStatus { Applied(PathBuf), Queued(PathBuf), Refused, NeedsHumanApproval, RetriesExhausted, DryRun }`.

## Layer 2: Property-Based Testing (`proptest`)

Hand-written tests find the cases you think of. Proptest finds the cases you don't.

- [ ] **`extract_priority()`** — property: any string containing `priority: P0..P3` or synonyms (`critical`, `high`, `medium`, `low`) maps to the correct `P0..P3` variant; strings without a priority keyword default to `P2`.
- [ ] **`extract_area()`** — property: comma/space-separated tags are normalized to lowercase, empty input returns `["bacon"]`.
- [ ] **`slugify()`** — property: output is lowercase, hyphens only, length ≤ 40, roundtrip preserves uniqueness for ASCII input.
- [ ] **`status_id_from_url` equivalent for spec paths** — property: roundtrip `spec_path` → serialize → deserialize → same path.
- [ ] **Patch parser robustness** — property: given any byte sequence, `parse_search_replace_blocks()` never panics; returns either `Ok(Vec<Block>)` or a structured error (never `unwrap`/`expect` failure).

## Layer 3: Fuzzing (`cargo fuzz`)

Best for parsing, deserialization, and any code that touches untrusted input.

- [ ] **LLM response parser** — the `LlmDecision` / patch extraction logic handles malformed JSON, truncated responses, unexpected fields, nested search/replace blocks. Fuzz it.
- [ ] **Spec YAML loader** — `spec.yaml` parsing accepts unknown keys, wrong types, missing fields. Ensure graceful error, not panic or `unwrap`.
- [ ] **Patch block parser** — SEARCH/REPLACE block splitting is regex-based. Fuzz with pathological delimiters, nested markers, and unicode.

## Layer 4: Mutation Testing (`cargo mutants`)

Verifies tests actually catch bugs instead of just passing.

- [ ] **Install** — `cargo install cargo-mutants`
- [ ] **Baseline** — run on `strategist.rs` extraction helpers first (pure functions, easy to mutate).
- [ ] **Threshold** — aim for < 10% surviving mutants on core logic modules.
- [ ] **Target** — `extract_priority`, `extract_area`, `validate_autonomous_plan`, `is_refusal`, `signal_scope_reduction`.

## Layer 5: Coverage-Guided Gap Analysis (`cargo llvm-cov`)

Untested branches, not untested lines.

- [ ] **`check_fast` / `check_full` fallback paths** — when `run_check_fast()` returns `Err`, does the Coder retry loop handle every `anyhow` error variant?
- [ ] **`GitSnapshot` rollback branches** — rename/copy/snapshot failures in `RealFileSystem` are untested.
- [ ] **Confidence extraction branches** — `match trimmed.to_lowercase().as_str()` in Strategist has exhaustive arms; are all of them reachable in tests?
- [ ] **Refusal handling** — `is_refusal()` in `coder.rs` has many phrases; are partial matches, unicode, and empty strings covered?

## Layer 6: Dynamic Analysis (`cargo miri`)

Detects undefined behavior in unsafe code. Run weekly.

- [ ] **`miri` on test suite** — check for UB in dependency crates too.
- [ ] **Focus** — any `unsafe` block, `transmute`, raw pointer arithmetic, FFI boundaries. Currently the codebase is likely safe Rust, but `git2` / `reqwest` / `tokio` internals may have UB triggers.

## Layer 7: Pipeline Resilience Improvements

These are not strictly testing, but address the most common failure modes seen in CI:

- [ ] **Atomic spec handoff** — `spec-lint.ps1` reads + writes `spec.yaml`; a crash mid-write can corrupt status. Use temp-file + rename pattern.
- [ ] **Idempotent stage execution** — if `bacon` is killed mid-Coder and restarted with `--stage coder`, does it resume correctly or duplicate work? Add stage-idempotency checks.
- [ ] **Structured error types for LLM calls** — replace ad-hoc `anyhow::Result` with domain errors (`LlmError::Timeout`, `LlmError::RateLimit`, `LlmError::InvalidResponse`) so retry logic can branch on cause.
- [ ] **Spec lock file** — prevent two pipeline instances from modifying the same spec directory concurrently. Simple `.lock` file with PID + mtime TTL.

## Priority Order

1. **Newtypes** (`SpecId`, `StageName`, `ConfidenceScore`) — quick, mechanical, prevents entire bug class at parse boundaries.
2. **Proptest** on `extract_priority`, `extract_area`, `slugify` — find edge cases in LLM output parsing now.
3. **Fuzz** LLM response parser + patch block parser — untrusted input, high impact.
4. **Mutants** baseline on Strategist extraction helpers — quick confidence boost for core decision logic.
5. **Coverage gap** on retry/rollback paths — find untested branches before they become live incidents.
6. **State machine** for patch outcomes — larger refactor, prevents logic bugs permanently in Coder/Auditor handoff.
