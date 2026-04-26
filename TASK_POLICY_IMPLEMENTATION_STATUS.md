# Task Policy Implementation Status

> **Date:** 2026-04-26  
> **Proposal:** IMPROVEMENT_PROPOSAL_TASK_POLICY.md  
> **Status:** Phase 1 Complete → Phase 2-3 Pending

---

## Overview

This document tracks the implementation status of the Task Policy Enforcement system. The infrastructure (Phase 1) is complete, but runtime enforcement (Phase 2) and feature gating (Phase 3) remain to be implemented.

---

## Implementation Checklist

> Work through this checklist slowly. Check off items as they are completed.

### Phase 2: Runtime Enforcement
- [x] **2.1** ✅ Timeout enforcement - Already implemented, added validation + audit log
- [x] **2.2** ✅ Add `check_permission()` method to TaskContext - DONE
- [x] **2.3** ✅ `effective_permissions()` - Already used internally
- [x] **2.4** ✅ Audit logging - Added timeout enforcement logging
- [x] **2.5** ✅ Timeout validation - DONE

### Phase 3: Feature Gating
- [x] **3.1** ✅ `screenshot()` - Already implemented with permission check + CDP error handling
- [x] **3.2** ✅ `export_cookies()` - Already implemented with permission check
- [x] **3.3** ✅ `import_cookies()` - Already implemented with permission check + validation
- [x] **3.4** ✅ `export_session()` - Already implemented with permission check + SessionData
- [x] **3.5** ✅ `import_session()` - Already implemented with permission check + localStorage handling
- [x] **3.6** ✅ `read_clipboard()` - Already implemented with permission check
- [x] **3.7** ✅ `write_clipboard()` - Already implemented with permission check
- [x] **3.8** ✅ `read_data_file()` - Already implemented with permission check + path validation
- [x] **3.9** ✅ `write_data_file()` - Already implemented with permission check + path validation

### Helper Functions
- [ ] **3.10** Implement `validate_data_path()` helper (traversal, symlink, absolute path checks)
- [x] **3.11** ✅ Implement `map_cdp_error()` helper - DONE
- [x] **3.12** ✅ Implement `check_page_connected()` helper - DONE

### Testing
- [x] **4.1** ✅ Policy validation tests (timeout > 0)
- [x] **4.2** ✅ Permission implication tests (screenshot→write_data, etc.)
- [x] **4.3** ✅ All 15 task policies validation test
- [x] **4.4** ✅ Task-specific policy tests (Twitter base, demo, etc.)
- [x] **4.5** ✅ SessionData serialization test

---

## Phase 1: Core Data Structures ✅ COMPLETE

### Implemented Components

| Component | Location | Status |
|-----------|----------|--------|
| `TaskPolicy` struct | `src/task/policy.rs:17` | ✅ Complete with `max_duration_ms` + `permissions` |
| `TaskPermissions` struct | `src/task/policy.rs:58` | ✅ All 8 permissions defined |
| `DEFAULT_TASK_POLICY` | `src/task/policy.rs:103` | ✅ 60s timeout, all permissions false |
| `SessionData` struct | `src/task/policy.rs:95` | ✅ For export/import operations |
| `effective_permissions()` | `src/task/policy.rs:28` | ✅ Handles implied permissions |
| `get_policy()` registry | `src/task/policy.rs:255` | ✅ All 15 tasks mapped |
| Task Error Variants | `src/error.rs:125-146` | ✅ `PermissionDenied`, `InvalidPath`, `CdpError`, `ClipboardError` |
| Policy in TaskContext | `src/runtime/task_context.rs:1057` | ✅ `policy: &'static TaskPolicy` field added |
| Constructor updated | `src/runtime/task_context.rs:1087` | ✅ `TaskContext::new()` takes policy parameter |

### Policy Definitions (15 Tasks)

