# Task Policy Enforcement Proposal (Minimal)

## Philosophy

**One mandatory timeout + 8 permissions = 9 total policies.**

The only hard requirement is `max_duration_ms` - tasks cannot hang forever. Eight boolean permissions control what the task is allowed to do, defaulted as `false`.

**Design Notes:**
- `allow_screenshot` permission implies `allow_write_data` capability (screenshots must be saved)
- `allow_export_session` and `allow_import_session` are **intentionally separate** for granular control
- Clipboard read/write are **combined** as `allow_session_clipboard`

## Policy Structure

```rust
/// Task execution policy - one hard limit + 8 permissions
#[derive(Debug, Clone, Default)]
pub struct TaskPolicy {
    /// Maximum execution time in milliseconds (MANDATORY)
    /// Task will be killed after this duration
    pub max_duration_ms: u64,
    
    /// Permission flags - what this task is allowed to do
    pub permissions: TaskPermissions,
}

/// Simple boolean permissions
#[derive(Debug, Clone, Default)]
pub struct TaskPermissions {
    /// Allow capturing screenshots
    /// NOTE: Implies allow_write_data capability (screenshots must be saved)
    pub allow_screenshot: bool,
    
    /// Allow exporting cookies from browser
    pub allow_export_cookies: bool,
    
    /// Allow importing cookies into browser
    pub allow_import_cookies: bool,
    
    /// Allow exporting full session data (cookies + localStorage)
    pub allow_export_session: bool,
    
    /// Allow importing full session data (cookies + localStorage)
    /// NOTE: Intentionally separate from allow_export_session for granular control
    pub allow_import_session: bool,
    
    /// Allow reading/writing clipboard
    /// NOTE: Combined read+write permission
    pub allow_session_clipboard: bool,
    
    /// Allow reading data files from config/ or data/ folders
    pub allow_read_data: bool,
    
    /// Allow writing data files to config/ or data/ folders
    pub allow_write_data: bool,
}
```

## Permission Utilities & Use Cases

### 1. `allow_screenshot` - Visual Debugging & Evidence

**Purpose:** Allow tasks to capture screenshots of the browser window.

**Use Cases:**
- **Debugging failures** - Capture page state when task fails
- **Evidence collection** - Document what was seen during execution
- **Visual verification** - Confirm page loaded correctly

**Implementation:**
```rust
// CDP: Page.captureScreenshot
let screenshot = page.screenshot(ScreenshotParams::default()).await?;
// Saves to file (requires implied allow_write_data capability)
```

**Security consideration:** Screenshots may contain sensitive information (personal data, credentials, private content).

**Implies:** `allow_write_data` (screenshots must be saved to disk)

---

### 2. `allow_export_cookies` - Cookie Export

**Purpose:** Allow tasks to export browser cookies from the current session.

**Use Cases:**
- **Cookie consent verification** - Export cookies to verify consent state
- **Session debugging** - Export cookies for troubleshooting authentication

**Implementation:**
```rust
// CDP: Network.getCookies
let cookies = page.execute(Network::get_cookies()).await?;
```

**Security consideration:** Cookies may contain authentication tokens and session IDs.

---

### 3. `allow_import_cookies` - Cookie Import

**Purpose:** Allow tasks to import cookies into the browser session.

**Use Cases:**
- **Session restoration** - Restore previously exported cookies to resume session
- **Pre-authentication** - Set login cookies before navigating to authenticated pages
- **Multi-account switching** - Import different account cookies for testing

**Implementation:**
```rust
// CDP: Network.setCookie for each cookie
for cookie in cookies {
    page.execute(Network::set_cookie(
        cookie.name,
        cookie.value,
        Some(cookie.domain.as_str()),
        Some(cookie.path.as_str()),
        cookie.expires,
        cookie.http_only,
        cookie.secure,
        cookie.same_site,
        cookie.priority,
    )).await?;
}
```

**Security consideration:** Importing malicious cookies could hijack sessions or inject tracking data. Combined with `allow_export_cookies`, enables full session transfer between browsers.

---

### 4. `allow_export_session` - Full Session Export

**Purpose:** Allow tasks to export complete session data (cookies + localStorage).

**Use Cases:**
- **Session backup** - Export full session for backup purposes
- **Debugging complex auth** - Export both cookies and localStorage for troubleshooting

