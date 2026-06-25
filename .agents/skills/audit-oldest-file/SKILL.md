# Audit Oldest Files

Trigger: User says "audit oldest files", "find stale files", "run oldest audit", or provides a file list to audit.

**Purpose:** Identify the 2 oldest `.rs` and 2 oldest `.md` files by `LastWriteTime`, read and compare them against current codebase conventions, then "stamp" them with an audit note. Designed to be **loop-able** — each run stamps the oldest files, newer files surface next time. Both `.md` and `.rs` files get stamped so the loop progresses through all file types.

---

## 1. Workflow

```
find-oldest-files.ps1                   # Step 1: discover (2 per type, exits 0 if all current)
    ↓
Read each file                          # Step 2: study
    ↓
git log --oneline -- <path>             # Step 3: history
    ↓
Compare against current conventions     # Step 4: audit
    ↓
Stamp or file fix                       # Step 5: outcome
    ↓
Re-stamp any previously stamped files   # Step 6: always
    ↓
Loop back to Step 1                    # Loop: re-run finds next batch
```

### Step 1 — Run `find-oldest-files.ps1`

From the project root:

```powershell
.\find-oldest-files.ps1
```

This outputs:
- **2 oldest `.rs` files** by `LastWriteTime` (excludes `target/`, `.git/`, `crates/`, `build.rs`, `fuzz/`, `.opencode/`)
- **2 oldest `.md` files** by `LastWriteTime` (same exclusions)
- Each entry shows relative path, timestamp, and byte size
- **Exit code:** `0` if the oldest `.md` was modified within 24h (all files current); `1` if audit is needed

If exit code is `0`, the loop is done — no further audit rounds needed.

The script scans recursively from the project root. It is already defined at `./find-oldest-files.ps1`.

### Step 2 — Read each file

Use `read_files` to read the full contents of every file found. Do not skip files — even a 13-byte `build.rs` or an untracked fuzz target can reveal issues.

### Step 3 — Check git history

For each file, run:

```bash
git log --oneline -10 -- <path>
```

This reveals:
- When the file was last meaningfully changed
- Whether the file has ever been committed (fuzz targets may be untracked)
- Whether it was recently moved/archived (common for docs)
- The commit context for each change

### Step 4 — Audit categories

Compare each file against the **current codebase** using these criteria:

| Category | What to check | Signs of drift |
|---|---|---|
| **Doc → code** | Does the doc describe something that still exists? | Mentioned modules/functions removed or renamed; selectors changed; config fields restructured |
| **Dead code** | Is the file still used anywhere? | No imports/references; `crates/` code referencing removed deps; fuzz targets without CI or runner |
| **Pattern mismatch** | Does the file use current conventions? | Old error handling (`bail!` without context), old logging format (no context tags), old config struct pattern, old test naming |
| **Stale metadata** | Audit stamp, version references, date | `> *Last audited: DD-MM-YY by <name>*` is outdated; references to old branch names or feature flags |
| **Orphaned files** | Does nothing reference it? | `git grep` shows zero matches for its symbols; `docs/archive/` files without cross-references |

### Step 5 — Outcome

For each file, produce one of:

| Outcome | Action |
|---|---|
| **Re-stamp** | If the file already has an audit stamp — **always re-stamp it**. Update the date and agent name. Do not skip, flag, or ask. |
| **Stamp (.md)** | Add `> *Last audited: <DD-MM-YY> by <agent-name>*` on line 3 of markdown files |
| **Stamp (.rs)** | Add `// last audited <DD-MM-YY> by <agent-name>` on line 2 of Rust source files (after `//!` doc comments, before `use` statements) |
| **Fix** | Update the file to match current conventions and stamp it |
| **Delete** | Remove dead/unused files after confirming with user |
| **Archive** | Move to `docs/_archive/` if the content is historical but worth keeping |
| **Skip** | Leave as-is (e.g., `build.rs` with `fn main() {}` that is correctly minimal) |

### Step 6 — Always re-stamp previously stamped files

If a file already has an audit stamp (like `last audited 08-05-26 by Kilo`), **always re-stamp it** with the current date. Do not skip, flag, or ask the user — just update the date and agent name.

This applies even if:
- The file is an archived doc that hasn't changed
- The content appears accurate
- The file was stamped in a previous audit round
- The stamp format varies (`> *...*` blockquote vs plain `last audited` text)

Re-stamping tells future readers: "This file was verified as still relevant on this date." A stale stamp (over 1 month old) implies neglect, even if the content is fine.

---

## 2. Real Example from This Codebase

### Running the script

```powershell
.\find-oldest-files.ps1
```

**Output:**
```
=== Oldest .rs Files (by LastWriteTime) ===
  build.rs                               (13 bytes)
  fuzz/fuzz_targets/deserialize_spec_meta.rs  (425 bytes)
  fuzz/fuzz_targets/deserialize_task_definition.rs  (786 bytes)

=== Oldest .md Files (by LastWriteTime) ===
  docs/archive/plan/twitterActivity/02-config.md  (14720 bytes)
  docs/archive/plan/twitterActivity/03-agent.md   (22119 bytes)
  docs/archive/plan/twitterActivity/04-modules.md  (16042 bytes)
```

