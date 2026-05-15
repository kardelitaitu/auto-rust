# Bacon Pipeline Improvement Roadmap

> **Status**: Planning document  
> **Scope**: All `.bacon/` role prompts, `src/bacon_agent_pi/`, `src/bacon_agent_nvidia/`, configuration, tests, and documentation  
> **Audience**: Developers maintaining and extending the Bacon pipeline  

---

## Overview

This document defines a sequenced improvement plan for the Bacon autonomous coding pipeline. The items are ordered by dependency — each phase builds on the previous one. Skipping phases creates technical debt that makes later work harder.

### Quick Reference

| Phase | Focus | Items | Est. Effort | Status |
|-------|-------|-------|-------------|--------|
| **0** | Foundation & Infrastructure | 4 tasks | ~5 days | ✅ Complete |
| **1** | Pipeline Correctness | 4 tasks | ~4 days | ✅ Complete |
| **2** | Trust & Observability | 3 tasks | ~2 days | ✅ Complete |
| **3** | Production Hardening | 5 tasks | ~6 days | ✅ Complete |

**Note**: Effort estimates are optimistic and assume a single developer. Add 30–50% for context switching, code review, and unexpected edge cases.

---

## Phase 0 — Foundation & Infrastructure

*Goal: Eliminate the most dangerous technical debt before adding new features.*

---

### 0.1 Extract Shared Pipeline Core from Dual Implementations ✅

**File(s)**:  
- `src/bacon_agent_pi/pipeline.rs`  
- `src/bacon_agent_nvidia/pipeline.rs`  
- `src/bacon_agent_pi/spec_io.rs`  
- `src/bacon_agent_nvidia/spec_io.rs`  

**Current State**:  
The `pi` and `nvidia` agent directories each contain a nearly identical copy of the pipeline orchestrator. Both define:
- `Stage` enum (Observer, Strategist, Coder, Auditor) — **duplicated**
- `PipelineConfig` struct with agent-per-stage routing — **duplicated**
- `PipelineCtx` for passing state between stages — **duplicated**
- `run_external_agent()` function — **duplicated**
- `check_stale_in_progress()` — **duplicated**
- `WorkerOutput` JSON deserialization — **duplicated**
- `spec_io` module (list/move/copy specs) — **duplicated**

The two copies have already diverged slightly (different validation rules, different agent routing defaults). Every future change must be made twice, and the divergence will only accelerate.

**Solution**:  
Extract the shared core into `src/bacon_core/` with the following module layout:

```
src/bacon_core/
  mod.rs          # Re-exports
  pipeline.rs     # Pipeline orchestrator (single copy)
  config.rs       # PipelineConfig, agent routing, LLM config
  context.rs      # PipelineCtx, WorkerOutput
  spec_io.rs      # Spec filesystem operations (list, move, copy)
  errors.rs       # Shared error types
  external.rs     # run_external_agent() — shared binary resolution
  recovery.rs     # check_stale_in_progress() — crash recovery
```

Each agent module (`bacon_agent_pi`, `bacon_agent_nvidia`) then becomes thin wrappers that:
1. Create their `PipelineConfig` from `bacon.toml`
2. Pass it to `bacon_core::pipeline::Pipeline::new(cfg)`
3. Run `pipeline.run()`

**Migration Steps**:
1. Create `src/bacon_core/` directory structure
2. Move `Stage` enum, `PipelineConfig`, `PipelineCtx`, `WorkerOutput` into shared types
3. Move `run_external_agent()` into `external.rs` — parameterize agent-specific binary resolution
4. Move `check_stale_in_progress()` into `recovery.rs`
5. Move `spec_io` functions into `spec_io.rs`
6. Refactor `pi/pipeline.rs` to delegate to `bacon_core`
7. Refactor `nvidia/pipeline.rs` to delegate to `bacon_core`
8. Delete duplicated code from both agent directories
9. Update all imports across the codebase
10. Run `cargo check && cargo test && cargo clippy -D warnings`

**Validation**: Both pipeline smoke tests must pass. No behavioral change — purely structural.

---

### 0.2 Fix Section Header Parsing in Strategist ✅

**File(s)**:  
- `src/bacon_agent_pi/strategist.rs` (function `extract_section`)  
- `src/bacon_agent_nvidia/strategist.rs` (equivalent function)  
- `.bacon/roles/02_bacon-strategy.md` (prompt documentation)

**Current State**:  
The `extract_section()` function uses this logic to find section content:

```rust
let in_section = line.to_lowercase().starts_with("##") 
    && line.to_lowercase().contains(keyword);
```

Where `keywords` are bare substrings like `"baseline"`, `"api"`, `"validat"`, `"risk|decision|design"`.

This is fragile because:
- `contains("validat")` matches `## Validation`, `## Invalidate Cache`, `## Validation Approach`, `## Validate Input`
- `contains("baseline")` matches `## Baseline`, `## Baseline and Current State`, `## Post-Baseline Concerns`
- `contains("api")` matches `## API Changes`, `## API Design`, `## No API Surface Changes`, `## NAPIs` (yes, this could match accidentally)

The LLM prompt now warns about this, but the root cause is the parser, not the prompt. A single unexpected `##` header can corrupt the entire spec package.

**Solution**:  
Replace the substring `contains()` with a regex that matches the **exact header word** after `## `:

```rust
use regex::Regex;

// Match: "## " followed by optional whitespace, then an optional non-capturing 
// group for the keyword, followed by word boundary or end of string.
// The keyword list is: baseline, implementation, api, validat, risk, decision, design
fn extract_section(plan: &str, keywords: &str) -> String {
    let re = Regex::new(
        &format!(r"(?m)^##\s+(?:\w+\s+)?({})\b", 
                 keywords.replace("|", "|"))
    ).unwrap();
    // ... extract from match position to next ## header
}
```

This matches:
- `## Baseline` → match
- `## Validation` → match (via `validat` + `ion` word boundary)
- `## Baseline and Current State` → NO match (extra words before the keyword)
- `## Invalidate Cache` → NO match (`Invalidate` starts with `In` before `validat`)