**Implementation:**
```rust
// CDP: Network.getCookies + Runtime.evaluate (for localStorage)
let cookies = page.execute(Network::get_cookies()).await?;

// Execute JavaScript to read localStorage
let local_storage: Value = page.evaluate(r#"
    JSON.stringify(localStorage)
"#).await?;
```

**Security consideration:** This is a **HIGH RISK** permission. Enables session cloning (e.g., cloning Discord, Telegram sessions).

**Relationship:** Intentionally **separate** from `allow_import_session`. A task may only need to backup (export) without restoring (import).

---

### 5. `allow_import_session` - Full Session Import

**Purpose:** Allow tasks to import complete session data (cookies + localStorage).

**Use Cases:**
- **Session restoration** - Restore full saved session from file
- **Multi-account automation** - Switch between different saved sessions
- **Test environment setup** - Import pre-configured sessions for testing

**Implementation:**
```rust
// 1. Import cookies via CDP
for cookie in session_data.cookies {
    page.execute(Network::set_cookie(
        cookie.name, cookie.value,
        Some(cookie.domain.as_str()),
        Some(cookie.path.as_str()),
        cookie.expires, cookie.http_only,
        cookie.secure, cookie.same_site,
        cookie.priority,
    )).await?;
}

// 2. Import localStorage via JavaScript evaluation
let js_code = format!(
    r#"Object.entries({}).forEach(([k,v]) => localStorage.setItem(k,v))"#,
    session_data.local_storage
);
page.evaluate(js_code).await?;
```

**Security consideration:** This is a **HIGH RISK** permission. Importing a malicious session file could:
- Hijack authenticated sessions
- Inject malicious localStorage data
- Enable account takeover

**Relationship:** Intentionally **separate** from `allow_export_session`. Granular control allows:
- Backup-only tasks (export only)
- Import-only tasks (restore from trusted sources)
- Full session manager tasks (both)

---

### 6. `allow_session_clipboard` - Text Transfer (Read + Write)

**Purpose:** Allow tasks to read from and write to the system clipboard.

**Use Cases:**
- **Text extraction** - Copy text from page to clipboard
- **Form filling** - Paste data from clipboard into forms

**Implementation:**
```rust
// Read: OS clipboard API
let content = clipboard.get_text()?;

// Write: OS clipboard API  
clipboard.set_text("content")?;
```

**Security consideration:** Clipboard access can leak passwords and sensitive data.

**Design:** Combined read+write permission. Most clipboard use cases need both directions.

---

### 7. `allow_read_data` - External Data Loading

**Purpose:** Allow tasks to read files from `config/` or `data/` folders.

**Use Cases:**
- **Configuration loading** - Read task-specific config files
- **Data injection** - Load text data to type into forms
- **Session file loading** - Load exported session files for import

**Implementation:**
```rust
// Standard filesystem read, restricted to config/ and data/
let content = std::fs::read_to_string(path)?;
```

**Security consideration:** Path-restricted to config/data folders only. Rejects absolute paths and directory traversal.

---

### 8. `allow_write_data` - External Data Saving

**Purpose:** Allow tasks to write files to `config/` or `data/` folders.

**Use Cases:**
- **Session file saving** - Save exported sessions to disk
- **Screenshot saving** - Save captured screenshots (also implied by `allow_screenshot`)
- **Data persistence** - Save task results for later use
- **Log/debug output** - Write debug information to files

**Implementation:**
```rust
// Standard filesystem write, restricted to config/ and data/
std::fs::write(path, content)?;
```

**Security consideration:** Path-restricted to config/data folders only. Prevents overwriting system files or sensitive data.

**Relationship:** `allow_screenshot` permission **implies** `allow_write_data` (screenshots must be saved).

---

## Permission Matrix

| Permission | CookieBot | PageView | TwitterActivity | DataTyping | DemoQA | SessionManager | ScreenshotTask |
|------------|-----------|----------|-----------------|------------|--------|----------------|----------------|
| `allow_screenshot` | ✅ Debug | ✅ Verify | ✅ Debug | ❌ | ❌ | ✅ Debug | ✅ Capture |
| `allow_export_cookies` | ✅ Verify | ❌ | ✅ Debug | ❌ | ❌ | ✅ Export | ❌ |
| `allow_import_cookies` | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ Import | ❌ |
| `allow_export_session` | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ Export | ❌ |
| `allow_import_session` | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ Import | ❌ |
| `allow_session_clipboard` | ❌ | ❌ | ✅ Copy/paste | ✅ Paste | ❌ | ❌ | ❌ |
| `allow_read_data` | ❌ | ❌ | ✅ Persona | ✅ Data file | ❌ | ✅ Session files | ❌ |
| `allow_write_data` | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ Save sessions | ✅ Save images |

