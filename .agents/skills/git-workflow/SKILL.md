# Git Workflow

Spec-driven development conventions — branch naming, commit format, spec lifecycle, checkpoint/restore, and CI verification.

---

## Branch Naming

Current branch pattern: `v0.2.32` (semantic versioning)

- All development happens on version branches
- Format: `v{major}.{minor}.{patch}`
- Remote tracked: `origin/v0.2.32`

---

## Commit Message Format

```
type: short summary (reason/impact)
```

### Allowed Types

| Type | When to Use | Example |
|---|---|---|
| `feat` | New feature or capability | `feat: add twitterquote task (reuse LLM reply flow)` |
| `fix` | Bug fix | `fix(session): handle rate limits in connector` |
| `docs` | Documentation changes | `docs: trim README TOC (faster first read)` |
| `chore` | Maintenance, deps, config | `chore: normalize formatting after test additions` |
| `test` | Test additions or fixes | `test: add integration test for graceful shutdown` |
| `refactor` | Code restructuring (no behavior change) | `refactor: extract common retry logic` |
| `perf` | Performance improvement | `perf: cache selector lookups in hot path` |
| `style` | Formatting, lints | `style: fix clippy warnings` |
| `ci` | CI config or scripts | `ci: add nightly miri to workflow` |
| `revert` | Revert a previous commit | `revert: undo session timeout change` |
| `build` | Build system, Cargo.toml | `build: update chromiumoxide to 0.7` |

### Scope (Optional)
Add a scope in parentheses after the type:
```
fix(session): handle rate limits in connector
feat(twitter): add quote tweet support
docs(readme): update installation instructions
```

### Breaking Changes
Use `!` before the colon:
```
feat!: change API signature
feat(api)!: remove deprecated methods
```

### Good Examples (from history)
```
feat: add --debug flag to enable debug-level logging at runtime
fix: resolve follow action warnings and test/cli compilation errors
docs: approve 0030-typing-realism spec
refactor: DRY refactoring — ErrorClassifier::classify() now calls is_rate_limit_error(self)
test: add 7 end-to-end integration tests in twitteractivity_retry.rs
```

### Anti-patterns (Avoid)
```
update          # Too vague — what was updated?
fix             # What was fixed?
changes         # What changes?
WIP             # Use fixup! or squash! instead
```

### Commit Hook Validation
- **Pre-commit**: `cargo fmt --check` on staged `.rs` files
- **Commit-msg**: Validates conventional commits format using `scripts/commit-msg`
- Allowed through without validation: merge commits, `fixup!`/`squash!`, `WIP`, empty draft messages

---

## Spec Workflow

```
1. Create spec directory in docs/specs/_active/<id>/
2. Write spec.yaml (metadata, acceptance criteria)
3. Write plan.md (implementation steps, scope)
4. Write validation.md (verification criteria)
5. Run spec-lint.ps1 to validate
6. Implement changes
7. Run check-fast.ps1 during iteration
8. Run check.ps1 before push
9. Move spec to docs/specs/_done/<id>/ (after all checks pass)
```

### Spec Package (3 files)
| File | Required | Purpose |
|---|---|---|
| `spec.yaml` | Yes | Metadata: id, title, status, implementer, acceptance criteria |
| `plan.md` | Yes | Implementation steps and scope |
| `validation.md` | Yes | Acceptance criteria and audit results |

### Valid Statuses by Bucket

| Bucket | Valid Statuses |
|---|---|
| `_template/` | `draft` only |
| `_active/` | `approved`, `in-progress`, `implemented`, `needs-human-approval` |
| `_done/` | `done` only |

### Spec Directory IDs
Examples from the project: `0030-typing-realism`, `0019-session-lifecycle-extraction`

---

## Checkpoint / Restore

Use these scripts for agent handoffs and safe checkpoints:

### spec-stash.ps1
```powershell
.\spec-stash.ps1 <checkpoint-name>
```
- Creates a named git stash with untracked files
- Timestamps the stash: `spec-checkpoint: <name> (YYYYMMDD-HHmmss)`
- Prints the restore command on success

