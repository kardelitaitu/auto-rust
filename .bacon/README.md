# Bacon — Autonomous Coding Pipeline

*last audited 16-06-26 by opencode*

Bacon is a gated 4-agent LLM pipeline for autonomous code improvements. It turns prompts or compiler warnings into verified, spec-driven code changes.

## Quick Start

```bash
# Prerequisites
cargo build --release --bin bacon
export NVIDIA_API_KEY="nvapi-<your-key>"

# Basic usage (interactive — pauses for approval)
bacon -p "refactor error handling in src/utils/"

# Full automation (unattended)
bacon --auto -p "fix all clippy warnings"

# Fast path for trivial fixes (skips Strategist + Auditor)
bacon --fast -p "remove unused imports"
```

## Pipeline Flow

```
Prompt → Observer → Strategist → Coder → Auditor → Done
         ╰─────────── gates ───────────╯
```

Each stage validates the previous. The spec filesystem (`docs/specs/_active/`, `_done/`) is the source of truth.

- **Observer**: Scans codebase or reads approved specs
- **Strategist**: Creates spec package (`spec.yaml`, `plan.md`, `validation.md`)
- **Coder**: Implements changes, validated by `check-fast.ps1` (up to 4 retries)
- **Auditor**: Validates patch against spec criteria

### Detailed Stage Flow

```
run()
├── 1. Parse CLI args → RunArgs
├── 2. Load bacon.toml → PipelineConfig
├── 3. Create Llm client
├── 4. Validate bacon_local_only()
├── 5. Create initial PipelineCtx { prompt, .. }
│
├── 6. Observer
│     ├── Check for approved specs (fast-path)
│     ├── If found → use existing spec, skip LLM
│     ├── If not → call LLM with role prompt
│     ├── Extract confidence
│     └── Return updated ctx { prompt, confidence, .. }
│
├── 7. Strategist
│     ├── Call LLM with role prompt + observer output
│     ├── Extract sections (plan, scope, acceptance criteria)
│     ├── Write spec package (spec.yaml, plan.md, validation.md)
│     ├── Run spec-lint.ps1 gate
│     └── Return ctx { plan, spec_path, .. }
│
├── 8. Coder (the most complex stage)
│     ├── Read spec package files
│     ├── Collect source context (up to 8 files, 100 lines each)
│     ├── GitSnapshot (save pre-apply state)
│     │
│     ├── RETRY LOOP (up to 4 attempts)
│     │   ├── Call LLM with spec + error feedback from previous attempt
│     │   ├── Parse response for SEARCH/REPLACE blocks
│     │   ├── Apply patches to working tree with GitSnapshot rollback
│     │   ├── Run check-fast.ps1 gate
│     │   ├── On refusal (≥2 consecutive) → abort pipeline
│     │   ├── On repeated error → skip remaining retries
│     │   ├── On success → save patch file, break
│     │   └── On failure → feedback to next attempt
│     │
│     ├── If all retries exhausted → write failure report
│     ├──   → mark needs-human-approval
│     │
│     ├── If success:
│     │   ├── Show diff to user (confirmation gate)
│     │   ├── auto-apply or queue patch
│     │   └── GitSnapshot.mark_applied()
│     │
│     └── Return ctx { patch_path, coder_refused, .. }
│
├── 9. If coder_refused → skip Auditor, abort
│
├── 10. Auditor
│      ├── Read spec.yaml + patch file (NOT git diff)
│      ├── Call LLM with spec criteria + actual diff
│      ├── Parse decision: PASS / FAIL
│      │   (uses regex `^(?i:PASS)\b` to avoid false positives)
│      ├── If PASS: run spec-lint, archive to _done/
│      ├── If FAIL: prepend audit report to validation.md
│      └──   → mark needs-human-approval
│
└── 11. Shutdown / cleanup
```

## Key Commands

| Command | Purpose |
|---------|---------|
| `bacon -p "..."` | Guided pipeline (gates after Strategist & Coder) |
| `bacon --auto -p "..."` | Unattended (skips all gates) |
| `bacon --fast -p "..."` | Skip Strategist + Auditor |
| `bacon --dry-run -p "..."` | Sandbox mode (no writes) |
| `bacon --stage coder --spec 55` | Resume from specific stage |
| `bacon test` | Run pipeline test harness |

## Configuration

Single file: `.bacon/bacon.toml`

```toml
[pipeline]
observer = "nvidia_observer"
strategist = "nvidia_strategist"
coder = "nvidia_coder"
auditor = "nvidia_auditor"
```

Each stage uses a dedicated agent entry in `[agents.<name>]` with its own temperature, max_tokens, and timeout settings. The NVIDIA model defaults to `meta/llama-3.3-70b-instruct` and can be overridden with `NVIDIA_MODEL` or per-agent `[agents.<name>].model`.

See `.bacon/workflow.md` for full configuration reference and error recovery procedures.

## Project Structure

```
.bacon/
├── bacon.toml           # Pipeline & agent configuration
├── workflow.md          # Technical reference (this doc)
├── README.md            # Quick start (this file)
├── roles/               # LLM role prompts (4 files)
└── scripts/             # PowerShell utilities

docs/
├── specs/               # Spec packages
│   ├── _active/         # In-progress specs
│   ├── _done/           # Completed specs
│   └── _template/       # Spec template (3 files)
├── _archive/            # Archived historical documents
└── ...                  # Other project docs

crates/
├── bacon-pipeline/      # Shared pipeline types, traits & agent implementations
└── ...                  # Future workspace crates

src/
├── bin/                 # Binary entry points
└── ...                  # Application modules
```

## Validation

| Check | Command |
|-------|---------|
| Quick | `check-fast.ps1` (cargo check, clippy, fmt) |
| Full | `check.ps1` (slow + integration tests) |
| Specs | `spec-lint.ps1` |
| Tests | `cargo nextest run` (3,510+ tests) |

For detailed pipeline operations, configuration, error recovery, and CLI worker contracts, see [workflow.md](workflow.md).
