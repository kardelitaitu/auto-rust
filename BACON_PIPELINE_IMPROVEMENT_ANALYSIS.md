# Bacon Pipeline — Improvement & Repair Analysis

**Date:** 2026-05-14  
**Branch:** `bacon-dev`  
**Analyst:** Buffy (Codebuff AI Agent)  
**Scope:** `src/bacon_core/`, `src/bacon_agent_nvidia/`, `src/bacon_agent_ollama/`, `.bacon/`, `docs/specs/`

---

## Overview

The Bacon pipeline is a gated 4-stage LLM coding pipeline (Observer → Strategist → Coder → Auditor). It is functional and has undergone multiple rounds of auditing (see `BACON_CORE_NVIDIA_AUDIT_VERIFIED.md`, `bacon-audit.md`, `DOTBACON-WORKFLOW-AUDIT.md`). This analysis consolidates existing findings and identifies additional areas for improvement based on a deep codebase review.

**Total pipeline source:** ~3,530 lines (bacon_core + bacon_agent_nvidia)  
**Existing audits:** 3 completed audits with ~54 findings identified, ~51 fixed  
**Remaining known issues:** 3 from bacon-audit.md + 9 from BACON_CORE_NVIDIA_AUDIT + 10 from DOTBACON-WORKFLOW-AUDIT

---

## 🔴 Priority 0 — Critical / Must Fix

P0.1 Dead Code: `run_reduce_scope()` Never Called
**Evidence:** `src/bacon_core/agent.rs` — the method `run_reduce_scope()` is defined in the `PipelineAgent` trait as an `async fn` with a default no-op implementation (returns `PipelineCtx` unchanged). It is never invoked anywhere in the codebase. This is a **confirmed HIGH** finding from `BACON_CORE_NVIDIA_AUDIT.md`.

**Impact:** This is a silent-footgun pattern — any trait implementer that doesn't override `run_reduce_scope()` gets a no-op, which would silently skip scope reduction if the method were ever called. The original design had a Coder→Strategist fallback loop for scope reduction, but the current implementation (Coder retry loop with `MAX_ATTEMPTS=4` and ultimate `needs-human-approval`) replaced it without removing the dead code.

**Repair:** Either:
- Remove `run_reduce_scope()` from the trait and all implementations (preferred — simplifies the trait contract)
- Or wire it into the Coder failure path as a genuine fallback before `needs-human-approval`

**Quick win?** ✅ ~15 minutes to remove

### P0.2 Untested Unified-Diff Fallback Code

**Evidence:** `src/bacon_agent_nvidia/coder.rs:58-92` (unified-diff regex) and `coder.rs:708-764` (unified-diff parsing path) — **87 lines of completely untested code** that implements an alternative diff format parsing path. Confirmed HIGH finding from the nvidia audit.

**Impact:** This code will silently fail at runtime if triggered, since it has zero test coverage and the regex is complex (`r"(?m)^diff --git a/(.+?) b/(.+?)\n(?:.*\n)*?^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@.*\n((?:(?!^diff )[^]?)"`). The primary SEARCH/REPLACE path is well-tested, making this dead weight.

**Repair:** Either:
- Remove the unified-diff fallback entirely (simplest, since SEARCH/REPLACE is the primary path)
- Or add proper unit tests and wire it into the fallback chain

### P0.3 Dual `needs-human-approval` Representations

**Evidence:** `src/bacon_core/mod.rs:355` defines `needs_human_approval: bool` as a struct field, while `src/bacon_agent_nvidia/coder.rs:632` and `auditor.rs:165` use the status string `"needs-human-approval"` in `spec.yaml`. Confirmed MEDIUM finding from the nvidia audit.

**Impact:** Two different mechanisms track the same state — one in-memory boolean and one filesystem string. They can drift apart. The Coder sets the string status but may not update the boolean, potentially causing inconsistent behavior.

**Repair:** Unify into a single source of truth. Either remove the boolean field and read from `spec.yaml` exclusively, or make the boolean authoritative and write it to `spec.yaml`.

---

## 🟠 Priority 1 — High Impact

### P1.1 Doc-Code Drift (10 Confirmed Inconsistencies)

**Evidence:** `DOTBACON-WORKFLOW-AUDIT.md` identified 10 inconsistencies between `.bacon/workflow.md` and the actual implementation:

| # | Severity | Finding |
|---|----------|---------|
| H1 | HIGH | Doc says `bacon-coder` prompt asks for unified-diff format; code uses SEARCH/REPLACE |
| H2 | HIGH | Doc claims `check-fast.ps1` runs `nextest` + `spec-lint`; code only runs `cargo check`, `clippy`, `fmt` |
| M1 | MEDIUM | Doc mentions `spec life-cycle` status "in-progress" but this status is handled implicitly |
| M2 | MEDIUM | Doc references `sessions/security_audit.log` but the file is created only when explicitly enabled |
| M3 | MEDIUM | Doc describes Coder→Strategist fallback loop; code uses internal retry loop (no Strategist fallback) |
| M4 | MEDIUM | Doc describes spec archive flow differently from actual `promote_to_done()` |
| L1 | LOW | Doc mentions audit PASS/FAIL status markers; code uses dual representations |
| L2 | LOW | Doc references utility scripts that may not be wired into the active workflow |
| L3 | LOW | Doc section references may be stale after refactoring |
| N1 | NEW | Doc doesn't mention the `--parallel` flag or `stage_delay_ms` config which were recently added |

**Repair:** Update `.bacon/workflow.md` to match actual implementation. The docs-auditor skill can help with this systematically.

### P1.2 Orphaned `_created_at` Field in GitSnapshot

**Evidence:** `src/bacon_core/git_snapshot.rs:27` defines `_created_at: Instant` which is written but never read. Confirmed MEDIUM finding.

**Impact:** Unnecessary field adds noise. If the field is intended for future use (e.g., snapshot cleanup/TTL), it should be documented and used.

**Repair:** Either remove the field or use it to implement snapshot TTL-based cleanup.

### P1.3 Phantom File Reads (internal-api-outline.md + implementation-notes.md)

**Evidence:** `src/bacon_agent_nvidia/strategist.rs` attempts to read `internal-api-outline.md` and `implementation-notes.md` from spec directories, but the streamlined spec package (since Phase 2) only uses 3 files: `spec.yaml`, `plan.md`, `validation.md`. Confirmed MEDIUM findings.

**Impact:** Silent warnings or error paths that serve no purpose. The `spec-archive.ps1` was also recently updated to make `README.md` optional, confirming the 3-file standard.

**Repair:** Remove the phantom file reads from the strategist and any other stage that references them.

---

## 🟡 Priority 2 — Medium Impact

### P2.1 Hardcoded Spec Metadata

**Evidence:** `src/bacon_agent_nvidia/strategist.rs:177-178`:
```rust
owner: "pipeline".to_string(),
implementer: "pipeline".to_string(),
// Also: priority: "P2", area: ["bacon"]
```
All generated specs get identical metadata regardless of actual scope. Confirmed LOW finding from nvidia audit.

**Impact:** If any downstream tooling depends on priority or area to triage specs, all specs look identical. Prevents any kind of spec triage or filtering.

**Repair:** Either:
- Have the LLM generate these fields dynamically
- Or accept the defaults and document that they are always "P2/bacon"

### P2.2 Missing Unit Tests in Key Orchestration Modules

**Evidence:** 
- `src/bacon_agent_nvidia/pipeline.rs` (119 lines) — no unit tests
- `src/bacon_agent_nvidia/observer.rs` (75 lines) — no unit tests  
- `src/bacon_agent_nvidia/auditor.rs` (175 lines) — no unit tests
- `src/bacon_agent_nvidia/coder.rs` (827 lines) — minimal unit tests
- `src/bacon_core/agent.rs` (258 lines) — no unit tests for the trait default `run()` method

**Impact:** The 4 most critical pipeline orchestration files have near-zero unit test coverage. The integration tests in `tests/bacon_pipeline_integration.rs` cover happy paths but not edge cases.

**Repair:** Add unit tests for:
- Pipeline stage transitions and error handling
- Observer spec fast-path logic
- Auditor PASS/FAIL decision handling
- Coder SEARCH/REPLACE parsing edge cases

### P2.3 `unused_imports` + `dead_code` Suppressions

**Evidence:** 131+ `#[allow(dead_code)]` and `#[allow(unused_imports)]` annotations across the codebase. While not all are in bacon-specific code, several are:

- `src/bacon_core/agent.rs` — some trait methods may only be used by specific implementations
- `src/bacon_agent_nvidia/` — module structure may have dead exports

**Impact:** Masks real dead code and makes it harder to identify genuinely unused functionality. The compiler is being silenced rather than the code being cleaned up.

**Repair:** Audit and remove dead code rather than suppressing warnings. Enable `#![deny(dead_code)]` in CI.

### P2.4 Stale Ollama Agent

**Evidence:** `src/bacon_agent_ollama/` has only 2 files (92 lines total) and a simple `run()` function. The ollama agent was part of an older architecture and may not be compatible with the current `PipelineAgent` trait.