**Notes:**
- `allow_screenshot` tasks get `allow_write_data` implicitly (for saving images)
- `SessionManager` has most permissions for full session management
- Most tasks only need 1-3 permissions

---

## Default Policy (Safe Defaults)

```rust
// src/task/policy.rs

pub static DEFAULT_TASK_POLICY: TaskPolicy = TaskPolicy {
    max_duration_ms: 60_000,  // 1 minute - safe default
    permissions: TaskPermissions {
        allow_screenshot: false,
        allow_export_cookies: false,
        allow_import_cookies: false,
        allow_export_session: false,
        allow_import_session: false,
        allow_session_clipboard: false,
        allow_read_data: false,
        allow_write_data: false,
    },
};
```

**Rationale:** All permissions default to `false`. Tasks must explicitly request each capability they need.

---

## Task Policy Code Examples

### 1. CookieBot Task Policy

File: `src/task/cookiebot.rs`

```rust
use crate::task::policy::{TaskPolicy, TaskPermissions};

/// CookieBot policy - handles cookie consent dialogs
/// Needs: allow_export_cookies (verify consent), allow_screenshot (debug)
pub const COOKIEBOT_POLICY: TaskPolicy = TaskPolicy {
    max_duration_ms: 30_000,  // 30 seconds max for consent handling
    permissions: TaskPermissions {
        allow_export_cookies: true,   // Export to verify consent state
        allow_screenshot: true,       // Capture consent dialog for debugging
        // allow_write_data implied by allow_screenshot
        ..Default::default()
    },
};

// Task entry point
pub async fn run(ctx: &TaskContext, payload: Value) -> Result<TaskResult> {
    // Policy already validated at orchestrator level
    // Permission checks happen automatically in ctx methods
    
    let url = payload["url"].as_str().unwrap_or("");
    ctx.goto(url).await?;
    
    // This call checks allow_screenshot permission internally
    let screenshot = ctx.screenshot().await?;
    
    // This call checks allow_export_cookies permission internally
    let cookies = ctx.export_cookies(url).await?;
    
    // Handle consent dialog...
    Ok(TaskResult::success())
}
```

---

### 2. PageView Task Policy

File: `src/task/pageview.rs`

```rust
use crate::task::policy::{TaskPolicy, TaskPermissions};

/// PageView policy - simple page loading with verification
/// Needs: allow_screenshot (verify page loaded)
pub const PAGEVIEW_POLICY: TaskPolicy = TaskPolicy {
    max_duration_ms: 30_000,  // Page load timeout
    permissions: TaskPermissions {
        allow_screenshot: true,       // Verify page loaded correctly
        // allow_write_data implied by allow_screenshot
        ..Default::default()
    },
};

pub async fn run(ctx: &TaskContext, payload: Value) -> Result<TaskResult> {
    let url = payload["url"].as_str().unwrap_or("");
    
    ctx.goto(url).await?;
    ctx.pause(2000).await; // Wait for load
    
    // Verify with allow_screenshot
    let img = ctx.screenshot().await?;
    let path = format!("data/screenshots/{}.png", ctx.task_id);
    std::fs::write(&path, img)?;  // allow_write_data implied
    
    Ok(TaskResult::success())
}
```

---

### 3. TwitterActivity Task Policy

File: `src/task/twitteractivity.rs`

