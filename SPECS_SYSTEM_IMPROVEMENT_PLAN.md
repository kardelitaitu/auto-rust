# Specs System Improvement Plan

This document outlines the roadmap for evolving the **Specs-Driven Development (SDD)** system in the `auto-rust` project. The goal is to reduce administrative friction, improve architectural traceability, and ensure the system scales with the codebase.

## 1. Identified Weaknesses

- **High Friction:** Mandatory 9+ files for every initiative regardless of complexity.
- **Path Fragility:** Full bucket-relative paths in `spec.yaml` make migrations to `_done/` tedious and error-prone.
- **Context Rot:** Historical specs can become misleading as the codebase evolves without a "superseded" mechanism.
- **Disconnected Traceability:** Hard to map specific lines of code back to their originating specs.
- **Mechanical Linting:** Current linting verifies presence but not semantic alignment.

---

## 2. Proposed Improvements

### Phase 1: Friction Reduction & Tooling (Short Term)
- [ ] **Relative Pathing**
  - [ ] **Lint Update:** Modify `spec-lint.ps1` to accept `.` prefixed paths in `spec.yaml`.
  - [ ] **ID Uniqueness:** Update `spec-lint.ps1` to verify that `spec.yaml` IDs are unique across all buckets.
  - [ ] **Migration:** Batch-replace bucket paths (`docs/specs/_active/`, `docs/specs/_done/`) with `./` in all `spec.yaml` and `README.md` files.
  - [ ] **Full-Text Scrub:** Search for any remaining bucket-relative strings in spec docs and convert to relative.
  - [ ] **Template Sync:** Update `docs/specs/_template/spec.yaml` to use relative paths.
- [ ] **Automation Tooling (`.\spec-finish.ps1`)**
  - [ ] **Scaffold:** Create script with `[Parameter(Mandatory=$true)][string]$Id`.
  - [ ] **Status Logic:** Regex replace `Status: `implementing`` with `Status: `done`` in `README.md`.
  - [ ] **YAML Logic:** Regex replace `status: implementing` with `status: done` in `spec.yaml`.
  - [ ] **Relocation:** Use `Move-Item` to shift folder from `_active/` to `_done/`.
  - [ ] **Post-Flight:** Automatically run `.\spec-lint.ps1` and `.\spec-index.ps1` to ensure consistency and refresh the global index.
- [ ] **Micro-Spec Template**
  - [ ] **Directory:** Create `docs/specs/_template_micro/`.
  - [ ] **Files:** Add `spec.yaml`, `README.md`, and `implementation-notes.md`.
  - [ ] **Lint Logic:** Add a `$MicroFiles` array to `spec-lint.ps1`; bypass full checks if `spec.yaml` area includes `micro`.

### Phase 2: Traceability & Context (Medium Term)
- [ ] **Source Linking**
  - [ ] **Standards:** Standardize `// @spec <id>` for `.rs` and `[Spec: <id>](path)` for `.md`.
  - [ ] **Manual Update:** Add tags to core modules (`orchestrator.rs`, `browser.rs`) for recent specs.
- [ ] **Spec Indexing (`.\spec-index.ps1`)**
  - [ ] **Parser:** Extract `id`, `title`, and `files.code` list from all `_done/**/spec.yaml`.
  - [ ] **Renderer:** Generate `docs/specs/INDEX.md` with "Code-to-Spec Map" section.
  - [ ] **Sort:** Group by source file path (alphabetical).
- [ ] **Supersession Logic**
  - [ ] **Schema:** Add `supersedes: [id]` as an optional list in `spec.yaml`.
  - [ ] **UI:** Update `INDEX.md` generator to strikethrough or label superseded IDs.

### Phase 3: Semantic & Workflow Evolution (Long Term)
- [ ] **Baseline "Spiking" Policy**
  - [ ] **AGENTS.md:** Add "Research Phase" section allowing code modifications for baseline proofing.
  - [ ] **Cleanup Rules:** Mandatory `git checkout .` before transitioning from Spec to Implementer role.
- [ ] **Semantic Linting**
  - [ ] **LLM Integration:** Create a specialized sub-agent `spec-validator` to compare `plan.md` against `spec.yaml` acceptance criteria.

### Phase 4: Future-Proofing & Resilience (Long Term)
- [ ] **Schema Versioning**
  - [ ] Update `spec-lint.ps1` to handle multiple `version` schemas (v1 vs v2).
- [ ] **Native Tooling Migration**
  - [ ] Build a native Rust CLI tool (`cargo spec`) to replace PowerShell scripts.
  - [ ] Implement proper YAML parsing and cross-platform path handling.
- [ ] **Invariant Monitoring**
  - [ ] Create a "Watcher" tool to flag PRs that violate `quality-rules.md` in relevant `_done/` specs.
- [ ] **Semantic Archeology**
  - [ ] Build `git-spec` to automate line-to-spec mapping via `git blame`.
- [ ] **Ecosystem Portability**
  - [ ] Maintain a "Spec Ontology" document for AI model interoperability.

---

## 3. Implementation Log

| Date | Improvement | Status |
|------|-------------|--------|
| 2026-05-06 | Initial Improvement Plan Created | ✅ Done |

---
*Created by Gemini CLI on May 6, 2026*