**Impact:** Untested integration point. If someone configures `ollama` as their agent in `bacon.toml`, it may not work correctly with the current pipeline flow.

**Repair:** Either:
- Remove the ollama agent if it's no longer used
- Or update it to implement the `PipelineAgent` trait properly
- Or add a warning/error when an unsupported agent is configured

### P2.5 Self-Healing System Not Wired Into Bacon Pipeline

**Evidence:** `src/adaptive/self_healing/` contains ~1,778 lines across 6 files (`system.rs`, `strategy.rs`, `state.rs`, `health.rs`, `history.rs`, `mod.rs`) with extensive recent unit test additions. However, the bacon pipeline (`agent.rs`, `pipeline.rs`, `coder.rs`, `auditor.rs`) does not reference or import any self-healing components.

**Impact:** ~1,778 lines of dead code with zero integration into the pipeline it was presumably designed for. The self-healing system has no effect on pipeline behavior — failures, retries, and recovery are handled ad-hoc within each stage rather than through a centralized self-healing framework.

**Repair:** Either:
- Wire the self-healing system into the pipeline (e.g., wrap stage execution with health checks, auto-recovery on failure patterns)
- Or clearly document that this is a standalone/experimental module not yet integrated
- Or remove it if the concept has been abandoned

### P2.6 `spec-archive.ps1` vs. `promote_to_done()` Dual Path

**Evidence:** The pipeline's `auditor.rs` has a `promote_to_done()` function that handles archiving completed specs (spec-lint, status update, move to `_done/`). Independently, `spec-archive.ps1` is a standalone PowerShell script that also handles archiving but was recently modified to support optional README.md and accept `"implemented"` status.

**Impact:** Two separate archiving paths with potentially different criteria and behavior. The PowerShell script and the Rust code may handle edge cases differently (e.g., the script allows `"implemented"` status; the Rust code may have its own criteria). This creates a risk where manual archiving via the script produces a different result than pipeline-driven archiving.

**Repair:** Either:
- Make `spec-archive.ps1` a thin wrapper that calls the Rust `promote_to_done()` logic
- Or consolidate archiving rules into a single source of truth and derive both paths from it

### P2.7 `bacon-test.rs` Binary May Be Stale

**Evidence:** `src/bin/bacon-test.rs` is a separate test harness binary. Its `#[allow(dead_code)]` annotation at line 21 suggests some of its code paths may be unused. The binary was not audited in any of the existing audits.

**Impact:** If this binary is out of sync with the current pipeline API, it could mislead developers about expected behavior or simply be dead weight.

**Repair:** Audit `src/bin/bacon-test.rs` against the current `PipelineAgent` trait and pipeline flow. Remove or update any stale test harness code.

---

## 🔵 Priority 3 — Low Impact / Quality of Life

### P3.1 `unwrap()`/`expect()` Calls in Production Paths

**Evidence:** 17 `unwrap()`/`expect()` calls in non-test bacon code:
- `src/bacon_core/mod.rs:1008,1057,1144` — Regex::new() unwraps (safe with static regexes, but fragile if regex changes)
- `src/bacon_core/mod.rs:1431,1443,1467,1479,1485,1487,1498` — Test helpers mixed with production code
- `src/bacon_core/cli_types.rs:90,96,98` — Test assertions that would panic
- `src/bacon_core/spec_io.rs:344,345` — Serialization expects
- `src/bacon_agent_nvidia/coder.rs:62` — Regex expect
- `src/bacon_agent_nvidia/nvidia_api.rs:259,299` — JSON serialize expects