```rust
use crate::task::policy::{TaskPolicy, TaskPermissions};

/// TwitterActivity policy - complex social media automation
/// Needs: allow_export_cookies, allow_session_clipboard, allow_read_data, allow_screenshot
pub const TWITTERACTIVITY_POLICY: TaskPolicy = TaskPolicy {
    max_duration_ms: 300_000,  // 5 minutes for feed scanning
    permissions: TaskPermissions {
        allow_export_cookies: true,       // Verify login session
        allow_session_clipboard: true,     // Copy tweet text, paste replies
        allow_read_data: true,            // Read persona files from config/
        allow_screenshot: true,           // Debug screenshots
        // allow_write_data implied by allow_screenshot
        ..Default::default()
    },
};

pub async fn run(ctx: &TaskContext, payload: Value) -> Result<TaskResult> {
    // Verify login via cookies
    let cookies = ctx.export_cookies("https://twitter.com").await?;
    if !has_auth_cookie(&cookies) {
        return Err(TaskError::NotAuthenticated);
    }
    
    // Read persona config
    let persona_json = ctx.read_data_file("config/twitter_persona.json").await?;
    let persona: Persona = serde_json::from_str(&persona_json)?;
    
    // Interact with tweets
    for tweet in scan_feed(ctx).await? {
        ctx.click(tweet.selector).await?;
        
        // Copy tweet text to clipboard
        let text = ctx.text(tweet.text_selector).await?;
        ctx.write_clipboard(&text).await?;  // allow_session_clipboard check
        
        // Generate reply using LLM...
        let reply = generate_reply(&text, &persona).await?;
        ctx.write_clipboard(&reply).await?;
        ctx.paste().await?;
    }
    
    Ok(TaskResult::success())
}
```

---

### 4. Data Typing Task Policy

File: `src/task/data_typing.rs`

```rust
use crate::task::policy::{TaskPolicy, TaskPermissions};

/// DataTyping policy - keyboard automation with clipboard
/// Needs: allow_session_clipboard (paste), allow_read_data (load text)
pub const DATATYPING_POLICY: TaskPolicy = TaskPolicy {
    max_duration_ms: 60_000,
    permissions: TaskPermissions {
        allow_session_clipboard: true,  // Paste from clipboard
        allow_read_data: true,          // Load text files from data/ folder
        ..Default::default()
    },
};

pub async fn run(ctx: &TaskContext, payload: Value) -> Result<TaskResult> {
    let file_path = payload["file"].as_str()
        .ok_or(TaskError::MissingPayload("file"))?;
    let selector = payload["selector"].as_str()
        .ok_or(TaskError::MissingPayload("selector"))?;
    
    // Read data file
    let content = ctx.read_data_file(file_path).await?;
    
    // Copy to clipboard and paste
    ctx.write_clipboard(&content).await?;
    ctx.click(selector).await?;
    ctx.paste().await?;
    
    Ok(TaskResult::success())
}
```

---

### 5. Session Manager Task Policy (New Task)

File: `src/task/session_manager.rs`

```rust
use crate::task::policy::{TaskPolicy, TaskPermissions};

/// SessionManager policy - full session import/export
/// Needs: allow_export_cookies, allow_import_cookies, allow_export_session, allow_import_session, allow_read_data, allow_write_data
pub const SESSIONMANAGER_POLICY: TaskPolicy = TaskPolicy {
    max_duration_ms: 120_000,
    permissions: TaskPermissions {
        allow_export_cookies: true,   // Export cookies
        allow_import_cookies: true,   // Import cookies
        allow_export_session: true,   // Export full session
        allow_import_session: true,   // Import full session
        allow_read_data: true,        // Load session files
        allow_write_data: true,       // Save session files
        allow_screenshot: true,     // Debug session state
        // allow_write_data also implied by allow_screenshot
        ..Default::default()
    },
};

pub async fn run(ctx: &TaskContext, payload: Value) -> Result<TaskResult> {
    let action = payload["action"].as_str().unwrap_or("export");
    let site = payload["site"].as_str().unwrap_or("");
    let filename = payload["filename"].as_str().unwrap_or("session.json");
    
    match action {
        "export" => {
            // Export full session
            let session = ctx.export_session(site).await?;
            let json = serde_json::to_string_pretty(&session)?;
            
            // Save to file (write_data permission)
            let path = format!("data/sessions/{}", filename);
            std::fs::write(&path, json)?;
            
            log::info!("Exported session to {}", path);
        }
        "import" => {
            // Load session file (read_data permission)
            let path = format!("data/sessions/{}", filename);
            let json = std::fs::read_to_string(&path)?;
            let session: SessionData = serde_json::from_str(&json)?;
            
            // Restore session
            ctx.import_session(&session).await?;
            
            log::info!("Imported session from {}", path);
        }
        _ => return Err(TaskError::InvalidAction(action.to_string())),
    }
    
    Ok(TaskResult::success())
}
```