### spec-restore.ps1
```powershell
.\spec-restore.ps1 [stash-ref]
```
- Default: restores `stash@{0}` (most recent)
- Lists available stashes if ref not found
- Leaves the stash entry intact (use `git stash drop` to remove)

### When to Use
- Before agent handoff (save worktree state)
- Before risky refactoring
- When switching between spec implementations

---

## CI Scripts

### check-fast.ps1 (Fast Iteration)
```
.\check-fast.ps1
```
- Reads `git status --porcelain` to detect changed files
- Runs only relevant checks based on changed paths:
  - `src/`, `tests/` → cargo check + clippy
  - `src/main.rs`, `src/bin/` → adds --bins
  - `benches/`, `src/benchmarks/` → adds --benches
  - `docs/specs/`, `spec-lint.ps1` → runs spec-lint
- Runs `rustfmt --check` on changed `.rs` files
- Use during iteration — fast feedback

### check.ps1 (Full CI — Before Push)
```
.\check.ps1                          # Full run
.\check.ps1 -SkipTests               # Skip nextest
.\check.ps1 -SkipClippy              # Skip clippy
.\check.ps1 -SkipFormat              # Skip format check
.\check.ps1 -SkipBuild               # Skip cargo check
.\check.ps1 -SkipSpecLint            # Skip spec lint
```

**Run order** (short-circuits on first failure):
1. `spec-lint.ps1`
2. `cargo check`
3. `cargo fmt --all -- --check`
4. `cargo clippy --all-targets --all-features -- -D warnings`
5. `cargo clippy --lib -- -D warnings -D clippy::unwrap_used`
6. `cargo clippy --lib -- -D warnings -D clippy::expect_used`
7. `cargo clippy --bins -- -D clippy::unwrap_used -D clippy::expect_used`
8. `cargo nextest run --all-features --lib`

### spec-lint.ps1
```
.\spec-lint.ps1 [Directory]
```
- Validates spec package structure and bucket rules
- Self-audits: verifies it never modifies files (read-only assertion)
- Lint rules:
  - `REQUIRED_FILES`: Every spec must have spec.yaml + plan.md + validation.md
  - `BUCKET_RULES`: Template=draft, Active=valid statuses, Done=done
  - `ID_MATCH`: Folder name must match spec.yaml id (non-template only)
  - `PATH_SANITY`: No stale path references (e.g., `_done/` paths in `_active/` specs)
  - Quality checks: validation.md must not be a Coder Failure Report, no generic acceptance criteria

---

## Pre-Push Checklist

```powershell
# 1. Fast check during iteration
.\check-fast.ps1

# 2. Full CI before push (runs all 8 steps)
.\check.ps1

# 3. If all green, commit and push
git add <files>
git commit -m "type: summary (reason/impact)"
git push origin <branch>
```

### What check.ps1 Verifies
| Step | Check | Failure Guide |
|---|---|---|
| Spec lint | Spec package structure | `.\spec-lint.ps1` to see errors |
| Build | Compilation errors | `cargo check` for details |
| Format | Rustfmt compliance | `cargo fmt --all` to auto-fix |
| Clippy | Code quality (all targets) | `cargo clippy --fix` for auto-fixes |
| Clippy (lib) | `.unwrap()` banned | Use `?` or `.expect("...")` |
| Clippy (lib) | `.expect()` banned | Use `?` or `#[allow(...)]` |
| Clippy (bins) | unwrap/expect banned in binary targets | Use `?` or `.unwrap_or_default()` |
| Tests | Nextest lib tests passing | `cargo nextest run` for details |

---

## Git Hooks

Scripts in `scripts/` directory — installed via:
```bash
bash scripts/setup-hooks.sh
```

| Hook | Action |
|---|---|
| `pre-commit` | `cargo fmt --check` on staged `.rs` files |
| `commit-msg` | Validates conventional commits format |

> last audited 26-06-26 by docs-auditor
