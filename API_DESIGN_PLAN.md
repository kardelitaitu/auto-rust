# API Design Plan: Permission-Gated Task Context Methods

**Document Purpose**: Define consistent, discoverable API methods for all permission-gated operations in TaskContext.

**Target Version**: v0.0.3
**Status**: Draft

## Quick Reference: Planned API Methods

### Current APIs (v0.0.2)
- `api.screenshot()` - Capture WebP screenshot at 50% quality
- `api.screenshot_with_quality(q)` - Screenshot with custom quality (1-100)
- `api.export_cookies(url)` - Export cookies for URL
- `api.import_cookies(cookies)` - Import cookies into browser
- `api.export_session(url)` - Export cookies + localStorage
- `api.import_session(data)` - Import cookies + localStorage
- `api.read_clipboard()` - Read system clipboard
- `api.write_clipboard(text)` - Write to system clipboard
- `api.read_data_file(path)` - Read file from data/ or config/
- `api.write_data_file(path, content)` - Write file to data/ or config/

### Planned APIs (v0.0.3)

**Cookie Management**
- `api.export_cookies_for_domain(domain)` - Export cookies matching domain
- `api.export_session_cookies(url)` - Export only session cookies
- `api.clear_cookies_for_domain(domain)` - Clear cookies for domain
- `api.has_cookie(name, domain)` - Check if specific cookie exists

**Session Management**
- `api.export_local_storage(url)` - Export only localStorage
- `api.import_local_storage(url, data)` - Import only localStorage
- `api.validate_session_data(data)` - Validate session without importing
- `api.is_session_valid(url)` - Check if session is fresh

**Clipboard Management**
- `api.clear_clipboard()` - Clear clipboard content
- `api.read_clipboard_with_timeout(ms)` - Wait for clipboard update
- `api.has_clipboard_content()` - Check if clipboard non-empty
- `api.append_clipboard(text, sep)` - Append text to clipboard

**Data File Management**
- `api.list_data_files(subdir)` - List files in data directory
- `api.append_data_file(path, content)` - Append to file
- `api.data_file_exists(path)` - Check if file exists
- `api.delete_data_file(path)` - Delete file
- `api.read_json_data(path)` - Read and parse JSON
- `api.write_json_data(path, data)` - Write JSON (pretty)
- `api.data_file_metadata(path)` - Get file size/modified time

**Network/HTTP** (new permission: `allow_http_requests`)
- `api.http_get(url)` - HTTP GET request
- `api.http_post_json(url, body)` - HTTP POST with JSON
- `api.download_file(url, path)` - Download to data directory

**DOM Inspection** (new permission: `allow_dom_inspection`)
- `api.get_computed_style(selector, property)` - Get CSS property
- `api.get_element_rect(selector)` - Get element position/size
- `api.get_scroll_position()` - Get page scroll position
- `api.count_elements(selector)` - Count matching elements
- `api.is_in_viewport(selector)` - Check if element visible

---

## Design Principles

1. **Verb-Noun Naming**: `action_resource()` (e.g., `export_cookies()`, `read_clipboard()`)
2. **Async for I/O**: All methods that touch browser, files, or network are `async`
3. **Sync for Memory**: Pure memory operations can be synchronous
4. **Explicit Permissions**: Every method documents its required permission
5. **Consistent Returns**: `Result<T>` with descriptive error messages
6. **Rust Idioms**: Use `&str` for inputs, owned types for outputs

---

## Current API State

| Permission | Current Method | Status | Issues |
|------------|---------------|--------|--------|
| `allow_screenshot` | `screenshot()` | ✅ Implemented | Good |
| `allow_screenshot` | `screenshot_with_quality(q)` | ✅ Implemented | Good |
| `allow_export_cookies` | `export_cookies(url)` | ✅ Implemented | Good |
| `allow_import_cookies` | `import_cookies(cookies)` | ✅ Implemented | Good |
| `allow_export_session` | `export_session(url)` | ✅ Implemented | Good |
| `allow_import_session` | `import_session(data)` | ✅ Implemented | Good |
| `allow_session_clipboard` | `read_clipboard()` | ✅ Implemented | Good |
| `allow_session_clipboard` | `write_clipboard(text)` | ✅ Implemented | Good |
| `allow_read_data` | `read_data_file(path)` | ✅ Implemented | Good |
| `allow_write_data` | `write_data_file(path, content)` | ✅ Implemented | Good |

---

## Proposed API Additions for v0.0.3

### 1. Cookie Management (Enhanced)