---

### 6. Simple Screenshot Task

File: `src/task/screenshot.rs`

```rust
use crate::task::policy::{TaskPolicy, TaskPermissions};

/// ScreenshotTask policy - page capture
/// Needs: allow_screenshot (implies allow_write_data)
pub const SCREENSHOTTASK_POLICY: TaskPolicy = TaskPolicy {
    max_duration_ms: 30_000,
    permissions: TaskPermissions {
        allow_screenshot: true,   // Capture and save screenshot
        // allow_write_data is IMPLIED by allow_screenshot
        // TaskContext::screenshot() handles both capture AND save
        ..Default::default()
    },
};

pub async fn run(ctx: &TaskContext, payload: Value) -> Result<TaskResult> {
    let url = payload["url"].as_str().unwrap_or("");
    let output = payload["output"].as_str().unwrap_or("data/screenshot.png");
    
    ctx.goto(url).await?;
    
    // screenshot() captures AND saves (uses implied allow_write_data)
    let path = ctx.screenshot_to_file(output).await?;
    
    Ok(TaskResult::success().with_output("path", path))
}
```

---

## Policy with Implied Permissions

Some permissions imply others. The orchestrator **automatically grants** implied permissions:

```rust
impl TaskPolicy {
    /// Get effective permissions (including implied)
    pub fn effective_permissions(&self) -> TaskPermissions {
        let mut perms = self.permissions.clone();
        
        // allow_screenshot implies allow_write_data (must save the image)
        if perms.allow_screenshot {
            perms.allow_write_data = true;
        }
        
        // allow_export_session implies allow_export_cookies (uses same CDP call)
        if perms.allow_export_session {
            perms.allow_export_cookies = true;
        }
        
        // allow_import_session implies allow_import_cookies
        if perms.allow_import_session {
            perms.allow_import_cookies = true;
        }
        
        perms
    }
}
```

**Usage in TaskContext:**
```rust
impl TaskContext {
    pub async fn screenshot_to_file(&self, path: &str) -> Result<String> {
        // Use effective permissions (includes implied ones)
        let perms = self.policy.effective_permissions();
        
        if !perms.allow_screenshot {
            return Err(TaskError::PermissionDenied("allow_screenshot"));
        }
        
        // allow_screenshot implies allow_write_data, so this is allowed
        let image = self.capture_screenshot().await?;
        std::fs::write(path, image)?;
        
        Ok(path.to_string())
    }
}
```

---

## Enforcement Points

### 1. Orchestrator: Hard Timeout

```rust
// src/orchestrator.rs

pub async fn execute_task(
    task: &TaskDefinition,
    policy: &TaskPolicy,
    ctx: &TaskContext,
) -> Result<TaskResult> {
    let timeout_duration = Duration::from_millis(policy.max_duration_ms);
    
    match tokio::time::timeout(timeout_duration, execute(task, ctx)).await {
        Ok(result) => result,
        Err(_) => {
            log::error!("Task '{}' killed after exceeding {}ms limit", task.name, policy.max_duration_ms);
            Err(TaskError::Timeout(policy.max_duration_ms))
        }
    }
}
```

### 2. TaskContext: Permission Checks

