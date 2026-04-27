# API Usage Guide

Practical recipes and patterns for using the v0.0.3 TaskContext APIs.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Cookie Management Recipes](#cookie-management-recipes)
3. [Session Management Recipes](#session-management-recipes)
4. [Data File Patterns](#data-file-patterns)
5. [Network Integration](#network-integration)
6. [DOM Inspection Workflows](#dom-inspection-workflows)
7. [Browser State Management](#browser-state-management)
8. [Error Handling Patterns](#error-handling-patterns)

---

## Getting Started

### Basic TaskContext Usage

```rust
use auto::runtime::TaskContext;
use anyhow::Result;

async fn my_task(ctx: &TaskContext) -> Result<()> {
    // All APIs are available through the context
    let cookies = ctx.export_cookies_for_domain("example.com").await?;
    println!("Found {} cookies", cookies.len());
    Ok(())
}
```

---

## Cookie Management Recipes

### Check if User is Logged In

```rust
use anyhow::Result;

async fn check_login_status(ctx: &auto::runtime::TaskContext) -> Result<bool> {
    // Look for session cookie
    let has_session = ctx.has_cookie("session_id", Some("example.com")).await?;
    
    // Also check for auth token
    let has_token = ctx.has_cookie("auth_token", None).await?;
    
    Ok(has_session || has_token)
}
```

### Export Cookies for Login Persistence

```rust
use anyhow::Result;
use serde_json::Value;

async fn save_login_session(ctx: &auto::runtime::TaskContext) -> Result<Vec<Value>> {
    // Export cookies for the auth domain
    let cookies = ctx.export_cookies_for_domain("api.example.com").await?;
    
    // Filter for session-related cookies only
    let session_cookies: Vec<_> = cookies.into_iter()
        .filter(|c| {
            let name = c.get("name").and_then(|n| n.as_str());
            matches!(name, Some("session_id") | Some("auth_token") | Some("refresh_token"))
        })
        .collect();
    
    println!("Saving {} session cookies", session_cookies.len());
    Ok(session_cookies)
}
```

### Validate Session Before Action

```rust
use anyhow::Result;

async fn perform_authenticated_action(ctx: &auto::runtime::TaskContext) -> Result<()> {
    // Check for required cookie before proceeding
    if !ctx.has_cookie("api_key", Some("api.example.com")).await? {
        return Err(anyhow::anyhow!("Not authenticated - please login first"));
    }
    
    // Proceed with authenticated action
    println!("Performing authenticated action...");
    Ok(())
}
```

---

## Session Management Recipes

### Save and Restore localStorage State

```rust
use anyhow::Result;
use std::collections::HashMap;

async fn backup_local_storage(ctx: &auto::runtime::TaskContext) -> Result<HashMap<String, String>> {
    // Export current localStorage
    let storage = ctx.export_local_storage("").await?;
    
    // Save to file for later restoration
    let json = serde_json::to_string(&storage)?;
    std::fs::write("backup/storage.json", json)?;
    
    println!("Backed up {} localStorage entries", storage.len());
    Ok(storage)
}

async fn restore_local_storage(ctx: &auto::runtime::TaskContext) -> Result<()> {
    // Load from backup file
    let json = std::fs::read_to_string("backup/storage.json")?;
    let storage: HashMap<String, String> = serde_json::from_str(&json)?;
    
    // Restore to browser
    ctx.import_local_storage("", &storage).await?;
    println!("Restored {} localStorage entries", storage.len());
    Ok(())
}
```

### Transfer Session Between Pages

```rust
use anyhow::Result;
use auto::task::policy::SessionData;
use std::collections::HashMap;

async fn transfer_session(
    ctx: &auto::runtime::TaskContext,
    from_url: &str,
    to_url: &str,
) -> Result<()> {
    // Navigate to source page
    ctx.page.goto(from_url).await?;
    
    // Export session data
    let cookies = ctx.export_cookies_for_domain("example.com").await?;
    let local_storage = ctx.export_local_storage("").await?;
    
    let session_data = SessionData {
        cookies,
        local_storage,
        exported_at: chrono::Utc::now(),
        url: from_url.to_string(),
    };
    
    // Validate before transferring
    let warnings = ctx.validate_session_data(&session_data)?;
    if !warnings.is_empty() {
        println!("Session validation warnings: {:?}", warnings);
    }
    
    // Navigate to destination and restore
    ctx.page.goto(to_url).await?;
    ctx.import_local_storage("", &session_data.local_storage).await?;
    
    println!("Session transferred from {} to {}", from_url, to_url);
    Ok(())
}
```

---

## Data File Patterns

### Reading Configuration Files

```rust
use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize)]
struct AppConfig {
    api_key: String,
    timeout_seconds: u64,
    retry_count: u32,
}

fn load_config(ctx: &auto::runtime::TaskContext) -> Result<AppConfig> {
    // Check if config exists
    if !ctx.data_file_exists("config/app.json")? {
        return Err(anyhow::anyhow!("Configuration file not found"));
    }
    
    // Read and parse config
    let config: AppConfig = ctx.read_json_data("config/app.json")?;
    println!("Loaded config with timeout: {}s", config.timeout_seconds);
    
    Ok(config)
}
```

### Writing Results Data

```rust
use anyhow::Result;
use serde::Serialize;
use chrono::Utc;

#[derive(Serialize)]
struct TaskResult {
    timestamp: String,
    success: bool,
    items_processed: usize,
    errors: Vec<String>,
}

fn save_results(
    ctx: &auto::runtime::TaskContext,
    items: usize,
    errors: Vec<String>,
) -> Result<()> {
    let result = TaskResult {
        timestamp: Utc::now().to_rfc3339(),
        success: errors.is_empty(),
        items_processed: items,
        errors,
    };
    
    // Write with timestamped filename
    let filename = format!(
        "results/run_{}.json",
        Utc::now().format("%Y%m%d_%H%M%S")
    );
    
    ctx.write_json_data(&filename, &result)?;
    println!("Results saved to {}", filename);
    
    Ok(())
}
```

### Appending to Log Files

```rust
use anyhow::Result;
use chrono::Utc;

fn log_activity(ctx: &auto::runtime::TaskContext, action: &str) -> Result<()> {
    let entry = format!(
        "[{}] {}\n",
        Utc::now().format("%Y-%m-%d %H:%M:%S"),
        action
    );
    
    ctx.append_data_file("logs/activity.log", entry.as_bytes())?;
    Ok(())
}

fn log_error(ctx: &auto::runtime::TaskContext, error: &str) -> Result<()> {
    let entry = format!(
        "[{}] ERROR: {}\n",
        Utc::now().format("%Y-%m-%d %H:%M:%S"),
        error
    );
    
    ctx.append_data_file("logs/errors.log", entry.as_bytes())?;
    Ok(())
}
```

### Managing Data Directories

```rust
use anyhow::Result;

fn cleanup_old_exports(ctx: &auto::runtime::TaskContext, keep_count: usize) -> Result<()> {
    // List all files in exports directory
    let mut files = ctx.list_data_files(Some("exports"))?;
    
    // Sort by name (assuming timestamp prefix)
    files.sort();
    
    // Delete oldest files if exceeding keep_count
    if files.len() > keep_count {
        for old_file in &files[..files.len() - keep_count] {
            let path = format!("exports/{}", old_file);
            ctx.delete_data_file(&path)?;
            println!("Deleted old export: {}", old_file);
        }
    }
    
    Ok(())
}
```

---

## Network Integration

### Calling REST APIs

```rust
use anyhow::Result;

async fn fetch_user_data(ctx: &auto::runtime::TaskContext, user_id: &str) -> Result<serde_json::Value> {
    let url = format!("https://api.example.com/users/{}", user_id);
    let response = ctx.http_get(&url).await?;
    
    if response.status == 200 {
        let data: serde_json::Value = serde_json::from_str(&response.body)?;
        Ok(data)
    } else {
        Err(anyhow::anyhow!(
            "API request failed: {} - {}",
            response.status,
            response.body
        ))
    }
}
```

### POST with JSON Payload

```rust
use anyhow::Result;

async fn create_record(ctx: &auto::runtime::TaskContext) -> Result<String> {
    let payload = serde_json::json!({
        "name": "New Record",
        "type": "task",
        "priority": "high",
        "created_at": chrono::Utc::now().to_rfc3339(),
    });
    
    let response = ctx
        .http_post_json("https://api.example.com/records", &payload)
        .await?;
    
    match response.status {
        201 => {
            let data: serde_json::Value = serde_json::from_str(&response.body)?;
            let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
            println!("Created record with ID: {}", id);
            Ok(id.to_string())
        }
        400 => Err(anyhow::anyhow!("Invalid request data")),
        401 => Err(anyhow::anyhow!("Authentication required")),
        _ => Err(anyhow::anyhow!("API error: {}", response.status)),
    }
}
```

### Downloading Files

```rust
use anyhow::Result;

async fn download_report(
    ctx: &auto::runtime::TaskContext,
    report_id: &str,
) -> Result<String> {
    let url = format!("https://api.example.com/reports/{}/download", report_id);
    let filename = format!("reports/report_{}.pdf", report_id);
    
    // Ensure directory exists
    if !ctx.data_file_exists("reports")? {
        // Create directory via write
        ctx.write_json_data("reports/.gitkeep", &serde_json::json!({}))?;
    }
    
    // Download file
    let bytes = ctx.download_file(&url, &filename).await?;
    println!("Downloaded {} bytes to {}", bytes, filename);
    
    Ok(filename)
}
```

---

## DOM Inspection Workflows

### Get Element Styles

```rust
use anyhow::Result;

async fn check_element_visibility(ctx: &auto::runtime::TaskContext, selector: &str) -> Result<bool> {
    // Get display property
    let display = ctx.get_computed_style(selector, "display").await?;
    if display == "none" {
        return Ok(false);
    }
    
    // Check visibility
    let visibility = ctx.get_computed_style(selector, "visibility").await?;
    if visibility == "hidden" {
        return Ok(false);
    }
    
    // Check opacity
    let opacity = ctx.get_computed_style(selector, "opacity").await?;
    if opacity == "0" {
        return Ok(false);
    }
    
    Ok(true)
}
```

### Get Element Position and Size

```rust
use anyhow::Result;

async fn get_element_info(
    ctx: &auto::runtime::TaskContext,
    selector: &str,
) -> Result<(f64, f64, f64, f64)> {
    let rect = ctx.get_element_rect(selector).await?;
    
    println!("Element position: ({}, {})", rect.x, rect.y);
    println!("Element size: {} x {}", rect.width, rect.height);
    
    // Check if element is in viewport
    let viewport_width = 1920.0; // Or get from page
    let viewport_height = 1080.0;
    
    let in_viewport = rect.x >= 0.0
        && rect.y >= 0.0
        && rect.x + rect.width <= viewport_width
        && rect.y + rect.height <= viewport_height;
    
    println!("In viewport: {}", in_viewport);
    
    Ok((rect.x, rect.y, rect.width, rect.height))
}
```

### Count Elements for Validation

```rust
use anyhow::Result;

async fn validate_page_state(ctx: &auto::runtime::TaskContext) -> Result<bool> {
    // Check if expected elements are present
    let product_count = ctx.count_elements(".product-item").await?;
    if product_count == 0 {
        println!("No products found on page");
        return Ok(false);
    }
    
    // Verify error messages are not present
    let error_count = ctx.count_elements(".error-message").await?;
    if error_count > 0 {
        println!("Found {} error messages", error_count);
        return Ok(false);
    }
    
    // Check pagination exists if needed
    if product_count >= 20 {
        let has_pagination = ctx.count_elements(".pagination").await? > 0;
        if !has_pagination {
            println!("Warning: Many products but no pagination");
        }
    }
    
    Ok(true)
}
```

### Check and Scroll to Element

```rust
use anyhow::Result;

async fn ensure_element_visible(
    ctx: &auto::runtime::TaskContext,
    selector: &str,
) -> Result<()> {
    // Check if element is in viewport
    if !ctx.is_in_viewport(selector).await? {
        println!("Element not visible, scrolling...");
        ctx.scroll_to(selector).await?;
        
        // Wait a moment for scroll to complete
        ctx.pause(500).await;
        
        // Verify visibility after scroll
        if !ctx.is_in_viewport(selector).await? {
            return Err(anyhow::anyhow!("Element still not visible after scroll"));
        }
    }
    
    println!("Element is now visible");
    Ok(())
}
```

---

## Browser State Management

### Complete Export for Testing

```rust
use anyhow::Result;

async fn create_test_snapshot(
    ctx: &auto::runtime::TaskContext,
    name: &str,
) -> Result<()> {
    // Export complete browser state
    let browser_data = ctx.export_browser("https://example.com").await?;
    
    println!("Exported {} cookies", browser_data.cookies.len());
    println!("Local storage origins: {}", browser_data.local_storage.len());
    println!("Session storage origins: {}", browser_data.session_storage.len());
    println!("IndexedDB databases: {}", browser_data.indexeddb_names.len());
    
    // Save to file with timestamp
    let filename = format!(
        "snapshots/{}_{}.json",
        name,
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    
    ctx.write_json_data(&filename, &browser_data)?;
    println!("Snapshot saved to {}", filename);
    
    Ok(())
}
```

### Restore State for Testing

```rust
use anyhow::Result;

async fn restore_test_snapshot(
    ctx: &auto::runtime::TaskContext,
    snapshot_path: &str,
) -> Result<()> {
    // Load snapshot from file
    let browser_data: auto::task::policy::BrowserData = 
        ctx.read_json_data(snapshot_path)?;
    
    println!(
        "Restoring snapshot from {} (exported at {})",
        browser_data.source,
        browser_data.exported_at
    );
    
    // Restore browser state
    ctx.import_browser(&browser_data).await?;
    
    // Navigate to original URL
    ctx.page.goto(&browser_data.source).await?;
    
    println!("Browser state restored successfully");
    Ok(())
}
```

---

## Error Handling Patterns

### Permission Error Handling

```rust
use anyhow::Result;

async fn safe_export_cookies(ctx: &auto::runtime::TaskContext) -> Result<Vec<serde_json::Value>> {
    match ctx.export_cookies_for_domain("example.com").await {
        Ok(cookies) => Ok(cookies),
        Err(e) if e.to_string().contains("Permission denied") => {
            eprintln!("Warning: No permission to export cookies. Check task policy.");
            Ok(vec![]) // Return empty instead of failing
        }
        Err(e) => Err(e),
    }
}
```

### Graceful Degradation

```rust
use anyhow::Result;

async fn get_user_preferences(ctx: &auto::runtime::TaskContext) -> Result<serde_json::Value> {
    // Try localStorage first
    if let Ok(storage) = ctx.export_local_storage("").await {
        if let Some(prefs) = storage.get("user_prefs") {
            return Ok(serde_json::json!({"source": "localStorage", "data": prefs}));
        }
    }
    
    // Fall back to API call
    if let Ok(response) = ctx.http_get("https://api.example.com/user/prefs").await {
        if response.status == 200 {
            let data: serde_json::Value = serde_json::from_str(&response.body)?;
            return Ok(serde_json::json!({"source": "api", "data": data}));
        }
    }
    
    // Final fallback: default values
    Ok(serde_json::json!({
        "source": "default",
        "data": {"theme": "light", "language": "en"}
    }))
}
```

### Retry Pattern

```rust
use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;

async fn download_with_retry(
    ctx: &auto::runtime::TaskContext,
    url: &str,
    path: &str,
    max_retries: u32,
) -> Result<u64> {
    let mut last_error = None;
    
    for attempt in 1..=max_retries {
        match ctx.download_file(url, path).await {
            Ok(bytes) => {
                println!("Downloaded {} bytes on attempt {}", bytes, attempt);
                return Ok(bytes);
            }
            Err(e) => {
                println!("Attempt {} failed: {}", attempt, e);
                last_error = Some(e);
                
                if attempt < max_retries {
                    let delay = Duration::from_secs(2_u64.pow(attempt)); // Exponential backoff
                    println!("Waiting {:?} before retry...", delay);
                    sleep(delay).await;
                }
            }
        }
    }
    
    Err(anyhow::anyhow!(
        "Failed after {} attempts: {:?}",
        max_retries,
        last_error
    ))
}
```

---

## See Also

- [API Reference](API_REFERENCE.md) - Complete API documentation
- [Task Authoring Guide](TASK_AUTHORING_GUIDE.md) - Building custom tasks
- [rustdoc](https://docs.rs) - Generated API docs with examples
