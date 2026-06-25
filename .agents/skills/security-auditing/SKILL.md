# Security Auditing

Guide to the security subsystem — path validation, policy permission model, cargo deny configuration, and safe task execution patterns.

---

## Architecture

```
Task Execution → TaskPolicy (permissions check) → Permission Granted → Execute
                                                       ↓
                                              Permission Denied → Error

External Input → validate_data_path() → Path Security Checks → Safe Path
                                           ↓
                                     cargo deny (vulnerability + license audit)
```

---

## File Map

| File | Purpose |
|---|---|
| `src/task/security.rs` | Path validation utilities — `validate_data_path()`, `is_safe_path()`, `contains_traversal()` |
| `src/task/policy.rs` | `TaskPolicy`, `TaskPermissions` (12 flags), `DEFAULT_TASK_POLICY`, 16+ per-task static policies, `get_policy_from_registry()`, `effective_permissions()`, `SessionData`, `BrowserData` |
| `deny.toml` | `cargo deny` configuration — advisory ignores, license allowlist, ban rules, source restrictions |
| `.cargo/audit.toml` | `cargo audit` advisory exceptions |
| `Cargo.toml` | Dependency tree — audited for unsafe deps, unmaintained packages |

---

## Path Validation (`security.rs`)

The `validate_data_path()` function enforces 4 security checks:

### Check 1: Empty Rejection
```rust
if relative_path.is_empty() {
    return Err("Path cannot be empty");
}
```

### Check 2: Absolute Path Rejection
Rejects:
- Unix absolute: `/etc/passwd`
- Windows absolute: `C:\Windows\System32`, `D:\file.txt`
- UNC paths: `\\server\share`
- Root-relative: `/abs/path`

### Check 3: Directory Traversal Rejection
Rejects `..` components in paths:
- Normalizes `\` to `/` before checking
- Rejects: `../secret.txt`, `foo/../bar`, `data/../../../etc/passwd`
- Allows single dots: `./file.txt`, `data/./file.json` (current directory reference)

### Check 4: Allowed Directory Enforcement
- Must resolve within `config/` or `data/` directories
- Uses `canonicalize()` to check resolved path doesn't escape allowed dir
- For write operations where file doesn't exist yet: defaults to `config/` prefix

### Helper Functions

| Function | Description |
|---|---|
| `contains_traversal(path)` | Lightweight syntactic check (no filesystem access) — detects `..` in any component |
| `is_safe_path(path)` | Combination check (not empty, not absolute, no traversal) — useful for write operations |

### Test Coverage
- 20+ unit tests covering: absolute paths (Unix/Windows), traversal (prefix/middle/suffix), empty, safe relative, single dot, double slash, mixed separators

---

## Policy Permission Model (`policy.rs`)

### TaskPolicy Structure
```rust
pub struct TaskPolicy {
    pub max_duration_ms: DurationMs,    // MANDATORY — non-zero, type-guaranteed
    pub permissions: TaskPermissions,   // 12 boolean flags
}
```

### TaskPermissions (12 Flags)

| Flag | Implication | Description |
|---|---|---|
| `allow_screenshot` | ⇒ `allow_write_data` | Capture screenshots |
| `allow_export_cookies` | — | Export cookies from browser |
| `allow_import_cookies` | — | Import cookies into browser |
| `allow_export_session` | ⇒ `allow_export_cookies` | Export session (cookies + localStorage) |
| `allow_import_session` | ⇒ `allow_import_cookies` | Import session |
| `allow_session_clipboard` | — | Read/write clipboard |
| `allow_read_data` | — | Read files from `config/` or `data/` |
| `allow_write_data` | — | Write files to `config/` or `data/` |
| `allow_http_requests` | — | HTTP requests (GET, POST) |
| `allow_dom_inspection` | — | DOM inspection (styles, positions) |
| `allow_browser_export` | — | Export complete browser data |
| `allow_browser_import` | — | Import complete browser data |

### Effective Permissions
`effective_permissions()` adds implied permissions:
- `allow_screenshot` ⇒ `allow_write_data` = true (must save image)
- `allow_export_session` ⇒ `allow_export_cookies` = true (uses same CDP call)
- `allow_import_session` ⇒ `allow_import_cookies` = true

### DEFAULT_TASK_POLICY
- **Timeout**: 180,000ms (3 minutes)
- **All permissions**: false
- Used for unknown tasks and as safe base

### Per-Task Policy Registry
16+ policies registered via `match_policy_by_name()`:
- Twitter family (9 tasks) inherit from `TWITTER_BASE_POLICY`
- CookieBot, PageView, Demo tasks have distinct policies
- Each task's duration is sourced from its own `DEFAULT_*_DURATION_MS` constant

### SessionData & BrowserData
- `SessionData` — cookies + localStorage + export timestamp
- `BrowserData` — cookies + localStorage + sessionStorage + IndexedDB names + browser version
- Both exported with `exported_at` timestamp for audit trail

---

## Cargo Deny Configuration (`deny.toml`)

### Advisories
```toml
[advisories]
ignore = [
    "RUSTSEC-2025-0068",  # serde_yml 0.0.13 — unsound (migration tracked in TODO.md)
    "RUSTSEC-2025-0052",  # async-std 1.13.2 — unmaintained (transitive dep only)
]
```
- Vulnerability database fetched automatically from rustsec advisory-db
- Only 2 advisories ignored (both tracked for resolution)

### License Allowlist
**Allowed licenses** (13): MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, CC0-1.0, Unlicense, MPL-2.0, OpenSSL, CDLA-Permissive-2.0, Apache-2.0 WITH LLVM-exception

- Confidence threshold: 0.8
- Private crate check: enabled (no ignoring)

### Ban Rules
- Multiple versions: warn (not deny)
- Wildcards: allow
- Highlight: all

### Source Restrictions
- Unknown registries: deny
- Unknown git: deny
- Allowed registry: `https://github.com/rust-lang/crates.io-index`