```rust
impl TaskContext {
    pub async fn screenshot(&self) -> Result<Vec<u8>> {
        if !self.policy.permissions.allow_screenshot {
            return Err(TaskError::PermissionDenied("allow_screenshot"));
        }
        // CDP: Page.captureScreenshot
    }
    
    pub async fn export_cookies(&self, url: &str) -> Result<Vec<Cookie>> {
        if !self.policy.permissions.allow_export_cookies {
            return Err(TaskError::PermissionDenied("allow_export_cookies"));
        }
        // CDP: Network.getCookies
    }
    
    pub async fn import_cookies(&self, cookies: &[Cookie]) -> Result<()> {
        if !self.policy.permissions.allow_import_cookies {
            return Err(TaskError::PermissionDenied("allow_import_cookies"));
        }
        // CDP: Network.setCookie for each
    }
    
    pub async fn export_session(&self, url: &str) -> Result<SessionData> {
        if !self.policy.permissions.allow_export_session {
            return Err(TaskError::PermissionDenied("allow_export_session"));
        }
        // CDP: Network.getCookies + Runtime.evaluate (localStorage)
        log::warn!("Task '{}' exported full session from {}", self.task_name, url);
    }
    
    pub async fn import_session(&self, session_data: &SessionData) -> Result<()> {
        if !self.policy.permissions.allow_import_session {
            return Err(TaskError::PermissionDenied("allow_import_session"));
        }
        // CDP: Network.setCookies + Runtime.evaluate (localStorage restore)
        log::warn!("Task '{}' imported full session", self.task_name);
    }
    
    pub fn read_clipboard(&self) -> Result<String> {
        if !self.policy.permissions.allow_session_clipboard {
            return Err(TaskError::PermissionDenied("allow_session_clipboard"));
        }
        // OS clipboard read
    }
    
    pub fn write_clipboard(&self, text: &str) -> Result<()> {
        if !self.policy.permissions.allow_session_clipboard {
            return Err(TaskError::PermissionDenied("allow_session_clipboard"));
        }
        // OS clipboard write
    }
    
    pub async fn read_data_file(&self, path: &str) -> Result<String> {
        if !self.policy.permissions.allow_read_data {
            return Err(TaskError::PermissionDenied("allow_read_data"));
        }
        // Validate path is within config/ or data/
        // Standard filesystem read
    }
    
    pub async fn write_data_file(&self, path: &str, content: &[u8]) -> Result<()> {
        if !self.policy.permissions.allow_write_data {
            return Err(TaskError::PermissionDenied("allow_write_data"));
        }
        // Validate path is within config/ or data/
        // Standard filesystem write
    }
}
```

---

## CDP Feature Implementation Guide

### export_cookies (Network.getCookies)

```rust
use chromiumoxide::cdp::browser_protocol::network::GetCookiesParams;

async fn export_cookies(page: &Page) -> Result<Vec<Cookie>> {
    let cookies = page.execute(GetCookiesParams::default()).await?;
    Ok(cookies.cookies)
}
```

### import_cookies (Network.setCookie)

```rust
use chromiumoxide::cdp::browser_protocol::network::SetCookieParams;

async fn import_cookies(page: &Page, cookies: &[Cookie]) -> Result<()> {
    for cookie in cookies {
        let params = SetCookieParams::builder()
            .name(&cookie.name)
            .value(&cookie.value)
            .domain(&cookie.domain)
            .path(&cookie.path)
            .build()?;
        
        page.execute(params).await?;
    }
    Ok(())
}
```

### export_session (Network.getCookies + Runtime.evaluate)

```rust
async fn export_session(page: &Page) -> Result<SessionData> {
    // Get cookies via CDP
    let cookies = export_cookies(page).await?;
    
    // Get localStorage via JavaScript
    let local_storage_js = r#"
        (function() {
            let data = {};
            for (let i = 0; i < localStorage.length; i++) {
                let key = localStorage.key(i);
                data[key] = localStorage.getItem(key);
            }
            return JSON.stringify(data);
        })()
    "#;
    
    let local_storage_json: String = page.evaluate(local_storage_js).await?;
    let local_storage: Value = serde_json::from_str(&local_storage_json)?;
    
    Ok(SessionData {
        cookies,
        local_storage,
    })
}
```

### import_session (Network.setCookie + Runtime.evaluate)

```rust
async fn import_session(page: &Page, session_data: &SessionData) -> Result<()> {
    // Set cookies via CDP
    import_cookies(page, &session_data.cookies).await?;
    
    // Set localStorage via JavaScript
    let local_storage_js = format!(
        r#"
        (function() {{
            let data = {};
            Object.entries(data).forEach(([key, value]) => {{
                localStorage.setItem(key, value);
            }});
            return 'localStorage restored';
        }})()
        "#,
        serde_json::to_string(&session_data.local_storage)?
    );
    
    page.evaluate(local_storage_js).await?;
    
    Ok(())
}
```

### screenshot (Page.captureScreenshot)

```rust
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotParams;

async fn screenshot(page: &Page) -> Result<Vec<u8>> {
    let params = CaptureScreenshotParams::default();
    let screenshot = page.screenshot(params).await?;
    Ok(screenshot)
}
```

---

## Permission Denial Behavior

When a task tries to use a capability without permission:

1. **Return error** - `TaskError::PermissionDenied("allow_screenshot")`
2. **Log warning** - "Task 'cookiebot' attempted allow_screenshot without permission"
3. **Continue execution** - Task can handle error or fail gracefully

---

## Validation Flow

```
Task Registration
      ↓
[Check policy exists]
      ↓
No policy? → WARN: "Task X missing policy, using default (60s timeout)"
      ↓
[Validate max_duration_ms > 0]
      ↓
Zero? → ERROR: "max_duration_ms must be > 0"
      ↓
[Register task with policy]
      ↓
Execution time:
      ↓
[Enforce timeout]
      ↓
[Gate each privileged operation]
```

---

## Migration Strategy

### Phase 1: Infrastructure (2 hours)
1. Create `TaskPolicy` and `TaskPermissions` structs (8 permissions)
2. Create `DEFAULT_TASK_POLICY`
3. Add policy registry
4. Add timeout enforcement in orchestrator
5. Add implied permissions logic

### Phase 2: Add Policies to Tasks (1 hour)
Add `*_POLICY` constant to each task file:

```rust
pub const TASKNAME_POLICY: TaskPolicy = TaskPolicy {
    max_duration_ms: 60_000,
    ..DEFAULT_TASK_POLICY
};
```

### Phase 3: Permission Gates (3 hours)
Wrap sensitive operations in `TaskContext`:
- Screenshot methods
- Cookie export/import
- Session export/import (with audit logging)
- Clipboard access
- Data file read/write

---

## Summary

| Policy | Type | Purpose | Security Risk | Implies |
|--------|------|---------|---------------|---------|
| `max_duration_ms` | `u64` | **Mandatory** - Kill task if it hangs | None | - |
| `allow_screenshot` | `bool` | Visual debugging & evidence | May capture sensitive data | `allow_write_data` |
| `allow_export_cookies` | `bool` | Export browser cookies | Low | - |
| `allow_import_cookies` | `bool` | Import cookies into browser | Medium | - |
| `allow_export_session` | `bool` | Export cookies + localStorage | **HIGH** | `allow_export_cookies` |
| `allow_import_session` | `bool` | Import cookies + localStorage | **HIGH** | `allow_import_cookies` |
| `allow_session_clipboard` | `bool` | Read/write system clipboard | Medium | - |
| `allow_read_data` | `bool` | Read files from config/data | Low | - |
| `allow_write_data` | `bool` | Write files to config/data | Path-restricted | - |

**Permission Dependencies:**
- `allow_screenshot` → `allow_write_data` (must save images)
- `allow_export_session` → `allow_export_cookies` (uses same CDP call)
- `allow_import_session` → `allow_import_cookies` (uses same CDP call)

---

## Benefits

1. **Simplicity** - One mandatory rule, 8 simple permissions (9 total)
2. **Security** - Tasks can't overreach without explicit permission
3. **Granularity** - Session import/export are separate permissions
4. **Implied permissions** - allow_screenshot automatically allows saving images
5. **Session management** - Full import/export capabilities for advanced use cases
6. **Malware prevention** - Unauthorized access blocked
7. **Debugging** - Clear errors when permissions missing
8. **Easy migration** - Default policy works for most tasks

---

## Implementation Checklist

### Phase 1: Core Data Structures
- [ ] Create `TaskPolicy` struct with `max_duration_ms` + `permissions`
- [ ] Create `TaskPermissions` struct with 8 bool flags
- [ ] Define `DEFAULT_TASK_POLICY`
- [ ] Create policy registry (task name → policy lookup)

### Phase 2: Runtime Enforcement
- [ ] Add timeout enforcement in orchestrator (`tokio::time::timeout`)
- [ ] Add permission check methods to `TaskContext` (return errors if denied)
- [ ] Add implied permissions logic (`allow_screenshot` → `allow_write_data`, `allow_export_session` → `allow_export_cookies`, etc.)

### Phase 3: Feature Implementation
- [ ] Add path validation for `allow_read_data` and `allow_write_data` (config/ and data/ only)
- [ ] Add CDP implementations for all 7 features (allow_screenshot, allow_export_cookies, allow_export_session, allow_session_clipboard)
- [ ] Add audit logging for session export/import operations

### Phase 4: Application & Testing
- [ ] Add policies to all task files (`*_POLICY` constants)
- [ ] Add tests for timeout enforcement
- [ ] Add tests for permission denial