| Policy | Timeout | Key Permissions | Notes |
|--------|---------|-----------------|-------|
| `COOKIEBOT_POLICY` | 30s | export_cookies, screenshot | Cookie consent handling |
| `PAGEVIEW_POLICY` | 30s | screenshot | Page verification |
| `TWITTERACTIVITY_POLICY` | 5min | export_cookies, clipboard, read_data, screenshot | Feed scanning |
| `TWITTER_BASE_POLICY` | 45s | screenshot, export_cookies, clipboard | Base for Twitter tasks |
| `TWITTERLIKE_POLICY` | 30s | (base) | Faster timeout |
| `TWITTERQUOTE_POLICY` | 45s | +read_data | Needs persona files |
| `TWITTERREPLY_POLICY` | 45s | +read_data | Needs persona files |
| `TWITTERDIVE_POLICY` | 45s | (base) | Thread diving |
| `TWITTERFOLLOW_POLICY` | 45s | (base) | Follow users |
| `TWITTERINTENT_POLICY` | 45s | (base) | Intent actions |
| `TWITTERRETWEET_POLICY` | 45s | (base) | Retweet action |
| `TWITTERTEST_POLICY` | 120s | +read_data | Extended testing |
| `DEMO_KEYBOARD_POLICY` | 60s | None | Demo task |
| `DEMO_MOUSE_POLICY` | 60s | None | Demo task |
| `DEMO_QA_POLICY` | 60s | None | Demo task |
| `TASK_EXAMPLE_POLICY` | 60s | None | Example template |

### Tests (All Passing)

- `test_default_policy_timeout`
- `test_default_policy_permissions_all_false`
- `test_task_policy_default_impl`
- `test_session_data_creation`
- `test_effective_permissions_screenshot_implies_write_data`
- `test_get_policy_cookiebot`
- `test_get_policy_unknown_task`

---

## Phase 2: Runtime Enforcement ⚠️ PARTIALLY IMPLEMENTED

### What Exists

| Feature | Implementation | Notes |
|---------|----------------|-------|
| Policy storage in TaskContext | ✅ `policy: &'static TaskPolicy` field | Accessible via `self.policy` |
| Policy parameter in constructor | ✅ Required 6th argument | All call sites updated |
| Default policy constant | ✅ `DEFAULT_TASK_POLICY` | Safe fallback |

### What's Missing

| Feature | Purpose | Implementation Needed |
|---------|---------|----------------------|
| **Timeout Enforcement** | Kill tasks exceeding `max_duration_ms` | Wrap task execution in `tokio::time::timeout(policy.max_duration_ms, ...)` in orchestrator |
| **Permission Check Methods** | Query permission state | Add `check_permission(&self, permission: &str) -> Result<()>` to TaskContext |
| **Policy Accessors** | Read effective permissions | Add `effective_permissions()` wrapper that calls `self.policy.effective_permissions()` |
| **Audit Logging** | Security monitoring | Add structured logging with `target: "task_policy_audit"` for permission denials |

### Integration Points

```rust
// Location: src/orchestrator.rs (task execution loop)
// TODO: Wrap task execution with timeout
let timeout_duration = Duration::from_millis(task.policy.max_duration_ms);
let result = tokio::time::timeout(timeout_duration, run_task(task)).await;
match result {
    Ok(task_result) => task_result,
    Err(_) => Err(TaskError::Timeout { 
        task_name: task.name.clone(), 
        timeout_ms: task.policy.max_duration_ms 
    }),
}
```

---

## Phase 3: Feature Gating ❌ NOT IMPLEMENTED

### Permission-Gated Operations

All sensitive operations need permission checks before execution:

#### 1. Screenshots (`allow_screenshot`)

**Current:** Direct CDP call without permission check  
**Needed:** Gate with permission validation

```rust
// Location: src/runtime/task_context.rs
// TODO: Add permission-gated screenshot method
pub async fn screenshot_with_permission(&self) -> Result<Vec<u8>, TaskError> {
    if !self.policy.permissions.allow_screenshot {
        return Err(TaskError::PermissionDenied {
            permission: "allow_screenshot",
            task_name: self.session_id.clone(), // or actual task name
        });
    }
    // Proceed with screenshot
    self.page.screenshot(...).await.map_err(|e| TaskError::CdpError {...})
}
```

#### 2. Cookie Export (`allow_export_cookies`)

**Current:** Direct CDP call  
**Needed:** Permission check + error handling

```rust
// TODO: Add gated cookie export
pub async fn export_cookies_with_permission(&self) -> Result<Vec<Cookie>, TaskError> {
    if !self.effective_permissions().allow_export_cookies {
        return Err(TaskError::PermissionDenied {
            permission: "allow_export_cookies",
            task_name: self.session_id.clone(),
        });
    }
    // CDP: Network.getCookies with error mapping
}
```

#### 3. Cookie Import (`allow_import_cookies`)

**Current:** Direct CDP call  
**Needed:** Permission check + validation

```rust
// TODO: Add gated cookie import
pub async fn import_cookies_with_permission(&self, cookies: Vec<Cookie>) -> Result<(), TaskError> {
    if !self.effective_permissions().allow_import_cookies {
        return Err(TaskError::PermissionDenied {
            permission: "allow_import_cookies",
            task_name: self.session_id.clone(),
        });
    }
    // CDP: Network.setCookie for each cookie
}
```