Wait — actually the current behavior allows `## Baseline and Current State` to match because `contains("baseline")` is true. The regex approach with `\b` word boundary would also match `## Baseline Discussion` because `Baseline` starts at word boundary. The real fix is to match the **exact keyword** as the header text, not just a substring.

Better approach:

```rust
fn extract_section(plan: &str, keyword: &str) -> String {
    // Match: "##" followed by whitespace, then exactly the keyword (case-insensitive),
    // optionally followed by more text like ": ...", then end of understanding.
    // But we want to capture the section until the next ## header.
    let pattern = format!(r"(?im)^##\s+{}\b.*$", regex::escape(keyword));
    // ...
}
```

**However**, there's a complication. The current keywords are truncations: `validat` matches both `Validation` and `Validate`. The correct approach is:

```rust
// Define exact header names, not substrings
const SECTION_KEYWORDS: &[(&str, &str)] = &[
    ("baseline", "baseline.md"),
    ("implementation steps", "plan.md"),  // part of plan.md
    ("api changes", "internal-api-outline.md"),
    ("validation", "validation.md"),
    ("design decisions and risks", "notes.md"),
];

fn extract_section(plan: &str, keyword: &str) -> String {
    // Match: line starts with ##, then optional whitespace,
    // then the keyword (case-insensitive), with word boundary
    // followed by optional colon + text
    let re = Regex::new(
        &format!(r"(?im)^##\s+{}[\s\S]*?(?=^##\s|\z)", 
                 regex::escape(keyword))
    ).unwrap();
    
    if let Some(cap) = re.find(plan) {
        cap.as_str().to_string()
    } else {
        String::new() // caller uses fallback
    }
}
```

**Migration Steps**:
1. Add `regex` to `Cargo.toml` dependencies (check if already present)
2. Replace `extract_section()` with regex-based implementation
3. Update keyword constants from truncations to exact header words
4. Update `02_bacon-strategy.md` prompt to reference exact header names
5. Run `cargo test` with focus on strategist tests
6. Manually test with a sample spec that has `## Baseline`, `## Validate`, `## Validation` — only the exact match should extract

---

### 0.3 Add End-to-End Integration Tests for the Full Pipeline ✅

**File(s)**:  
- `tests/bacon_dry_run_smoke.rs` (existing)  
- New test files as needed

**Current State**:  
Only two test files exist for the pipeline:
- `bacon_dry_run_smoke.rs` — Tests that `--dry-run --auto` exits cleanly and prints "Stage 1: Observer". Does not verify spec creation, coder output, or auditor decisions.
- `bacon_cli_worker_contract.rs` — Tests that each CLI worker outputs valid JSON for the `observer` role only. Does not test `strategist`, `coder`, or `auditor` roles.

There are no tests that:
- Create a real spec and verify the directory structure
- Exercise the coder validation gate (diff → git apply → check-fast.ps1)
- Verify the auditor PASS/FAIL logic
- Test the retry loop with mock error feedback
- Test external agent JSON parsing with malformed input
- Test crash recovery (`_active/` with `status: in-progress`)
- Test `--fast` mode skips Strategist and Auditor

**Solution**:  
Add a new integration test module `tests/bacon_pipeline_integration.rs`:

```rust
// Test 1: Full pipeline creates spec and verifies directory structure
#[test]
fn full_pipeline_creates_spec_package() {
    // Given: a mock LLM that returns a valid strategist plan
    // When: running pipeline with --auto --dry-run
    // Then: _active/ directory contains a spec with plan.md, baseline.md, spec.yaml
}

// Test 2: Coder validation gate accepts valid patches
#[test]
fn coder_validation_gate_accepts_valid_diff() {
    // Given: a valid unified diff that passes check-fast.ps1
    // When: verify_and_queue_patch is called
    // Then: patch is saved to approved_patches/
}

// Test 3: Coder validation gate rejects invalid patches
#[test]
fn coder_validation_gate_rejects_invalid_diff() {
    // Given: an invalid diff (wrong syntax, non-existent file)
    // When: verify_and_queue_patch is called
    // Then: returns error, spec stays in-progress
}

// Test 4: Auditor PASS promotes to _done/
#[test]
fn auditor_pass_promotes_to_done() {
    // Given: an implemented spec in _active/
    // When: auditor run with LLM returning "PASS"
    // Then: spec moved to _done/ with status=done
}

// Test 5: Auditor FAIL moves to needs-human-approval
#[test]
fn auditor_fail_marks_needs_approval() {
    // Given: an implemented spec in _active/
    // When: auditor run with LLM returning "FAIL: ..."
    // Then: spec stays in _active/ with status=needs-human-approval
    // And: validation.md contains the audit report
}

// Test 6: Retry loop feeds error back to LLM
#[test]
fn retry_loop_includes_error_feedback() {
    // Given: a coder attempt that produces a bad patch
    // When: validation fails
    // Then: error output is included in the next LLM call
}

// Test 7: External agent malformed JSON is handled gracefully
#[test]
fn external_agent_malformed_json_returns_error() {
    // Given: an external agent returning non-JSON output
    // When: run_external_agent parses the output
    // Then: pipeline aborts with clear error message
}

// Test 8: --fast mode skips strategist and auditor
#[test]
fn fast_mode_skips_stages() {
    // Given: pipeline started with --fast
    // When: running
    // Then: Observer runs, Strategist skipped, Coder runs, Auditor skipped
}
```

**Mock LLM Strategy**:  
Extend the `spawn_fake_ollama` pattern from `bacon_dry_run_smoke.rs` to support deterministic responses for each role. Create a `MockLlmServer` that returns pre-configured responses based on the request path:

```rust
struct MockLlmServer {
    observer_response: String,
    strategist_response: String,
    coder_response: String,
    auditor_response: String,
}
```

---

## Phase 1 — Pipeline Correctness

*Goal: Make each pipeline stage work with real information, not guesses.*

---

### ⚡ 2.1 Config Cleanup ✅

