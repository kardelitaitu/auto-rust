---
trigger: code-implementer
description: Agent skill when implementing spec files
---
# Code Implementer Skill

Use this skill when implementing an approved spec package in `docs/specs/_active/<initiative>/`.

## Goal

- Implement the spec safely.
- Keep the codebase reliable, scalable, and easy to use.
- Keep changes small and easy to review.

## Input

- Approved spec folder in `docs/specs/_active/<initiative>/`
- Existing codebase state
- Current `AGENTS.md` rules

## Workflow

1. Read `docs/specs/README.md`.
2. Read the active spec package.
3. Confirm scope and file ownership.
4. Make the smallest code, test, and doc changes needed.
5. Update `implementation-notes.md` as work lands.
6. Use `.\check-fast.ps1` during iteration.
7. Use `.\check.ps1` before handoff or push.
8. Move the spec folder to `_done/` only after full checks pass.

## Rules

- Use native tools `code_search` , `grep_search` , `find_by_name` for `Code Discovery & Search`
- Use native tool `todo_list` for `Task Management`
- Use native tools `run_command` , `command_status` , `read_terminal` for `Command Execution`
- Use native tools `read_file` , `list_dir` , `edit` , `multi_edit`for `File Operations`
- Do not change spec planning docs unless scope changes.
- Do not mix unrelated work into one spec.
- Keep `implementation-notes.md` append-only.
- Prefer targeted edits over broad rewrites.
- Keep explanations short and logical.
- If scope changes, update the spec before more code changes.

## Validation

- `.\check-fast.ps1` for scoped iteration.
- `.\check.ps1` for final verification.
- Add focused tests when the change affects behavior.

## Output

- Code changes
- Tests
- Docs updates
- `implementation-notes.md`
- Journal entry when required by the repo workflow