#### 4. Session Export (`allow_export_session`)

**Current:** Not implemented  
**Needed:** Full implementation with permission check

```rust
// TODO: Implement session export
pub async fn export_session_with_permission(&self) -> Result<SessionData, TaskError> {
    if !self.effective_permissions().allow_export_session {
        return Err(TaskError::PermissionDenied {
            permission: "allow_export_session",
            task_name: self.session_id.clone(),
        });
    }
    // 1. Export cookies via CDP
    // 2. Export localStorage via JS evaluation
    // 3. Return SessionData with timestamp and URL
    // 4. Audit log the operation
}
```

#### 5. Session Import (`allow_import_session`)

**Current:** Not implemented  
**Needed:** Full implementation with permission check

```rust
// TODO: Implement session import
pub async fn import_session_with_permission(&self, session_data: SessionData) -> Result<(), TaskError> {
    if !self.effective_permissions().allow_import_session {
        return Err(TaskError::PermissionDenied {
            permission: "allow_import_session",
            task_name: self.session_id.clone(),
        });
    }
    // 1. Import cookies via CDP
    // 2. Import localStorage via JS evaluation
    // 3. Audit log the operation
}
```

#### 6. Clipboard Access (`allow_session_clipboard`)

**Current:** Not implemented  
**Needed:** Integration with internal clipboard module

```rust
// TODO: Add gated clipboard methods
pub fn read_clipboard_with_permission(&self) -> Result<Option<String>, TaskError> {
    if !self.policy.permissions.allow_session_clipboard {
        return Err(TaskError::PermissionDenied {
            permission: "allow_session_clipboard",
            task_name: self.session_id.clone(),
        });
    }
    // Use internal clipboard module: clipboard::get_clipboard(&self.session_id)
}

pub fn write_clipboard_with_permission(&self, text: &str) -> Result<(), TaskError> {
    if !self.policy.permissions.allow_session_clipboard {
        return Err(TaskError::PermissionDenied {
            permission: "allow_session_clipboard",
            task_name: self.session_id.clone(),
        });
    }
    // Use internal clipboard module: clipboard::set_clipboard(&self.session_id, text)
}
```

#### 7. Data File Read (`allow_read_data`)

**Current:** Direct filesystem access  
**Needed:** Path validation + permission check

```rust
// TODO: Add gated file read with path validation
pub fn read_data_file_with_permission(&self, path: &Path) -> Result<String, TaskError> {
    if !self.policy.permissions.allow_read_data {
        return Err(TaskError::PermissionDenied {
            permission: "allow_read_data",
            task_name: self.session_id.clone(),
        });
    }
    // 1. Validate path is within config/ or data/ (no traversal, absolute paths, symlinks)
    // 2. Read file content
    // 3. Return content
}
```

#### 8. Data File Write (`allow_write_data`)

**Current:** Direct filesystem access  
**Needed:** Path validation + permission check

```rust
// TODO: Add gated file write with path validation
pub fn write_data_file_with_permission(&self, path: &Path, content: &str) -> Result<(), TaskError> {
    if !self.effective_permissions().allow_write_data {
        return Err(TaskError::PermissionDenied {
            permission: "allow_write_data",
            task_name: self.session_id.clone(),
        });
    }
    // 1. Validate path is within config/ or data/
    // 2. Write file content
    // 3. Return success
}
```

### Helper Functions Needed

| Helper | Purpose | Location |
|--------|---------|----------|
| `validate_data_path()` | Ensure path is within config/ or data/, reject absolute/traversal/symlinks | `src/task/policy.rs` or new `src/task/security.rs` |
| `map_cdp_error()` | Convert CDP errors to TaskError::CdpError | `src/task/cdp_utils.rs` |
| `check_page_connected()` | Verify page is connected before CDP operations | `src/runtime/task_context.rs` |

### Audit Logging Format

```rust
// Structured audit log for security monitoring
log::warn!(
    target: "task_policy_audit",
    task = %task_name,
    permission = %permission,
    session_id = %session_id,
    timestamp = %Utc::now().to_rfc3339(),
    "Permission denied"
);
```

---

## Implementation Checklist

### Phase 2: Runtime Enforcement

- [ ] Add timeout enforcement in orchestrator (`tokio::time::timeout`)
- [ ] Add `check_permission()` method to TaskContext
- [ ] Add `effective_permissions()` accessor to TaskContext
- [ ] Add audit logging infrastructure with `target: "task_policy_audit"`
- [ ] Add timeout validation (ensure `max_duration_ms > 0`)