[Phase 1.4](#14-make-nvidia-agent-read-from-baconroles-files) adds a new config key to `bacon.toml`. Phase 2.1 (config cleanup) happens after Phase 1 — this is intentional: the cleanup removes *unimplemented* keys, and new keys added in Phase 1 are *implemented* by the same work. If you reorder, add Phase 2.1 to Phase 0 instead.

---

### 1.1 Include Source File Contents in Coder Prompt ✅

**File(s)**:  
- `src/bacon_agent_pi/coder.rs` (the `run` function where prompt is constructed)  
- `src/bacon_agent_nvidia/coder.rs` (equivalent)  
- `.bacon/roles/03_bacon-coder.md` (prompt already updated — now needs code to match)

**Current State**:  
The Coder prompt only receives spec metadata files (`plan.md`, `baseline.md`, `internal-api-outline.md`, `validation.md`). These describe *what* to change but do not include the actual source file contents. The LLM is asked to generate precise unified diffs for files it has never seen.

The prompt now says *"The Rust code includes relevant source file context in your prompt. Study this context carefully"* — but the Rust code does not do this. The prompt is aspirational.

**Solution**:  
Before calling the LLM, read every source file referenced in the spec files and include their contents in the prompt:

```rust
// In coder.rs, before constructing the LLM prompt:
async fn read_spec_files_and_context(spec_path: &Path) -> Result<PromptContext> {
    let plan = read_file(spec_path.join("plan.md"))?;
    let baseline = read_file(spec_path.join("baseline.md"))?;
    let api_outline = read_file(spec_path.join("internal-api-outline.md"))?;
    
    // Parse referenced files from the spec
    let referenced_files = extract_file_paths(&plan, &baseline, &api_outline);
    
    let mut file_contents = Vec::new();
    for path in referenced_files {
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => file_contents.push((path, content)),
            Err(e) => warn!("Could not read referenced file {}: {}", path, e),
        }
    }
    
    PromptContext { plan, baseline, api_outline, file_contents }
}

// Helper: extract paths like `src/utils/dom.rs` from markdown text
fn extract_file_paths(texts: &[&str]) -> Vec<PathBuf> {
    // Use regex for Rust file paths in backticks or plain text
    // Pattern: `src/**/*.rs` or `src/**/*.toml`
    let re = Regex::new(r"`([\w/.-]+\.(?:rs|toml))`").unwrap();
    // ...
}
```

**Prompt Construction**:
```rust
let context = read_spec_files_and_context(spec_path).await?;

let mut prompt = format!(
    "## Spec Plan\n\n{}\n\n## Baseline\n\n{}\n\n",
    context.plan, context.baseline
);

if !context.file_contents.is_empty() {
    prompt.push_str("## Current Source Files\n\n");
    for (path, content) in &context.file_contents {
        prompt.push_str(&format!("### {}\n```rust\n{}\n```\n\n", 
                        path.display(), content));
    }
}

if let Some(error_feedback) = retry_error {
    prompt.push_str(&format!(
        "## Previous Validation Errors\n\n```\n{}\n```\n\n", 
        error_feedback
    ));
}
```

**Migration Steps**:
1. Implement `extract_file_paths()` using regex for Rust source files referenced in spec text
2. Implement `read_spec_files_and_context()` to read spec files + referenced source files
3. Modify the prompt construction to include file contents before calling the LLM
4. Set a max context size limit (e.g., 10 files, 500 lines each) to avoid token overflow
5. Update `03_bacon-coder.md` prompt to remove the tool-access assumption (already done in previous edit — verify it reads correctly)
6. Run `cargo test` — existing tests must still pass
7. Manual test: run pipeline with a spec that references a real file — verify the file contents appear in the prompt (use `--dry-run --log-level debug`)

---

### 1.2 Send Diff + Spec Criteria to Auditor for Real Review ✅

**File(s)**:  
- `src/bacon_agent_pi/auditor.rs` (the `run` function)  
- `src/bacon_agent_nvidia/auditor.rs` (equivalent)  
- `.bacon/roles/04_bacon-auditor.md` (prompt)

**Current State**:  
The Auditor receives only: spec title, spec status (`implemented`), and spec path. It is asked to evaluate:

1. Does the implementation match the spec's acceptance criteria?
2. Are all stated goals met?
3. Any missed edge cases or regressions?
4. Is the scope appropriate?

But it cannot answer any of these without seeing the actual diff. The Auditor stage is currently a rubber stamp — it always PASSes because it has no information to challenge with.

**Solution**:  
Send three pieces of information to the Auditor:

1. **The spec files** — `spec.yaml` (for acceptance criteria, scope, non-goals), `validation.md` (for verification criteria)
2. **The diff** — `git diff` output showing all changes from the Coder
3. **The implementation notes** — `implementation-notes.md` (any notes from the Coder about what was done)

```rust
// In auditor.rs, enhanced prompt construction:
async fn build_auditor_prompt(spec_path: &Path) -> Result<String> {
    let spec_yaml = read_file(spec_path.join("spec.yaml"))?;
    let validation = read_file(spec_path.join("validation.md")).ok();
    let impl_notes = read_file(spec_path.join("implementation-notes.md")).ok();
    
    // Get the diff — we need the git diff at the current HEAD
    // Since the patch was already validated and potentially applied,
    // we need the approved patch file
    let diff = read_approved_patch(spec_path)?;
    // OR: run `git diff` against the working tree
    
    Ok(format!(
        "## Spec Metadata\n\n```yaml\n{}\n```\n\n\
         ## Validation Criteria\n\n{}\n\n\
         ## Implementation Notes\n\n{}\n\n\
         ## Changes (Diff)\n\n```diff\n{}\n```",
        spec_yaml,
        validation.unwrap_or_default(),
        impl_notes.unwrap_or_default(),
        diff
    ))
}
```

**The diff source**: The Coder queues the approved patch to `.bacon/sessions/approved_patches/<spec_path>.diff`. The Auditor should read this file to get the exact diff that was validated.

**Migration Steps**:
1. Modify auditor prompt construction to read and include spec.yaml, validation.md, and the approved diff
2. Update `04_bacon-auditor.md` prompt to describe the new information available
3. Update the review checklist to explicitly reference acceptance criteria from `spec.yaml`
4. Run `cargo test`
5. Manual test: create an implemented spec, run auditor with `--dry-run --log-level debug`, verify the prompt contains the diff

---

### 1.3 Handle Coder Refusal Gracefully ✅

**File(s)**:  
- `src/bacon_agent_pi/coder.rs` (function `run`, `verify_and_queue_patch`)  
- `src/bacon_agent_nvidia/coder.rs` (equivalent)  
- `.bacon/roles/03_bacon-coder.md` (prompt)

**Current State**:  
The Coder prompt (from our improvements) tells the LLM to *"state that clearly rather than generating a likely-wrong patch"* if it can't determine the correct approach. But `coder.rs` always expects a unified diff from the LLM — `extract_unified_diff()` will fail to find a diff, triggering the retry loop with **no error feedback** (the error is just "no diff found", not actual compiler output). The LLM retries, produces the same refusal, and the cycle repeats until all retries are exhausted.

**Solution**:  
Handle refusal responses explicitly before attempting diff extraction:

```rust
fn run(...) -> Result<...> {
    let response = llm.call(prompt).await?;
    
    // Check for explicit refusal before trying to extract a diff
    let refusal_patterns = [
        "cannot determine",
        "can't determine",
        "unable to implement",
        "cannot generate",
        "insufficient context",
        "file contents are missing",
        "cannot produce a valid patch",
    ];
    
    let lower = response.to_lowercase();
    if refusal_patterns.iter().any(|p| lower.contains(p)) {
        // Not a failure — the LLM identified a blocker.
        // Return the refusal reason as structured output so the
        // pipeline can feed it back to the Strategist for scope reduction.
        return Err(anyhow::anyhow!(
            "Coder refused: {}", 
            truncate_to_first_sentence(&response, 200)
        ));
    }
    
    // Normal path: extract diff from response
    let diff = extract_unified_diff(&response)?;
    // ...
}
```

The key insight: a refusal is **not** a failed attempt — it's a signal that the spec is unworkable. The error message gets fed to the Phase 1.4 fallback loop (or to human review), not recycled into a meaningless retry.

**Prompt update**:  
In `03_bacon-coder.md`, change the instruction from:
```
- If you cannot determine the correct approach because file contents are
  missing, state that clearly rather than generating a likely-wrong patch.
```
to:
```
- If you cannot determine the correct approach because file contents are
  missing or the spec is contradictory, respond with:
  "CANNOT IMPLEMENT: <one-sentence reason>"
  Then explain briefly. This is more useful than a wrong patch because it
  triggers scope reduction rather than silent retries.
```

This way the `extract_unified_diff` fallback is still safe (it won't match a fenced refusal), and the refusal-handling code can check for the `CANNOT IMPLEMENT:` prefix explicitly.

**Migration Steps**:
1. Add refusal-pattern checking logic to Coder before diff extraction
2. Update `03_bacon-coder.md` prompt to use explicit `CANNOT IMPLEMENT:` prefix
3. Wire the refusal error into the Phase 1.4 fallback loop (or human escalation if fallback doesn't exist yet)
4. Run `cargo test`

---

### 1.4 Add Coder→Strategist Fallback Loop ✅

**File(s)**:  
- `src/bacon_agent_pi/pipeline.rs` (the orchestrator)  
- `src/bacon_agent_nvidia/pipeline.rs` (equivalent)  
- `src/bacon_agent_pi/coder.rs` (retry loop)  
- `src/bacon_agent_nvidia/coder.rs` (retry loop)  

**Current State**:  
When the Coder exhausts 3 retry attempts, the pipeline:
1. Sets spec status to `needs-human-approval`
2. Stops
3. Requires human intervention

This breaks "24/7 continuous operation" promises. There is no automated fallback to a simpler approach.

**Solution**:  
Add an optional scope-reduction loop: after the Coder fails 3 retries, instead of going directly to human, return to the Strategist with the error feedback.

```rust
// Enhanced pipeline loop:
#[derive(Debug, Clone, Copy, PartialEq)]
enum PipelineState {
    Running,
    CoderRetry(u8),
    ScopeReduction(u8),  // New: return to Strategist
    NeedsHuman,
    Complete,
}

// In the run() method:
for coder_retry in 0..MAX_CODER_RETRIES {
    match run_coder(...) {
        Ok(ctx) => { /* proceed to auditor */ break; }
        Err(e) if coder_retry < MAX_CODER_RETRIES - 1 => {
            // Normal retry with error feedback
            retry_with_feedback(e);
        }
        Err(e) if scope_reduction_attempts < MAX_SCOPE_REDUCTIONS => {
            // Coder exhausted retries — return to Strategist
            let reduced_plan = strategist.reduce_scope(
                &original_plan, 
                &e.error_output
            ).await?;
            scope_reduction_attempts += 1;
            coder_retry = 0; // Reset coder retries with reduced plan
        }
        Err(e) => {
            // Both coder and scope reduction exhausted — go to human
            mark_needs_human_approval(spec, e);
            break;
        }
    }
}
```

**Strategist Scope Reduction Prompt**:
```
The Coder was unable to implement this plan after 3 attempts.
Last error: <compiler/test output>

Please produce a REDUCED SCOPE version of this plan that:
1. Removes or simplifies the parts that failed
2. Keeps the parts that are working or straightforward
3. Is significantly smaller (target: 50% fewer lines)
4. Lists what was deferred to a follow-up spec

Keep the same ## section headers as the original plan.
```

**Config Parameters** (add to `bacon.toml`):
```toml
[workflow]
max_scope_reductions = 2          # How many times to return to Strategist
scope_reduction_target = 0.5      # Target 50% size reduction each iteration
```

**Migration Steps**:
1. Add `PipelineState` enum to track the new states
2. Add `MAX_SCOPE_REDUCTIONS` constant (or config parameter)
3. Add scope reduction prompt in Strategist (or as a new variant of strategist prompt)
4. Modify pipeline orchestrator to implement the fallback loop
5. Update `02_bacon-strategy.md` to describe scope reduction as a scenario
6. Run `cargo test`
7. Test: simulate persistent coder failure, verify pipeline returns to Strategist with reduced scope

---

## Phase 2 — Trust & Observability

*Goal: Make the pipeline honest about what it can and cannot do, and surface useful metrics.*

---### ✅ COMPLETED: Phase 2.1 — Clean Up Configuration

**What was done**:
Removed 5 unused config sections (29+ keys) with zero Rust code backing from `.bacon/bacon.toml`:

| Removed Section | Keys Removed |
|----------------|-------------|
| `[workflow]` | stages, auto_approve, fast_mode, dry_run, max_retries, timeout_seconds |
| `[global]` | log_level, max_concurrent_jobs, retry_attempts |
| `[monitoring]` | enable_metrics, metrics_file, log_file, max_log_size_mb, dashboard_enabled, dashboard_port |
| `[monitoring.alerts]` | success_rate_below, avg_duration_above, error_rate_above, queue_depth_above, memory_usage_above, disk_usage_above |
| `[safety]` | enable_shadow_testing, shadow_workspace_dir, max_shadow_age_hours, enable_rollback, rollback_depth, enable_auto_apply, require_full_check_for_auto_apply, security_scan_enabled |

**Preserved** (have code backing): `[pipeline]`, `[jobs.*]`, `[agents.*]`

**Also audited**: `.bacon/.bacon-workflow.md` role docs — no references to removed keys found. `bacon-test.rs` config parsing test updated to check for `[pipeline]` and `[agents.]` sections instead of removed keys.

---

### 2.2 Standardize Confidence Format + Surface in Metrics ✅

**File(s)**:  
- `.bacon/roles/03_bacon-coder.md` and `04_bacon-auditor.md`  
- `src/bacon_core/mod.rs` (`Confidence` enum + `extract_confidence()`)  
- `src/bacon_agent_pi/observer.rs`, `strategist.rs`, `coder.rs`, `auditor.rs`  
- `src/bacon_agent_nvidia/observer.rs`, `strategist.rs`, `coder.rs`, `auditor.rs`  
- `docs/BACON_IMPROVEMENT_ROADMAP.md`

**What Changed**:

**Role prompts standardized:**
- `03_bacon-coder.md`: `Patch confidence:` → `Confidence:`  
- `04_bacon-auditor.md`: `Review confidence:` → `Confidence:`  
- Observer and Strategist already used the standard format

**New code in `src/bacon_core/mod.rs`:**
- `Confidence` enum with `High`, `Medium`, `Low` variants
- `Confidence::from_str()` — case-insensitive parsing
- `Confidence::as_str()` — static string representation
- `extract_confidence(response: &str) -> Option<Confidence>` — searches for `Confidence: High/Medium/Low` on its own line, handles markdown formatting (`**Confidence: High**`), trailing punctuation, and case-insensitive matching

**All 8 agents wired:**
- After each LLM call, agents call `extract_confidence(&response)` and log via `info!("... confidence={:?}", confidence)`
- Both PI and NVIDIA variants covered

**Tests:**
- 10 unit tests covering: standard values, empty/no match, markdown bold, trailing punctuation, case-insensitive, multi-line, invalid value, first-match-wins
- `cargo check` passes

---

### 2.3 Streamline Spec Package Generation ✅

**File(s)**:  
- `src/bacon_agent_pi/strategist.rs` (function `write_spec_package_in`)  
- `src/bacon_agent_nvidia/strategist.rs` (equivalent)  
- `spec-lint.ps1` (may need updating)  

**Current State**:  
Every Strategist run generates 11 files per spec:

| File | Content | Is it needed? |
|------|---------|---------------|
| `spec.yaml` | Metadata | Yes |
| `plan.md` | Full plan from LLM | Yes |
| `baseline.md` | ## Baseline section | Yes, if non-empty |
| `internal-api-outline.md` | ## API Changes section | Only if API changes exist |
| `validation.md` | ## Validation section | Yes, if non-empty |
| `notes.md` | ## Design Decisions/Risks | Yes, if non-empty |
| `README.md` | Auto-generated boilerplate | Debatable |
| `ci-commands.md` | `(TBD by Coder)` placeholder | No — always empty |
| `quality-rules.md` | `(TBD by Coder)` placeholder | No — always empty |
| `validation-checklist.md` | Auto-generated defaults | No — static boilerplate |
| `implementation-notes.md` | `(TBD by Coder)` placeholder | Filled later by Coder |

For a 15-line mechanical change, 7 of these files are empty boilerplate. Over 100 specs that's 700 files of `(TBD by Coder)`.

**⚠️ Prerequisite: Audit `spec-lint.ps1`**

Before removing any files, analyze what `spec-lint.ps1` checks:
```bash
cat spec-lint.ps1 | grep -i "spec\|baseline\|ci-commands\|quality\|validate"
```

If `spec-lint.ps1` validates that all 11 files exist (checksums, required paths), removing them will break the lint pass. The solution must either:
- (a) Update `spec-lint.ps1` to only require files that are actually generated, or
- (b) Keep stub files but with a note that they're auto-generated placeholders (less clean, but safer)

**What was done**:

Implemented **Option A** — removed boilerplate files `internal-api-outline.md` and `implementation-notes.md` from both strategists' `write_spec_package_in` functions. The spec package now generates 6 files matching the template:

| File | Purpose |
|------|---------|
| `spec.yaml` | Metadata (always generated) |
| `plan.md` | Full plan from LLM (always generated) |
| `baseline.md` | ## Baseline section (extracted from plan) |
| `validation.md` | ## Validation section (extracted from plan) |
| `notes.md` | ## Design Decisions/Risks (extracted from plan) |
| `README.md` | Package readme (required by spec-lint.ps1) |

**Files removed**: `internal-api-outline.md` (was always "No API changes" boilerplate), `implementation-notes.md` (was always "TBD by Coder" boilerplate). Neither was checked by `spec-lint.ps1`.

**Migration Steps completed**:
1. ✅ Audited `spec-lint.ps1` — it checks only: README.md, spec.yaml, plan.md, validation.md, notes.md
2. ✅ Removed `internal-api-outline.md` and `implementation-notes.md` from both strategists
3. ✅ Updated nvidia strategist test to match new 6-file set
4. ✅ Ran `cargo test --lib` — 2542 passed, 0 failed

---

## Phase 3 — Production Hardening

*Goal: Make the pipeline extensible, safe, and fully tested.*

---

### 3.0 Prerequisite: Verify `spec_io.rs` Divergence ✅

**File(s)**:  
- `src/bacon_core/spec_io.rs` (canonical shared implementation)  
- `src/bacon_agent_pi/spec_io.rs` (now re-exports from `bacon_core`)  
- `src/bacon_agent_nvidia/spec_io.rs` (now re-exports from `bacon_core`)  

**What was done**:

The two copies had minor differences (different function ordering, nvidia was missing `read_spec_meta`). A canonical shared `src/bacon_core/spec_io.rs` was created with the best of both — all public functions and comprehensive tests. Both agent-level files now re-export from `bacon_core`:

```rust
pub use crate::bacon_core::spec_io::*;
```

No consumer import changes were needed — all code uses `use super::spec_io;` which resolves through the local module.

✅ Added `pub mod spec_io;` to `bacon_core/mod.rs`
✅ Created canonical `bacon_core/spec_io.rs` with all functions + tests
✅ Both pi and nvidia `spec_io.rs` now re-export
✅ `cargo test --lib` — 2542 passed, 0 failed

---

### 3.1 Define `PipelineAgent` Trait for All Agent Implementations ✅

**File(s)**:  
- New: `src/bacon_core/agent.rs`  
- Modified: `src/bacon_agent_pi/mod.rs`, `src/bacon_agent_nvidia/mod.rs`  
- All existing role implementation files

**Current State**:  
Each agent module (`pi`, `nvidia`, `codex`, `gemini`, `kilocode`, `opencode`, `ollama`) implements its own version of the four pipeline roles. There is no shared trait, so:
- Adding a new agent requires implementing all 4 roles from scratch
- There's no contract enforcement — a new agent might forget to implement a required function
- Code reuse between agents is copy-paste, not inheritance

**Solution**:  
Define a trait that each agent must implement:

```rust
// src/bacon_core/agent.rs

/// A Bacon pipeline agent capable of executing all four pipeline stages.
#[async_trait]
pub trait PipelineAgent: Send + Sync {
    /// The agent's display name (e.g., "nvidia", "codex", "ollama")
    fn name(&self) -> &str;
    
    /// Execute the Observer stage: scan the codebase and identify improvements.
    async fn observe(&self, ctx: &PipelineCtx) -> Result<PipelineCtx>;
    
    /// Execute the Strategist stage: create an implementation plan.
    async fn strategize(&self, ctx: &PipelineCtx) -> Result<PipelineCtx>;
    
    /// Execute the Coder stage: generate and validate patches.
    async fn code(&self, ctx: &PipelineCtx) -> Result<PipelineCtx>;
    
    /// Execute the Auditor stage: review the implementation.
    async fn audit(&self, ctx: &PipelineCtx) -> Result<PipelineCtx>;
    
    /// Optional: reduce scope when the Coder fails (Phase 1.3).
    async fn reduce_scope(&self, ctx: &PipelineCtx, error: &str) -> Result<PipelineCtx> {
        // Default implementation: log and return error — agents opt in
        Err(anyhow::anyhow!("Scope reduction not supported"))
    }
}

/// A local LLM-based agent that uses the bacon role prompts.
pub struct LocalLlmAgent {
    name: String,
    llm: Arc<Llm>,
    config: AgentLlmConfig,
}

#[async_trait]
impl PipelineAgent for LocalLlmAgent {
    fn name(&self) -> &str { &self.name }
    
    async fn observe(&self, ctx: &PipelineCtx) -> Result<PipelineCtx> {
        // Load 01_bacon-observer.md prompt, construct user prompt, call LLM
        observer::run(&self.llm, ctx).await
    }
    
    async fn strategize(&self, ctx: &PipelineCtx) -> Result<PipelineCtx> {
        strategist::run(&self.llm, ctx).await
    }
    
    async fn code(&self, ctx: &PipelineCtx) -> Result<PipelineCtx> {
        coder::run(&self.llm, ctx).await
    }
    
    async fn audit(&self, ctx: &PipelineCtx) -> Result<PipelineCtx> {
        auditor::run(&self.llm, ctx).await
    }
}

/// An external CLI agent that shells out to a binary.
pub struct ExternalCliAgent {
    name: String,
    command_args: Vec<String>,
    timeout: Duration,
}

#[async_trait]
impl PipelineAgent for ExternalCliAgent {
    // Delegates to run_external_agent() for all stages
    async fn observe(&self, ctx: &PipelineCtx) -> Result<PipelineCtx> {
        run_external_agent(&self.name, "observer", ctx).await
    }
    // ... etc
}
```

**Benefits**:
- Third agent (e.g., `anthropic`) can be added by implementing 4 functions
- The pipeline orchestrator becomes generic over `Box<dyn PipelineAgent>`
- Testing becomes easier — mock agents can implement the trait
- The dual pipeline structure is eliminated in favor of a single generic pipeline

**Migration Steps**:
1. Create `src/bacon_core/agent.rs` with the trait definition
2. Implement `LocalLlmAgent` for `bacon_agent_pi`
3. Implement `ExternalCliAgent` using existing `run_external_agent` logic
4. Refactor `bacon_agent_nvidia` to use `ExternalCliAgent` (it's already almost identical to pi, just with different model config)
5. Make the pipeline orchestrator generic over `Box<dyn PipelineAgent>`
6. Update existing tests
7. Run `cargo check && cargo test`

---

### 3.2 Implement Proper Rollback for Auto-Apply Failures ✅

**File(s)**:  
- `src/bacon_agent_pi/coder.rs` (function `auto_apply_queued_patch`)  
- `src/bacon_agent_nvidia/coder.rs` (equivalent)  

**Current State**:  
The `auto_apply_queued_patch()` function implements rollback as a single `git apply -R` if `check-fast.ps1` fails after applying. This is fragile:
- Does not support chained patches (apply patch A, then B, rollback both)
- Does not track the pre-apply state (could partially apply then fail to rollback)
- Does not check for merge conflicts during rollback
- Does not handle the case where `check-fast.ps1` passes but tests within it fail mid-way

**Solution**:  
Implement proper snapshotted rollback:

```rust
async fn auto_apply_queued_patch(patch: &QueuedPatch) -> Result<()> {
    // 1. Snapshot the current state
    let snapshot = git::create_snapshot()?;
    //    Saves: git stash of staged changes, records HEAD commit, 
    //           saves modified files' content
    
    // 2. Apply the patch
    git::apply(&patch.diff).context("Failed to apply patch")?;
    
    // 3. Validate
    let check_result = run_check_fast().await;
    
    if check_result.is_ok() {
        // Success — commit the snapshot as "applied"
        snapshot.mark_applied();
        archive_patch(patch);
        Ok(())
    } else {
        // Failure — restore the snapshot
        snapshot.restore()?;
        // restore() does: git apply -R + git checkout of saved files + 
        //                 git stash pop (if anything was stashed)
        //                 verify no leftover changes
        bail!("Patch failed validation, rolled back. Error: {}", check_result.err());
    }
}

struct GitSnapshot {
    stash_ref: Option<String>,
    head_commit: String,
    file_backups: Vec<(PathBuf, String)>,  // path → original content
}

impl GitSnapshot {
    fn create() -> Result<Self> {
        // Stash uncommitted changes if any
        let stash_ref = if has_uncommitted_changes()? {
            Some(git_exec(["stash", "push", "-m", "bacon-auto-apply-snapshot"])?)
        } else { None };
        
        // Record current HEAD
        let head_commit = git_exec(["rev-parse", "HEAD"])?;
        
        // Save contents of files we're about to modify (from the patch)
        let mut file_backups = Vec::new();
        for path in files_in_patch()? {
            let content = std::fs::read_to_string(&path)?;
            file_backups.push((path, content));
        }
        
        Ok(Self { stash_ref, head_commit, file_backups })
    }
    
    fn restore(&self) -> Result<()> {
        // Restore files from backup
        for (path, content) in &self.file_backups {
            std::fs::write(path, content)?;
        }
        
        // Restore stash if anything was stashed
        if let Some(ref) = &self.stash_ref {
            git_exec(["stash", "pop"])?;
        }
        
        // Verify we're back at the original HEAD
        let current_head = git_exec(["rev-parse", "HEAD"])?;
        ensure!(current_head.trim() == self.head_commit.trim(),
                "Rollback failed: HEAD mismatch (expected {}, got {})",
                self.head_commit, current_head);
        
        Ok(())
    }
}
```

**Migration Steps**:
1. Implement `GitSnapshot` struct with `create()` and `restore()` methods
2. Modify `auto_apply_queued_patch()` to use snapshot-based rollback
3. Add tests for rollback: apply a faulty patch, verify rollback restores original state
4. Add tests for snapshot: simulate crash during apply, verify snapshot can be restored externally
5. Run `cargo test`

---

### 3.3 Test All 4 Roles in Contract Tests ✅

**File(s)**:  
- `tests/bacon_cli_worker_contract.rs`  

**Current State**:  
The contract tests only test the `observer` role for each CLI worker. The `strategist`, `coder`, and `auditor` roles are untested for contract compliance.

**Solution**:  
Extend the contract tests to cover all four roles:

```rust
// Test template that tests a specific role
async fn test_worker_role(tool: &str, role: &str, expected_description_contains: &str) {
    let args = match tool {
        "kilocode" => vec!["run", "contract fixture", "--role", role, "--dry-run"],
        _ => vec!["-p", "contract fixture", "--role", role, "--dry-run"],
    };
    
    let output = run_worker(tool, &args);
    
    assert!(output.status.success(), "{} {} exited with error", tool, role);
    
    let worker_output: WorkerOutput = serde_json::from_str(&output.stdout)
        .expect("{} {} output is not valid JSON");
    
    assert_eq!(worker_output.status, "ok", 
        "{} {} status should be ok", tool, role);
    assert!(worker_output.description.contains(expected_description_contains),
        "{} {} should mention '{}'", tool, role, expected_description_contains);
    
    // Role-specific assertions
    match role {
        "strategist" => assert!(worker_output.spec_path.is_some(), 
            "strategist should provide a spec_path"),
        "coder" => assert!(worker_output.diff.is_some() || worker_output.status == "error",
            "coder should provide a diff or error"),
        _ => {}
    }
}

// Observer-specific: description should mention the role
fn test_observer_contract(tool: &str) {
    test_worker_role(tool, "observer", "Observer");
}

// Strategist-specific: should produce a spec_path
fn test_strategist_contract(tool: &str) {
    test_worker_role(tool, "strategist", "plan");
}

// Coder-specific: should produce a diff
fn test_coder_contract(tool: &str) {
    test_worker_role(tool, "coder", "patch");
}

// Auditor-specific: should start with PASS or FAIL
fn test_auditor_contract(tool: &str) {
    test_worker_role(tool, "auditor", "PASS");
}
```

**Migration Steps**:
1. Add `test_<role>_contract(tool)` functions for strategist, coder, auditor
2. For each CLI worker, run all 4 role tests
3. Ensure each worker's implementation handles all roles
4. Run `cargo test`

---

### 3.4 Add NVIDIA Agent to Contract Tests ✅

**File(s)**:  
- `tests/bacon_cli_worker_contract.rs` (new test)  
- `src/bin/nvidia.rs` (ensure it handles `--role` correctly)

**Current State**:  
The NVIDIA agent (`src/bin/nvidia.rs`) is the default pipeline agent in `bacon.toml`, but it has no dedicated contract tests. The existing contract tests cover `codex`, `gemini`, `opencode`, `kilocode`, and `ollama_external` — but not `nvidia`.

**Solution**:  
Add a contract test for the nvidia agent:

```rust
#[test]
fn nvidia_observer_contract() {
    test_worker_contract("nvidia", &["-p", "contract fixture", "--role", "observer", "--dry-run"]);
}

#[test]
fn nvidia_strategist_contract() {
    test_worker_role("nvidia", "strategist", "plan");
}

#[test]
fn nvidia_coder_contract() {
    test_worker_role("nvidia", "coder", "patch");
}

#[test]
fn nvidia_auditor_contract() {
    test_worker_role("nvidia", "auditor", "PASS");
}
```

**Prerequisites**:  
- Ensure `nvidia.rs` accepts `--role` argument (check: it likely does based on code search showing role-based system prompts)
- Ensure `nvidia.rs` outputs valid JSON in `--dry-run` mode
- Ensure test environment has NVIDIA API key configured (or skip test if not available)

**Migration Steps**:
1. Add 4 new test functions for nvidia agent
2. If nvidia requires an API key, add `#[ignore]` or conditional compilation
3. Run `cargo test` — verify nvidia tests pass or are properly skipped
4. Run with `NVIDIA_API_KEY` set to verify actual contract compliance

---

## Appendix: Dependency Graph

```
Phase 0 (Foundation)
├── 0.1 Shared Pipeline Core           ← No dependencies
├── 0.2 Section Parsing Fix            ← No dependencies
├── 0.3 E2E Tests                      ← Depends on 0.1 (shared core makes testing easier)
└── 0.4 NVIDIA Prompt Source Fix       ← No dependencies

Phase 1 (Correctness)
├── 1.1 Coder File Context             ← No strict dependencies, benefits from 0.1
├── 1.2 Auditor Diff Context           ← No strict dependencies
├── 1.3 Coder Refusal Handling         ← No strict dependencies
└── 1.4 Coder→Strategist Loop          ← Depends on 0.2 (reliable parsing needed for scope reduction)

Phase 2 (Observability)
├── 2.1 Config Cleanup                 ← No dependencies (keep keys from Phase 1)
├── 2.2 Standardize Confidence + Metrics ← No dependencies
└── 2.3 Spec Package Streamline        ← Depends on spec-lint.ps1 analysis

Phase 3 (Hardening)
├── 3.0 Verify spec_io Divergence      ← Prerequisite for 3.1
├── 3.1 PipelineAgent Trait            ← Depends on 0.1 (shared core) + 3.0
├── 3.2 Rollback Implementation        ← No dependencies
├── 3.3 Role Contract Tests            ← No dependencies, but benefits from 0.3
├── 3.4 NVIDIA Contract Tests          ← No dependencies
└── (Future) CI/CD Integration         ← After all phases complete
```

Items within the same phase can be done in parallel. Cross-phase items should follow the dependency graph.

---

## Appendix: Effort Estimation

> **Caveat**: Estimates assume a single developer with full context. Add 30–50% for context switching, code review, and unexpected edge cases. Two major refactors (0.1, 3.1) and one control-flow change (1.4) are the highest-risk items.

| Task | Est. Coding | Est. Testing | Est. Total | Risk Level |
|------|------------|-------------|------------|------------|
| 0.1 Shared Pipeline Core | 2 days | 1 day | 3 days | 🔴 High — refactoring core without breaking tests |
| 0.2 Section Parsing Fix | 2 hours | 2 hours | 4 hours | 🟢 Low — contained change |
| 0.3 E2E Tests | 1 day | 0.5 day | 1.5 days | 🟡 Medium — requires mock infrastructure |
| 0.4 NVIDIA Prompt Source Fix | 4 hours | 2 hours | 6 hours | 🟢 Low — swapping string constants for file reads |
| 1.1 Coder File Context | 1 day | 0.5 day | 1.5 days | 🟡 Medium — token limit considerations |
| 1.2 Auditor Diff Context | 0.5 day | 0.5 day | 1 day | 🟢 Low — adding info to existing prompt |
| 1.3 Coder Refusal Handling | 0.5 day | 0.5 day | 1 day | 🟢 Low — adding pattern check before diff extraction |
| 1.4 Coder→Strategist Loop | 1.5 days | 1 day | 2.5 days | 🔴 High — modifies pipeline control flow |
| 2.1 Config Cleanup | 4 hours | 2 hours | 6 hours | 🟢 Low — search + remove |
| 2.2 Standardize Confidence + Metrics | 4 hours | 4 hours | 8 hours | 🟢 Low — parsing + logging |
| 2.3 Spec Package Streamline | 4 hours | 6 hours | 10 hours | 🟡 Medium — changes filesystem contract, depends on spec-lint |
| 3.0 Verify spec_io Divergence | 5 min | — | 5 min | 🟢 Low — one diff command |
| 3.1 PipelineAgent Trait | 1.5 days | 1 day | 2.5 days | 🔴 High — architectural change |
| 3.2 Rollback Implementation | 1 day | 0.5 day | 1.5 days | 🟡 Medium — filesystem safety critical |
| 3.3 Role Contract Tests | 0.5 day | 0.5 day | 1 day | 🟢 Low — extending existing tests |
| 3.4 NVIDIA Contract Tests | 2 hours | 2 hours | 4 hours | 🟢 Low — adding new test functions |

**Total Estimated Effort**: ~18 days (3.5 work weeks for a single developer)

**Suggested Sprint Planning**: 
- Sprint 1: Phase 0 (all 4 items) + 2.1 (config cleanup) = ~6 days
- Sprint 2: Phase 1 (all 4 items) = ~6 days  
- Sprint 3: Phase 2 (remaining 2 items) + Phase 3 (all 5 items) = ~6 days
