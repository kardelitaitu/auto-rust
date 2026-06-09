## Acceptance Criteria
- auditor::run() signature accepts `&dyn LlmClient` — existing callers pass `&Llm` with auto-coercion
- 5+ unit tests in auditor.rs cover the ad-hoc path (no spec on disk):
  - LLM returns PASS → output description starts with "PASS"
  - LLM returns FAIL → ctx.needs_human_approval set
  - LLM returns error → function propagates error
  - Confidence tag in response → parsed into output.confidence
  - Dry-run flag preserved from ctx to output
- All tests pass without network, git, or external agent binaries
- `check-fast.ps1` passes

## Test Commands
- `cargo nextest run --all-features -p bacon-pipeline --lib`
- `cargo clippy --all-features -p bacon-pipeline`
- `cargo fmt --all --check`
- `.\check-fast.ps1`

## Visual Inspection
- `auditor.rs` line 12: `pub async fn run(llm: &dyn crate::core::LlmClient, ...)` instead of `&crate::llm::Llm`
- `auditor.rs` end of file: new `#[cfg(test)] mod tests` block with MockLlmClient + 5+ test functions
- No `#[cfg(test)]` block existed before — verify with `rg '#\[cfg\(test\)\]' auditor.rs` returning 1 match