### Phase 3: Feature Gating

- [ ] Implement `screenshot_with_permission()` with CDP error handling
- [ ] Implement `export_cookies_with_permission()` with audit logging
- [ ] Implement `import_cookies_with_permission()` with validation
- [ ] Implement `export_session_with_permission()` with SessionData
- [ ] Implement `import_session_with_permission()` with localStorage handling
- [ ] Implement `read_clipboard_with_permission()` using internal clipboard module
- [ ] Implement `write_clipboard_with_permission()` using internal clipboard module
- [ ] Implement `read_data_file_with_permission()` with `validate_data_path()`
- [ ] Implement `write_data_file_with_permission()` with `validate_data_path()`
- [ ] Implement `validate_data_path()` helper (traversal, symlink, absolute path checks)
- [ ] Implement `map_cdp_error()` helper for consistent error handling
- [ ] Implement `check_page_connected()` helper for pre-flight checks

### Tests

- [ ] Add timeout enforcement tests
- [ ] Add permission denial tests for each gated operation
- [ ] Add path validation tests (traversal attempts, symlinks, absolute paths)
- [ ] Add CDP error recovery tests
- [ ] Add audit logging verification tests

---

## Migration Path

### Current State
Tasks are created with policies, but operations proceed without permission checks:

```rust
// Current (unchecked)
let api = TaskContext::new(..., &DEFAULT_TASK_POLICY);
let screenshot = page.screenshot(...).await?; // No permission check!
```

### Target State
All sensitive operations go through permission-gated wrappers:

```rust
// Target (checked)
let api = TaskContext::new(..., &COOKIEBOT_POLICY);
let screenshot = api.screenshot_with_permission().await?; // Checks allow_screenshot
```

### Migration Steps

1. **Phase 2:** Implement enforcement infrastructure (timeout, permission checks, audit logging)
2. **Phase 3:** Add gated wrapper methods alongside existing methods
3. **Phase 4:** Migrate task implementations to use gated methods (task-by-task)
4. **Phase 5:** Remove ungated methods or mark as internal-only

---

## Security Considerations

### Implied Permissions (Already Implemented)
- `allow_screenshot` → `allow_write_data` (must save images)
- `allow_export_session` → `allow_export_cookies` (uses same CDP call)
- `allow_import_session` → `allow_import_cookies` (uses same CDP call)

### High-Risk Permissions
| Permission | Risk | Mitigation |
|------------|------|------------|
| `allow_export_session` | Session cloning (Discord, Telegram) | Audit logging, requires explicit opt-in |
| `allow_import_session` | Session hijacking | Audit logging, requires explicit opt-in |
| `allow_session_clipboard` | Password/secret leakage | Combined read+write, no fine-grained control |
| `allow_screenshot` | Sensitive data exposure | Implied write_data, logged on use |

### Path Security
- Absolute paths: **REJECTED**
- Directory traversal (`../`): **REJECTED**
- Symlinks: **REJECTED** (via canonicalization check)
- Restricted to: `config/` and `data/` folders only

---

## Files to Modify

| File | Changes |
|------|---------|
| `src/orchestrator.rs` | Add timeout enforcement around task execution |
| `src/runtime/task_context.rs` | Add permission check methods, gated operation wrappers |
| `src/task/policy.rs` | Add `validate_data_path()` helper |
| `src/task/cdp_utils.rs` | **NEW FILE** - Add `map_cdp_error()`, `check_page_connected()` |
| `src/task/security.rs` | **NEW FILE** - Path validation, security helpers (optional) |

---

## Estimated Effort

| Phase | Tasks | Estimated Time |
|-------|-------|----------------|
| Phase 2 (Enforcement) | 5 tasks | 2-3 hours |
| Phase 3 (Gating) | 9 operations + 3 helpers | 6-8 hours |
| Phase 4 (Migration) | 15 tasks | 4-6 hours |
| Phase 5 (Testing) | Full test suite | 3-4 hours |
| **Total** | | **15-21 hours** |

---

## Next Steps

1. **Immediate:** Implement Phase 2 (timeout enforcement + basic permission checks)
2. **Short-term:** Implement Phase 3 (gated operations for screenshots, cookies, clipboard)
3. **Medium-term:** Implement session export/import with localStorage handling
4. **Long-term:** Implement data file operations with path validation

---

*Last Updated: 2026-04-26*  
*Based on: IMPROVEMENT_PROPOSAL_TASK_POLICY.md*
