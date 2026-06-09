# Auditor Agent Unit Tests via Dependency Injection

## Baseline

The `auditor.rs` module (209 lines) is the final gate in the bacon pipeline — it reads the spec + plan + validation criteria + git diff, calls the LLM for an audit decision (PASS/FAIL), archives passing specs to `_done/`, and writes failure reports to `validation.md`. It has **zero unit tests**. There is no `#[cfg(test)]` block anywhere in the file.

### Evidence

1. **209 lines, 0 tests**: `rg -c '#\[test\]|#\[tokio::test\]' auditor.rs` returns empty. No `#[cfg(test)]` block exists.
2. **18 external dep sites** across the 209 lines: `spec_io::read_spec_meta` (×2), `spec_io::read_spec_file` (×2), `std::fs::read_to_string` (×4), `std::fs::write` (×2), `std::process::Command::new("git")` (git diff), `llm.chat(messages)` (LLM API), `extract_confidence` (confidence parsing), `read_role_prompt` (file read), `run_spec_lint` (shell command), `promote_to_done` (spec archive), `write_audit_report` (file write).
3. **Proven pattern exists**: Observer tests (added in the prior session) proved that `&dyn LlmClient` injection + `MockLlmClient` + `Once::call_once` + temp-dir setup works end-to-end. The same infrastructure applies with zero new dependencies.
4. **Criticality**: The auditor is the final quality gate. A silent PASS-ALL bug ships broken code; a silent FAIL-ALL bug blocks all work. Zero test coverage on a gate this critical is a measurable risk.

### Problem

The `auditor::run()` function takes `&crate::llm::Llm` (concrete type), making it impossible to inject a mock LLM in tests. The `LlmClient` trait already exists in `core/traits.rs` and `impl LlmClient for Llm` was added in the prior session — but auditor.rs still uses the concrete type.

### Solution

1. Change `auditor::run()` signature from `&crate::llm::Llm` to `&dyn crate::core::LlmClient` (1 line, same pattern as observer).
2. The caller `pipeline.rs:282` (`super::auditor::run(&llm, &self.args, ctx)`) requires no change — `&Llm` auto-coerces to `&dyn LlmClient`.
3. Add `#[cfg(test)] mod tests` with `MockLlmClient` (reuse the same struct pattern from observer) + tests for the ad-hoc path.

### Implementation Steps

1. **`bacon-pipeline/src/agent/auditor.rs` line 12**: Change `pub async fn run(llm: &crate::llm::Llm, ...)` to `pub async fn run(llm: &dyn crate::core::LlmClient, ...)`.
2. No caller changes needed. 3 call sites (pipeline.rs:157, pipeline.rs:282, bacon-review.rs:97) all pass `&Llm` which auto-coerces to `&dyn LlmClient`.
3. **`bacon-pipeline/src/agent/auditor.rs`**: Append `#[cfg(test)] mod tests` with:
   - `MockLlmClient` struct implementing `crate::core::LlmClient`
   - `setup()` function with `Once::call_once` + temp-dir config init
   - `make_args()` / `make_ctx()` helpers
   - Tests for ad-hoc path (no `spec_path`):
     - `llm_returns_pass_returns_pass_context` — auditor returns PASS
     - `llm_returns_fail_returns_needs_approval` — auditor returns FAIL
     - `llm_error_propagates` — LLM error → function error
     - `confidence_extracted_from_response` — `Confidence: High` in response → parsed
     - `dry_run_flag_propagates` — ctx.dry_run → output.dry_run

### API Changes

- `auditor::run()` changes from `llm: &crate::llm::Llm` to `llm: &dyn crate::core::LlmClient`
- No functional change, no other public API surface changed

### Validation

- `cargo nextest run --all-features -p bacon-pipeline --lib` passes all 155+ tests
- `cargo clippy --all-features -p bacon-pipeline` clean
- `cargo fmt --all --check` clean
- `.\check-fast.ps1` passes

### Design Decisions and Risks

- **Ad-hoc path only**: The full spec-path tests (spec.yaml on disk, git diff, promote_to_done, write_audit_report) are deferred to a follow-up. The ad-hoc path covers LLM decision logic and confidence extraction — the most critical paths — without needing spec-lint or git.
- **`scope_reduction_needed` field**: Not removed. It's still set by coder.rs and kept for potential future use, even though no production code path currently reads it.
- **Risk**: MockLlmClient responses must include `PASS`/`FAIL` as the first whitespace-delimited token (line 127: `decision_first = ...split_whitespace().next()`) — test data must follow this convention exactly.
- **Confidence: Low / Medium / Low**

## Verification Steps
1. `cargo nextest run --all-features -p bacon-pipeline --lib`
2. `cargo clippy --all-features -p bacon-pipeline`
3. `cargo fmt --all --check`
4. `.\check-fast.ps1`