**Current**: Basic export/import
**Gap**: No bulk operations, no filtering

```rust
// Export cookies for specific domain
pub async fn export_cookies_for_domain(&self, domain: &str) -> Result<Vec<serde_json::Value>>
    requires: allow_export_cookies

// Export only session cookies (no persistent)
pub async fn export_session_cookies(&self, url: &str) -> Result<Vec<serde_json::Value>>
    requires: allow_export_cookies

// Clear all cookies for domain
pub async fn clear_cookies_for_domain(&self, domain: &str) -> Result<usize>  // returns count cleared
    requires: allow_export_cookies + allow_import_cookies (implied)

// Check if specific cookie exists
pub async fn has_cookie(&self, name: &str, domain: &str) -> Result<bool>
    requires: allow_export_cookies
```

### 2. Session Management (Enhanced)

**Current**: Full session export/import
**Gap**: No partial operations, no validation

```rust
// Export only localStorage (no cookies)
pub async fn export_local_storage(&self, url: &str) -> Result<HashMap<String, String>>
    requires: allow_export_session

// Import only localStorage
pub async fn import_local_storage(&self, url: &str, data: &HashMap<String, String>) -> Result<()>
    requires: allow_import_session

// Validate session data without importing
pub fn validate_session_data(&self, data: &SessionData) -> Result<Vec<String>>  // returns warnings
    requires: none (read-only validation)

// Check if session is "fresh" (not expired)
pub async fn is_session_valid(&self, url: &str) -> Result<bool>
    requires: allow_export_session
```

### 3. Clipboard Management (Enhanced)

**Current**: Basic read/write
**Gap**: No clear, no append, no format detection

```rust
// Clear clipboard
pub fn clear_clipboard(&self) -> Result<()>
    requires: allow_session_clipboard

// Read clipboard with timeout (wait for new content)
pub async fn read_clipboard_with_timeout(&self, timeout_ms: u64) -> Result<Option<String>>
    requires: allow_session_clipboard

// Check if clipboard has content (non-empty)
pub fn has_clipboard_content(&self) -> Result<bool>
    requires: allow_session_clipboard

// Append to clipboard (read + write combined)
pub fn append_clipboard(&self, text: &str, separator: &str) -> Result<()>
    requires: allow_session_clipboard
```

### 4. Data File Management (Enhanced)

**Current**: Read/write single files
**Gap**: No directory listing, no append, no JSON helpers

```rust
// List files in data directory (non-recursive)
pub fn list_data_files(&self, subdir: &str) -> Result<Vec<String>>
    requires: allow_read_data

// Append to file (create if not exists)
pub fn append_data_file(&self, relative_path: &str, content: &[u8]) -> Result<()>
    requires: allow_write_data

// Check if file exists
pub fn data_file_exists(&self, relative_path: &str) -> Result<bool>
    requires: allow_read_data

// Read and parse JSON
pub fn read_json_data<T: DeserializeOwned>(&self, relative_path: &str) -> Result<T>
    requires: allow_read_data

// Write JSON (pretty-printed)
pub fn write_json_data<T: Serialize>(&self, relative_path: &str, data: &T) -> Result<()>
    requires: allow_write_data

// Delete file
pub fn delete_data_file(&self, relative_path: &str) -> Result<()>
    requires: allow_write_data

// Get file metadata (size, modified time)
pub fn data_file_metadata(&self, relative_path: &str) -> Result<FileMetadata>
    requires: allow_read_data
```

### 5. New Permission: Network/HTTP (Proposed)

**Rationale**: Tasks often need to make HTTP requests
**Risk**: Could be abused for spam/DDoS
**Mitigation**: Separate permission, rate limiting

```rust
// New permission: allow_http_requests

// Simple GET request
pub async fn http_get(&self, url: &str) -> Result<HttpResponse>
    requires: allow_http_requests

// POST with JSON body
pub async fn http_post_json<T: Serialize>(&self, url: &str, body: &T) -> Result<HttpResponse>
    requires: allow_http_requests

// Download file to data directory
pub async fn download_file(&self, url: &str, relative_path: &str) -> Result<u64>  // bytes downloaded
    requires: allow_http_requests + allow_write_data (implied)
```

### 6. New Permission: DOM Inspection (Proposed)

**Rationale**: Advanced tasks need to read page state
**Current Gap**: Only `api.html()`, `api.text()` exist