---

## cargo audit Configuration (`.cargo/audit.toml`)

Advisory exception for unmaintained transitive dependency:
- `async-std` (unmaintained, transitive via chromiumoxide)
> `serde_yml` is only in `deny.toml`, not in `audit.toml` — the two files have separate ignore lists.

---

## Safe Task Execution Patterns

### Input Validation
- All task parameters validated before execution (type checks, URL format, selector format)
- Persona file paths validated through `validate_data_path()`
- YAML parsing with size limits prevents DoS via large task definitions

### Permission Enforcement
- Tasks declare their required permissions via `policy_name` in registry
- Runtime checks verify permissions before each API call
- Unknown tasks get `DEFAULT_TASK_POLICY` (all permissions off)
- Implied permissions ensure consistency (e.g., screenshot always includes write access)

### Circuit Breaker (Session-Level)
- Prevents cascading failures across tasks sharing a session
- Threshold: 5 failures within 30s window
- Automatically resets after timeout expires

### Dependency Security
- `cargo deny check` run as part of CI
- Only 2 ignored advisories (both documented with resolution tracking)
- Miri test suite passes (no undefined behavior in safe code paths)

> last audited 26-06-26 by docs-auditor

---

## Adding a New Security Check

1. **Path validation**: Add check to `validate_data_path()` in `security.rs`, add corresponding helper, add tests for each edge case
2. **Policy permission**: Add new flag to `TaskPermissions`, add implication in `effective_permissions()`, set in relevant per-task policies, update `DEFAULT_TASK_POLICY`
3. **Dependency audit**: Add to `deny.toml` advisory ignores only with documented tracking, copy to `audit.toml` if needed

---

## Testing

| Test Location | Command |
|---|---|
| Path validation tests | `cargo test --lib task::security::tests` |
| Policy tests (permissions, implications, registry) | `cargo test --lib task::policy::tests` |
| Dependency vulnerability check | `cargo deny check` |
| License check | `cargo deny check licenses` |
| Miri undefined behavior check | `cargo +nightly miri test --lib` |

---

## Pitfalls

| # | Pitfall | Explanation |
|---|---|---|
| 1 | **Effective permissions not checked at runtime** | `effective_permissions()` computes implications but callers must explicitly use it. Using raw `permissions` skips implications. |
| 2 | **Path validation requires canonicalize** | `validate_data_path()` calls `canonicalize()` which requires the file to exist. For write operations where the file doesn't exist yet, it falls back to `config/` prefix. |
| 3 | **DEFAULT_TASK_POLICY blocks everything** | Unknown tasks get zero permissions. If a new task's policy isn't registered in `match_policy_by_name()`, all API calls will fail. |
| 4 | **serde_yml ignore is a temporary gap** | Two advisories are ignored (`RUSTSEC-2025-0068` for serde_yml). Migration to `serde_yaml` is complete in code but the old dep may still appear in lockfile. |
| 5 | **SessionData doesn't encrypt cookies** | `SessionData.cookies` stores raw JSON values. If written to disk, they're in plaintext JSON. No encryption at rest. |
