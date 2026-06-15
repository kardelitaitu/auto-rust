# Spec Workspace

last audited 16-06-26 by opencode

This is the contract between spec planning and implementation.

## Contract

- One initiative per folder.
- Spec agent writes the spec package only.
- Implementer edits code, tests, and docs only after approval.
- `spec-lint.ps1` is system-owned, read-only, and regular feature specs must not target it.
- Before a risky handoff, checkpoint the worktree with `.\spec-stash.ps1` and keep the ref.
- If a handoff breaks the tree, restore with `.\spec-restore.ps1`.
- Spec packages are streamlined to 3 files: `spec.yaml`, `plan.md`, `validation.md`. The strategist generates these automatically.
- Old boilerplate files (`implementation-notes.md`, `internal-api-outline.md`, `ci-commands.md`, `quality-rules.md`, `baseline.md`, `notes.md`, etc.) are no longer generated.

## Lifecycle

| Status | Folder |
|---|---|
| `draft` | `_template/` working copy |
| `approved` / `in-progress` / `implemented` | `_active/<initiative>/` |
| `needs-human-approval` | `_active/<initiative>/` (stuck, needs review) |
| `done` | `_done/<initiative>/` |

## Enforcement

- `_active/` may contain `approved`, `in-progress`, `implemented`, or `needs-human-approval` specs.
- `_done/` may contain only `done` specs.
- `_done/` folders use sequential numeric prefixes (`0001-`, `0002-`, … `0007-`). When archiving a spec, increment to the next number (`0008-`, `0009-`, …).
- `spec-lint.ps1` prints package-level fix hints and can target one package with `-Directory`.
- Use `.\check-fast.ps1` during implementation.
- Move a spec to `_done/` only after `.\check.ps1` passes.
- Run `spec-lint.ps1` before handoff.
- Keep stash checkpoints named so a smaller agent can recover them without guessing.

## Archive Workflow

When a spec package is complete and ready for archival:

1. **Use the archive helper**: `.\spec-archive.ps1 <package-name>`
2. **The archive helper will**:
   - Run `spec-lint.ps1` as a pre-flight gate
   - Confirm the package is in an archiveable state (`approved` or `implemented`)
   - Update `spec.yaml` status to `done`
   - Normalize the implementer field to `archived-*` convention
   - Move the folder from `_active/` to `_done/`
3. **Status synchronization**: `spec.yaml` status must be `done` after archival
4. **Validation**: Run `spec-lint.ps1` after archival to ensure no status mismatches

**Note**: The archive helper is the normal handoff step. Do not manually move packages without using the archive helper, as this can cause status field mismatches that will fail linting.

## Recovery Workflow

To protect the worktree during handoffs between agents:

1. **Checkpoint**: Before handing a package to another agent, create a worktree snapshot:
   ```powershell
   .\spec-stash.ps1 "my-checkpoint-name"
   ```
   This creates a git stash named `spec-checkpoint: my-checkpoint-name` that includes untracked files.

2. **Record**: Note the stash reference (e.g., `stash@{0}`) printed by the script.

3. **Restore**: If an agent breaks the worktree or makes undesirable changes, restore from the checkpoint:
   ```powershell
   # Restore the latest checkpoint
   .\spec-restore.ps1

   # Restore a specific checkpoint ref
   .\spec-restore.ps1 "stash@{1}"
   ```
   The restore script uses `git stash apply`, meaning the checkpoint remains in your stash list until explicitly dropped.