**Impact:** Most are in safe contexts (test code, static regexes, serialization that shouldn't fail), but the test-helpers-in-production-code pattern in `mod.rs` is concerning — if these ever run outside tests, they panic.

**Repair:** Move test-only helpers into `#[cfg(test)]` blocks and convert production-path unwraps to proper error propagation with `anyhow::Context`.

### P3.2 Spec-Lint as a Gate vs. Advisory

**Evidence:** `spec-lint.ps1` is run as a validation gate after the Strategist and before the Coder. The `spec-archive.ps1` and `promote_to_done` also invoke it. However, the linting rules are not clearly documented.

**Impact:** A failing spec-lint blocks the pipeline, but it's not always clear what the lint checks or how to fix failures.

**Repair:** Add inline `Write-Host` comments per validation check in `spec-lint.ps1` so a failing user immediately sees *what* failed and *why*. Document the lint rules in the script header.

**Quick win?** ✅ ~20 minutes to add inline comments

### P3.3 Response Body Dropped on JSON Parse Failure

**Evidence:** `src/bacon_agent_nvidia/nvidia_api.rs:132-141` — When the LLM API returns a response with an unexpected JSON shape, the raw body is logged but not preserved for debugging. Confirmed MEDIUM finding from nvidia audit.

**Impact:** Debugging API integration issues requires reproducing the exact failure, which is time-consuming.

**Repair:** Save raw response bodies on parse failure (e.g., to `sessions/api_errors/`).

### P3.4 Confidence Extraction Fragility

**Evidence:** `src/bacon_core/mod.rs` uses regex-based `extract_confidence()` which looks for patterns like `Confidence: High` in LLM output. The regex is tightly coupled to the LLM's output format.

**Impact:** If the LLM changes its output format or is prompted differently, confidence extraction silently returns default (likely `Low`), potentially blocking the pipeline unnecessarily.

**Repair:** Add acceptance tests for the confidence extraction regex with sample LLM outputs. Consider structured output (e.g., JSON wrapper) instead of regex parsing.

### P3.5 `spec-lint.ps1` and `check-fast.ps1` Not Hardened

**Evidence:** Both scripts are PowerShell-based and invoked via `std::process::Command`. They assume PowerShell is available and the scripts are in the expected location.

**Impact:** On Windows without PowerShell (unlikely but possible) or in non-standard setups, these gates silently fail or produce confusing errors.

**Repair:** Add pre-flight checks for PowerShell availability and script existence before invoking. Provide clear error messages.

---

## 🚀 Quick Wins (Can Be Fixed in <30 Minutes Each)

| Item | Description | Est. Time |
|------|-------------|-----------|
| P0.1 | Remove dead `run_reduce_scope()` from trait | ~15 min |
| P1.2 | Remove orphaned `_created_at` field | ~5 min |
| P1.3 | Remove phantom file reads (`internal-api-outline.md`, `implementation-notes.md`) | ~10 min |
| P2.1 | Document hardcoded spec metadata as intentional | ~10 min |
| P2.4 | Remove or mark ollama agent as deprecated | ~15 min |
| P3.2 | Add inline `Write-Host` comments to `spec-lint.ps1` | ~20 min |
| P3.3 | Save raw API response bodies on parse failure | ~25 min |

---

## 🔮 Additional Areas to Investigate

These came up during review but require deeper investigation to confirm:

### A.1 Self-Healing Module: Is It Ever Used?
`src/adaptive/self_healing/` has ~1,778 lines with thorough unit tests but no apparent integration into the bacon pipeline or the main orchestrator. Check if it's imported anywhere outside its own module.

### A.2 `bacon-test.rs` Binary Currency
`src/bin/bacon-test.rs` has `#[allow(dead_code)]` — review whether it tests the current pipeline API or a legacy interface.

### A.3 Pipeline Metrics Feed-Through
The pipeline has `sessions/metrics.json` and `sessions/errors.json`, but are stages consistently reporting to them? An observability audit would confirm whether all 4 stages emit metrics.

### A.4 Concurrent Spec Execution
The CLI has a `--parallel` flag, but its implementation and test coverage are unclear from the audit documents. Verify whether parallel execution of multiple specs actually works end-to-end.

---

## 📊 Summary of Repair Recommendations

| Priority | Count | Key Actions |
|----------|-------|-------------|
| **P0 — Critical** | 3 | Remove dead code, remove untested fallback, unify needs-human-approval |
| **P1 — High** | 3 | Fix doc-code drift, remove orphaned field, remove phantom file reads |
| **P2 — Medium** | 7 | Fix hardcoded metadata, add unit tests, remove dead_code suppressions, fix ollama agent, wire self-healing, consolidate archive paths, audit test binary |
| **P3 — Low** | 5 | Fix unwraps, document spec-lint, preserve API error bodies, harden confidence parsing, validate scripts |

**Total items: 18 improvement/repair opportunities** (plus 4 additional areas to investigate)

---

## 🔗 Related Documents

- `BACON_CORE_NVIDIA_AUDIT.md` — Source audit of bacon_core + nvidia agent (16 files, 9 findings)
- `BACON_CORE_NVIDIA_AUDIT_VERIFIED.md` — Verified sign-off on all 9 findings
- `bacon-audit.md` — Workflow audit (45 issues, 42 fixed, 3 remaining)
- `DOTBACON-WORKFLOW-AUDIT.md` — Doc-code drift analysis (10 inconsistencies)
- `docs/_archive/BACON_IMPROVEMENT_ROADMAP.md` — Original roadmap (Phases 0-3, all complete)
- `TODO.md` — Test coverage improvement plan