```rust
// New permission: allow_dom_inspection

// Get computed CSS property for element
pub async fn get_computed_style(&self, selector: &str, property: &str) -> Result<String>
    requires: allow_dom_inspection

// Get element bounding box (position + size)
pub async fn get_element_rect(&self, selector: &str) -> Result<Rect>
    requires: allow_dom_inspection

// Get page scroll position
pub async fn get_scroll_position(&self) -> Result<(u32, u32)>  // (x, y)
    requires: allow_dom_inspection

// Count elements matching selector
pub async fn count_elements(&self, selector: &str) -> Result<usize>
    requires: allow_dom_inspection

// Check if element is in viewport
pub async fn is_in_viewport(&self, selector: &str) -> Result<bool>
    requires: allow_dom_inspection
```

---

## Method Naming Conventions

### Verbs (Actions)
- `export_*` - Extract data from browser/session
- `import_*` - Inject data into browser/session
- `read_*` - Get data (files, clipboard)
- `write_*` - Set data (files, clipboard)
- `clear_*` - Remove/delete data
- `check_*` / `has_*` / `is_*` - Boolean checks
- `get_*` - Retrieve computed values
- `download_*` - Network → file operations
- `append_*` - Add to existing data
- `validate_*` - Check without modifying

### Nouns (Resources)
- `*_cookies` - Browser cookies
- `*_session` - Cookies + localStorage
- `*_local_storage` - localStorage only
- `*_clipboard` - System clipboard
- `*_data_file` / `*_data` - File system (data/ config/)
- `*_storage` / `*_cache` - Future: IndexedDB, etc.

---

## Return Type Patterns

### Success Indicators
```rust
Result<()>              // Operation succeeded, no return value
Result<bool>            // Yes/no answer
Result<usize>           // Count of items affected
Result<String>          // Single text value
Result<Vec<T>>          // List of items
Result<HashMap<K, V>>   // Key-value data
Result<Option<T>>       // May or may not exist
Result<T>               // Specific struct (SessionData, FileMetadata, etc.)
```

### Error Handling
All methods return `anyhow::Result<T>` with descriptive errors:
- `PermissionDenied` - Missing required permission
- `InvalidPath` - Path validation failed
- `CdpError` - Browser/CDP operation failed
- `NotFound` - Element/file not found
- `Timeout` - Operation timed out

---

## Implementation Priority for v0.0.3

### Phase 1: Core Enhancements (Week 1)
1. `clear_clipboard()`
2. `data_file_exists()`
3. `delete_data_file()`
4. `read_json_data()` / `write_json_data()`

### Phase 2: Cookie/Session Expansion (Week 2)
1. `export_cookies_for_domain()`
2. `clear_cookies_for_domain()`
3. `export_local_storage()`
4. `validate_session_data()`

### Phase 3: Advanced Data Operations (Week 3)
1. `list_data_files()`
2. `append_data_file()`
3. `data_file_metadata()`
4. `download_file()` (new permission)

### Phase 4: DOM Inspection (Week 4)
1. `get_computed_style()`
2. `get_element_rect()`
3. `count_elements()`
4. `is_in_viewport()`

---

## Documentation Requirements

Every new method needs:
1. **Rust doc comment** with:
   - One-line description
   - Required permissions (explicitly stated)
   - Arguments with types
   - Return value description
   - Error conditions
   - Example usage

2. **TASK_AUTHORING_GUIDE.md** update:
   - New section for the feature area
   - Permission requirements
   - Common use cases
   - Best practices

3. **Test coverage**:
   - Unit tests for permission denial
   - Integration tests with browser fixture
   - Happy path + error path

---

## Migration Path

### From Current State
- Existing methods stay as-is (backward compatible)
- New methods follow this naming convention
- Eventually deprecate inconsistent names (if any)

### Breaking Changes (None planned for v0.0.3)
- All additions are new methods
- No existing method signatures change
- Permissions remain additive

---

## Open Questions

1. **Rate Limiting**: Should `http_*` methods have built-in rate limiting?
2. **File Size Limits**: Should data file operations have size caps?
3. **Async vs Sync**: Should clipboard operations stay sync (they use internal cache)?
4. **Batch Operations**: Should we add `export_cookies_batch(urls: &[&str])`?
5. **Caching**: Should DOM inspection results be cached briefly?

---

## Summary

**v0.0.3 Goal**: Add 15-20 new API methods following consistent naming conventions, all permission-gated, all well-documented.

**Impact**: Makes the task API comprehensive for advanced automation scenarios while maintaining security through permissions.

**Success Metric**: New tasks can be written without reaching into raw CDP or internal utilities.
