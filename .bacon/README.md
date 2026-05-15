# Bacon — Autonomous Coding Pipeline

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
observer = "nvidia"
strategist = "nvidia"
coder = "nvidia"
auditor = "nvidia"
```

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

src/
├── bacon_core/          # Shared pipeline types & traits
├── bacon_agent_nvidia/  # NVIDIA pipeline implementation
├── bin/                 # Binary entry points
└── ...                  # Other modules
```

## Validation

| Check | Command |
|-------|---------|
| Quick | `check-fast.ps1` (cargo check, clippy, fmt, nextest, spec-lint) |
| Full | `check.ps1` (slow + integration tests) |
| Specs | `spec-lint.ps1` |
| Tests | `cargo nextest run` (2,949+ tests) |

For detailed pipeline operations, configuration, error recovery, and CLI worker contracts, see [workflow.md](workflow.md).
