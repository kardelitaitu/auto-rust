# rust-orchestrator operating notes
*Last updated: June 16, 2026*

AGENTS.md is the router. Keep it short, stable, and direct readers to the right doc when a topic needs more depth.

## Project state

All **15 roadmap items (Phases 0–3)** are complete. The Bacon gated-LLM pipeline is production-ready:

- **Shared core** (`crates/bacon-pipeline/`) — canonical types (`Stage`, `PipelineConfig`, `PipelineCtx`, `WorkerOutput`), `PipelineAgent` trait, `GitSnapshot` rollback, `spec_io` module
- **Single agent pipeline** (`nvidia`) — implements `PipelineAgent` for all 4 roles
- **Spec-lint** ensures spec quality; `check-fast.ps1`/`check.ps1` verify code changes
- **All 4 roles** (Observer, Strategist, Coder, Auditor) tested in contract tests
- **Spec packages streamlined** to 3 files (`spec.yaml`, `plan.md`, `validation.md`)
- **Coder retry loop** — internal MAX_ATTEMPTS=4 with error feedback, then writes failure report to `validation.md` and marks `needs-human-approval` (no scope-reduction fallback to Strategist)
- **Confidence scoring** standardized across all agents with metrics tracking

See [docs/_archive/BACON_IMPROVEMENT_ROADMAP.md](docs/_archive/BACON_IMPROVEMENT_ROADMAP.md) for the full itemized completion status.

## Read first

| If you are changing... | Read first |
|---|---|
| `TaskContext`, `api.*` verbs, or the public API surface | [docs/TASKS/task-context.md](docs/TASKS/task-context.md) |
| runtime flow, browser/session lifecycle, orchestrator behavior | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) |
| built-in tasks, task syntax, or task-specific behavior | [docs/TASKS/overview.md](docs/TASKS/overview.md), then the matching task doc in `docs/TASKS/` |
| selector rules and selector-specific guidance | [docs/TASKS/selectors.md](docs/TASKS/selectors.md) |
| DSL task execution or validation | [docs/TASKS/dsl.md](docs/TASKS/dsl.md) |
| specs, handoff rules, or checkpoint/restore flow | [docs/specs/README.md](docs/specs/README.md) |
| overall repo orientation | [README.md](README.md) |
| Bacon pipeline usage | [.bacon/README.md](.bacon/README.md), [.bacon/workflow.md](.bacon/workflow.md) |
| Skills to load (`skill` tool) | first use `skill: "onboarding-guide"` for the router, or browse `.agents/skills/` directly |

## Operating rules

- Use this file for the default rules only.
- Put domain detail in the linked doc, not here.
- If docs disagree, follow the most specific doc first:
  - task doc
  - runtime/API doc
  - architecture doc
  - README
  - AGENTS.md
- Keep changes minimal and reliable.
- Prefer the smallest doc set that fully explains the work.

## Tooling rules

### Filesystem MCP

Use for local repo file work.

- Read files with `read_text_file` or `read_multiple_files`.
- Discover files with `search_files` or `list_directory`.
- Use absolute paths like `C:\My Script\auto-rust\...`.

### Context-mode MCP

Use for commands that produce output, or when indexing docs.

- Prefer `ctx_execute`, `ctx_execute_file`, `ctx_index`, `ctx_search`, and `ctx_batch_execute`.
- Force repo cwd in commands: `cd "C:\My Script\auto-rust" && ...`.
- Use `ctx_batch_execute` for multi-step command work.
- Use `ctx_search` with a few technical terms, not long prose.

## Work modes

### Spec agent

- Write the spec package from `docs/specs/_template/` before code changes.
- Own planning docs only: `spec.yaml`, `plan.md`, `validation.md`.
- The strategist generates these 3 files automatically; handwritten specs should match.
- Keep specs short, measurable, and easy to review.
- `spec-lint.ps1` is system-owned. Only touch it for spec-system or tooling work.
- For manual handoffs between agents, use `.\spec-stash.ps1` to checkpoint the worktree and `.\spec-restore.ps1` to restore.

### Implementer agent

- Edit code, tests, docs updates, and `implementation-notes.md` after spec approval.
- Use `.\check-fast.ps1` for scoped iteration.
- After the Coder stage completes and the Auditor returns PASS, the pipeline moves the spec folder to `_done/`. For manual workflows, run `spec-lint.ps1` and `.\check-fast.ps1` before archiving.
- Update the spec first if scope changes.
- Do not edit `spec-lint.ps1` unless the task explicitly targets the spec system.
- If a handoff breaks the worktree, restore from a named checkpoint with `.\spec-restore.ps1`.

## Change workflow

1. Baseline the repo state before changing behavior.
2. Diagnose scope and impact first.
3. Present findings when the change is risky or ambiguous.
4. Implement the smallest safe change.
5. Verify with the relevant checks.
6. Run `cargo check` before committing. Run `.\check.ps1` before pushing.
7. Close out with a short summary and, when useful, `JOURNAL.md` updates.

## Response style

- Keep explanations short and logical.
- When you propose a fix, include 3-5 best next moves.
- When comparing options, include at least one alternative plus a simple pros/cons view.
- Optimize for reliable, scalable, easy-to-use code.