### Audit example: `build.rs`

```bash
git log --oneline -- build.rs
# 58eb62d chore: remove Windows icon resource wiring (simplify build)
# a112dfa feat: add custom icon support for Windows executable
```

**Finding:** `fn main() {}` — correctly minimal since icon support was removed.  
**Outcome:** Skip (no action needed).

### Audit example: `fuzz/fuzz_targets/deserialize_spec_meta.rs`

```bash
git log --oneline -- fuzz/fuzz_targets/deserialize_spec_meta.rs
# (no output — never committed)
```

**Finding:** File exists in worktree but has no git history. Check if fuzz targets are in `Cargo.toml` or if they're dead.  
**Outcome:** Flag for user decision (may need commit, removal, or CI integration).

### Audit example: `docs/archive/plan/twitterActivity/02-config.md`

```bash
git log --oneline -3 -- docs/archive/plan/twitterActivity/02-config.md
# 170b2b1 docs: audit 48 documentation files with stamps, add twitterActivity archive stamps
# a53743e (earlier refactor — moved from plan/ to archive/)
```

**Finding:** Archived planning doc from May 2026, already stamped with `> *Last audited: 08-05-26 by Kilo*`. Contains planned config structs that may not match current implementation.  
**Outcome:** Re-stamp (always re-stamp previously stamped files).

---

## 3. Audit Stamp Format

Use this exact format for markdown files:

```markdown
> *Last audited: <DD-MM-YY> by <agent-name>*
```

Place it on **line 3** (right after the `# Title` line, separated by a blank line).

Example:
```markdown
# Twitter Activity — Configuration & Profile

> *Last audited: 26-06-26 by Buffy*
```

For the agent name, use the name of the agent performing the audit (e.g., `Buffy`, `Kilo`, or the spawned agent's name).

### Rust Source Files (`.rs`)

Use this format for Rust source files:

```rust
// last audited <DD-MM-YY> by <agent-name>
```

Place it on **line 2** (right after the opening `//!` module doc comment, before any `use` statements).

Example:
```rust
//! Click learning engine for adaptive automation.
// last audited 26-06-26 by Buffy

use crate::runtime::task_context::click_learning::...
```

If the file has no `//!` doc comment, place the stamp on **line 1** instead.

---

## 4. Conventions Reference

When auditing, compare against these current project conventions:

### Logging
```rust
// Current: context-tagged logging
info!("[twitter][cycle {}] Entry: {}", self.state.cycle, url);
// NOT: info!("Entry: {}", url);
```

### Error handling
```rust
// Current: anyhow with context
.await.with_context(|| format!("failed to read {}", path))?;
// NOT: unwrap() or expect() in production code
```

### Config structs
```rust
// Current: serde(default) on optional fields, env var overrides in config/env.rs
#[serde(default)]
pub some_field: bool,
```

### Selectors
```rust
// Current: data-testid selectors with fallback priority
pub const TWEET_ARTICLE: &str = "article[data-testid=\"tweet\"]";
```

### Test naming
```rust
// Current: test_ prefix with descriptive name
fn test_normalize_modifier_ctrl() { ... }
// NOT: test1() or it_works()
```

---

## 5. Common Findings

| Finding | Typical file type | Action |
|---|---|---|
| **Minimal/trivial** | `build.rs` | Skip — it's intentionally small |
| **Untracked by git** | `fuzz/` targets, generated files | Flag for user — may need `git add`, `.gitignore`, or removal |
| **Archived planning docs** | `docs/archive/plan/` | Always re-stamp — they already have stamps from the previous audit |
| **Comment-only files** | Any | Check if the comments are still accurate |
| **Dead modules** | `src/` files | Use `git grep <module_name>` to verify nothing imports them |
| **Rust source files** | `src/` files | Stamp with `// last audited` comment — this lets the loop progress to deeper .rs files |

---

## 6. Pitfalls

| # | Pitfall | Why |
|---|---|---|
| 1 | **Only check `LastWriteTime`** | File timestamps can change on clone/checkout — always cross-check with `git log` |
| 2 | **Skip fuzz targets** | They may be untracked dead code OR pending additions — always check both |
| 3 | **Assume archived = irrelevant** | Archived planning docs may still contain useful design rationale |
| 4 | **Over-audit `build.rs`** | `fn main() {}` is the correct pattern when no build script is needed |
| 5 | **Forget to stamp after update** | If you fix a doc, always add/renew the audit stamp |
| 6 | **Stamp without reading** | Always read the full file before stamping — a stamp implies the content was verified |
| 7 | **Ignore file size** | Very large files (>15KB) may need deeper review; very small files may be trivial |
| 8 | **Skip `docs/archive/`** | Archived docs are often the oldest by timestamp — they deserve audit too |

> last audited 26-06-26 by docs-auditor
