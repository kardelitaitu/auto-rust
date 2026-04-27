# API Design Plan: Permission-Gated Task Context Methods

**Document Purpose**: Define consistent, discoverable API methods for all permission-gated operations in TaskContext.

**Target Version**: v0.0.3
**Status**: Draft with Implementation Confidence Assessment

## Browser Management Research Update

### Research Date: April 27, 2026

#### CDP Methods Discovered

**Storage Domain (Available in chromiumoxide):**
- `Storage.getCookies` - Returns **ALL** browser cookies across all domains ✅
- `Storage.setCookies` - Sets multiple cookies at once ✅
- `Storage.clearCookies` - Clears all cookies ✅
- `Storage.clearDataForOrigin` - Clears storage (localStorage, sessionStorage, IndexedDB, cache) for specific origin ✅
- `Storage.getStorageKeyForFrame` - Gets storage key for frame (deprecated, use getStorageKey) ⚠️

**DOMStorage Domain (Available):**
- `DOMStorage.getDOMStorageItems` - Gets localStorage/sessionStorage items for a storage ID ✅
- `DOMStorage.setDOMStorageItem` - Sets a storage item ✅
- `DOMStorage.clear` - Clears all items for a storage ID ✅

**Implementation Strategy:**

#### `api.export_browser()` - Revised Confidence: ⚠️ 80%
```rust
pub async fn export_browser(&self) -> Result<BrowserData>
```

**Implementation Plan:**
1. **Cookies**: Use `Storage.getCookies` (no URL param = all cookies) ✅
2. **Storage Areas**: More complex - need to:
   - Get all frame IDs from Page domain
   - For each frame, call `Storage.getStorageKeyForFrame` to get storage key
   - Use `DOMStorage.getDOMStorageItems` with storage ID (localStorage = 0, sessionStorage = 1)
   
**Challenges:**
- Enumerating all frames/storage areas is complex
- Need to discover unique origins first
- May miss storage for frames that haven't been loaded

**Alternative Simpler Approach:**
- Export cookies via `Storage.getCookies` (all domains)
- Export localStorage/sessionStorage only for CURRENT page via JavaScript
- Document limitation: only exports visible frame's storage

#### `api.import_browser()` - Revised Confidence: ✅ 85%
```rust
pub async fn import_browser(&self, data: &BrowserData) -> Result<()>
```

**Implementation Plan:**
1. **Cookies**: Use `Storage.setCookies` - can set all at once ✅
2. **Storage**: Navigate to each domain, inject JavaScript to set localStorage/sessionStorage
   - Navigate to domain URL
   - Execute JS: `localStorage.setItem(key, value)` for each item
   - Execute JS: `sessionStorage.setItem(key, value)` for each item

**Challenges:**
- Slow - requires navigation to each domain
- sessionStorage is per-tab, may not persist across sessions
- Race conditions if pages load async

**Revised Confidence Levels:**

| API | Original | Revised | Notes |
|-----|----------|---------|-------|
| `api.export_browser()` | 🔍 65% | ⚠️ 80% | Can use Storage.getCookies for all cookies, JS for current page storage |
| `api.import_browser()` | ⚠️ 75% | ✅ 85% | Storage.setCookies handles all cookies efficiently, JS for storage |

**Implementation Priority:**
- **Phase 1**: Implement simpler version that exports:
  - All cookies (all domains)
  - Only current page's localStorage/sessionStorage
- **Phase 2**: Consider full multi-frame export if needed

**Security Note:**
- These methods expose ALL browser data across ALL domains
- High security risk - requires strict `allow_browser_export`/`allow_browser_import` permissions
- Should audit log these operations

---

## Quick Reference: All APIs

### Current APIs (v0.0.2) - ✅ Implemented
| API | Description | Confidence |
|-----|-------------|------------|
| `api.screenshot()` | Capture WebP screenshot at 50% quality | ✅ 100% |
| `api.screenshot_with_quality(q)` | Screenshot with custom quality (1-100) | ✅ 100% |
| `api.export_cookies(url)` | Export cookies for URL | ✅ 100% |
| `api.import_cookies(cookies)` | Import cookies into browser | ✅ 100% |
| `api.export_session(url)` | Export cookies + localStorage | ✅ 100% |
| `api.import_session(data)` | Import cookies + localStorage | ✅ 100% |
| `api.read_clipboard()` | Read system clipboard | ✅ 100% |
| `api.write_clipboard(text)` | Write to system clipboard | ✅ 100% |
| `api.read_data_file(path)` | Read file from data/ or config/ | ✅ 100% |
| `api.write_data_file(path, content)` | Write file to data/ or config/ | ✅ 100% |

### Planned APIs (v0.0.3) - Implementation Checklist

**Legend:**
- ☐ = Not implemented
- ☑ = Implemented + tested + `cargo test` passing
- ✅ **90-100%** - Can implement now, clear path
- ⚠️ **70-89%** - Implementable, some complexity
- 🔍 **50-69%** - Needs investigation/research
- ❓ **<50%** - Unclear if feasible

#### Cookie Management
| Status | API | Description | Confidence | Notes |
|--------|-----|-------------|------------|-------|
| ☑ | `api.export_cookies_for_domain(domain)` | Export cookies matching domain | ✅ 95% | Filter existing Network.getCookies result |
| ☑ | `api.export_session_cookies(url)` | Export only session cookies | ✅ 90% | Check cookie.session flag from CDP |
| ☑ | `api.has_cookie(name, domain)` | Check if cookie exists | ✅ 95% | Check result of export_cookies_for_domain |

#### Session Management
| Status | API | Description | Confidence | Notes |
|--------|-----|-------------|------------|-------|
| ☑ | `api.export_local_storage(url)` | Export only localStorage | ✅ 95% | Extract from existing export_session logic |
| ☑ | `api.import_local_storage(url, data)` | Import only localStorage | ✅ 95% | Extract from existing import_session logic |
| ☑ | `api.validate_session_data(data)` | Validate without importing | ✅ 98% | Pure JSON validation, no browser needed |

#### Clipboard Management
| Status | API | Description | Confidence | Notes |
|--------|-----|-------------|------------|-------|
| ☑ | `api.clear_clipboard()` | Clear clipboard content | ✅ 95% | Set empty string using existing clipboard module |
| ☑ | `api.has_clipboard_content()` | Check if clipboard non-empty | ✅ 95% | Check if read_clipboard returns non-empty |
| ☑ | `api.append_clipboard(text, sep)` | Append text to clipboard | ✅ 95% | Read + write combo using existing methods |

#### Data File Management
| Status | API | Description | Confidence | Notes |
|--------|-----|-------------|------------|-------|
| ☑ | `api.list_data_files(subdir)` | List files in data directory | ✅ 98% | Standard Rust std::fs::read_dir |
| ☑ | `api.append_data_file(path, content)` | Append to file | ✅ 98% | OpenOptions::append() - standard Rust |
| ☑ | `api.data_file_exists(path)` | Check if file exists | ✅ 100% | std::path::Path::exists() |
| ☑ | `api.delete_data_file(path)` | Delete file | ✅ 98% | std::fs::remove_file() |
| ☑ | `api.read_json_data<T>(path)` | Read and parse JSON | ✅ 95% | serde_json::from_str after read_data_file |
| ☑ | `api.write_json_data<T>(path, data)` | Write JSON (pretty) | ✅ 95% | serde_json::to_string_pretty then write |
| ☑ | `api.data_file_metadata(path)` | Get file size/modified time | ✅ 98% | std::fs::metadata() |

#### Network/HTTP (new permission: `allow_http_requests`)
| Status | API | Description | Confidence | Notes |
|--------|-----|-------------|------------|-------|
| ☑ | `api.http_get(url)` | HTTP GET request | ✅ 90% | reqwest already in dependencies |
| ☑ | `api.http_post_json(url, body)` | HTTP POST with JSON | ✅ 90% | reqwest with json feature available |
| ☑ | `api.download_file(url, path)` | Download to data directory | ✅ 90% | reqwest + write_data_file combo |

#### DOM Inspection (new permission: `allow_dom_inspection`)
| Status | API | Description | Confidence | Notes |
|--------|-----|-------------|------------|-------|
| ☑ | `api.get_computed_style(selector, property)` | Get CSS property | ✅ 95% | window.getComputedStyle via page.evaluate |
| ☑ | `api.get_element_rect(selector)` | Get element position/size | ✅ 95% | getBoundingClientRect already used in codebase |
| ☑ | `api.get_scroll_position()` | Get page scroll position | ✅ 95% | window.scrollX / scrollY via page.evaluate |
| ☑ | `api.count_elements(selector)` | Count matching elements | ✅ 95% | document.querySelectorAll().length via evaluate |
| ☑ | `api.is_in_viewport(selector)` | Check if element visible | ✅ 90% | getBoundingClientRect + window dimensions comparison |

#### Browser Management (new permissions: `allow_browser_export`, `allow_browser_import`)
| Status | API | Description | Confidence | Notes |
|--------|-----|-------------|------------|-------|
| ☐ | `api.export_browser()` | Export ALL browser data | 🔍 65% | CDP Storage.getStorageKeyForFrame? Complex |
| ☐ | `api.import_browser(data)` | Import complete browser state | ⚠️ 75% | Multiple import operations combined 

**Implementation Workflow:**
1. Implement method in `src/runtime/task_context.rs`
2. Add unit tests for permission denial
3. Add integration tests with browser fixture
4. Run `cargo test` and verify all tests pass
5. Mark checkbox as ☑ in this document
6. Commit with message: `feat: add api.xxx() for [purpose]`

---

## Implementation Feasibility Analysis

### High Confidence (90-100%) - Phase 1 Candidates
These can be implemented immediately using existing patterns:

1. **data_file_exists()** - Standard Rust, zero risk
2. **delete_data_file()** - Standard Rust, zero risk
3. **list_data_files()** - Standard Rust, zero risk
4. **read_json_data() / write_json_data()** - Serde already used
5. **data_file_metadata()** - Standard Rust
6. **append_data_file()** - Standard Rust
7. **clear_clipboard()** - Trivial wrapper
8. **has_clipboard_content()** - Trivial wrapper
9. **append_clipboard()** - Combine existing read+write
10. **get_element_rect()** - Already doing this in twitter tasks
11. **count_elements()** - Simple JS via evaluate
12. **get_scroll_position()** - Simple JS via evaluate

### Medium Confidence (70-89%) - Phase 2 Candidates
Implementable but need some investigation:

1. **export_cookies_for_domain()** - Need to filter CDP results
2. **export_session_cookies()** - Check session flag in cookie objects
3. **clear_cookies_for_domain()** - May need JS workaround
4. **export_local_storage()** - Extract from existing code
5. **http_get() / http_post()** - reqwest available but need error handling design
6. **get_computed_style()** - JS execution, straightforward
7. **is_in_viewport()** - Math calculation on rect

### Lower Confidence (50-69%) - Phase 3 Candidates
Need significant research:

1. **export_browser()** - Full browser export complex, may need multiple CDP calls
2. **clear_browser_data()** - CDP Storage.clearDataForOrigin needs testing
3. **read_clipboard_with_timeout()** - Platform-specific clipboard watching
4. **is_session_valid()** - Cookie expiry checking is non-trivial

---

## Recommended Implementation Order

### Phase 1: Quick Wins (Week 1) - Data Files & Clipboard
All 90%+ confidence, minimal risk:

```rust
// Data File Operations (8 methods)
pub fn data_file_exists(&self, path: &str) -> Result<bool>
pub fn delete_data_file(&self, path: &str) -> Result<()>
pub fn list_data_files(&self, subdir: &str) -> Result<Vec<String>>
pub fn data_file_metadata(&self, path: &str) -> Result<FileMetadata>
pub fn append_data_file(&self, path: &str, content: &[u8]) -> Result<()>
pub fn read_json_data<T: DeserializeOwned>(&self, path: &str) -> Result<T>
pub fn write_json_data<T: Serialize>(&self, path: &str, data: &T) -> Result<()>

// Clipboard Operations (3 methods)
pub fn clear_clipboard(&self) -> Result<()>
pub fn has_clipboard_content(&self) -> Result<bool>
pub fn append_clipboard(&self, text: &str, separator: &str) -> Result<()>
```

### Phase 2: DOM Inspection (Week 2)
All 90%+ confidence, build on existing patterns:

```rust
// DOM Inspection (5 methods)
pub async fn get_computed_style(&self, selector: &str, property: &str) -> Result<String>
pub async fn get_element_rect(&self, selector: &str) -> Result<Rect>
pub async fn get_scroll_position(&self) -> Result<(u32, u32)>
pub async fn count_elements(&self, selector: &str) -> Result<usize>
pub async fn is_in_viewport(&self, selector: &str) -> Result<bool>
```

### Phase 3: Cookie/Session Enhancement (Week 3)
Medium complexity:

```rust
// Cookie Management (4 methods)
pub async fn export_cookies_for_domain(&self, domain: &str) -> Result<Vec<serde_json::Value>>
pub async fn export_session_cookies(&self, url: &str) -> Result<Vec<serde_json::Value>>
pub async fn clear_cookies_for_domain(&self, domain: &str) -> Result<usize>
pub async fn has_cookie(&self, name: &str, domain: &str) -> Result<bool>

// Session Management (4 methods)
pub async fn export_local_storage(&self, url: &str) -> Result<HashMap<String, String>>
pub async fn import_local_storage(&self, url: &str, data: &HashMap<String, String>) -> Result<()>
pub fn validate_session_data(&self, data: &SessionData) -> Result<Vec<String>>
pub async fn is_session_valid(&self, url: &str) -> Result<bool>
```

### Phase 4: Network Operations (Week 4)
New permission needed:

```rust
// Network/HTTP (3 methods) - requires allow_http_requests permission
pub async fn http_get(&self, url: &str) -> Result<HttpResponse>
pub async fn http_post_json<T: Serialize>(&self, url: &str, body: &T) -> Result<HttpResponse>
pub async fn download_file(&self, url: &str, relative_path: &str) -> Result<u64>
```

### Phase 5: Browser Management (Week 5-6)
Complex, needs research:

```rust
// Browser Management (4 methods) - requires allow_browser_export/import
pub async fn export_browser(&self) -> Result<BrowserData>
pub async fn export_browser_for_domain(&self, domain: &str) -> Result<BrowserData>
pub async fn import_browser(&self, data: &BrowserData) -> Result<()>
pub async fn clear_browser_data(&self) -> Result<()>
```

---

## Design Principles

1. **Verb-Noun Naming**: `action_resource()` (e.g., `export_cookies()`, `read_clipboard()`)
2. **Async for I/O**: All methods that touch browser, files, or network are `async`
3. **Sync for Memory**: Pure memory operations can be synchronous
4. **Explicit Permissions**: Every method documents its required permission
5. **Consistent Returns**: `Result<T>` with descriptive error messages
6. **Rust Idioms**: Use `&str` for inputs, owned types for outputs

---

## Detailed Method Specifications

### 1. Cookie Management (Enhanced)

#### `export_cookies_for_domain()` - ✅ 95% Confidence
```rust
pub async fn export_cookies_for_domain(&self, domain: &str) -> Result<Vec<serde_json::Value>>
```
**Implementation**: Filter result of Network.getCookies by domain field.
**Requires**: `allow_export_cookies`
**Feasibility**: CDP returns all cookies, filter in Rust by checking cookie["domain"].
**Risk**: Low - just filtering existing data.

#### `export_session_cookies()` - ✅ 90% Confidence
```rust
pub async fn export_session_cookies(&self, url: &str) -> Result<Vec<serde_json::Value>>
```
**Implementation**: Filter cookies where session=true or expires is None.
**Requires**: `allow_export_cookies`
**Feasibility**: Cookie object has session flag from CDP.
**Risk**: Low - need to verify CDP cookie structure.

#### `clear_cookies_for_domain()` - ⚠️ 80% Confidence
```rust
pub async fn clear_cookies_for_domain(&self, domain: &str) -> Result<usize>
```
**Implementation**: Either Network.deleteCookies (if available) or JS workaround.
**Requires**: `allow_export_cookies` + `allow_import_cookies` (implied)
**Feasibility**: Chromiumoxide may not expose Network.deleteCookies directly.
**Risk**: Medium - may need JS execution with document.cookie manipulation.
**Alternative**: Execute JS: `document.cookie = "name=; expires=Thu, 01 Jan 1970...; domain=..."`

#### `has_cookie()` - ✅ 95% Confidence
```rust
pub async fn has_cookie(&self, name: &str, domain: &str) -> Result<bool>
```
**Implementation**: Check if any cookie in filtered list matches name.
**Requires**: `allow_export_cookies`
**Feasibility**: Trivial wrapper around export_cookies_for_domain.
**Risk**: None.

---

### 2. Session Management (Enhanced)

#### `export_local_storage()` - ✅ 95% Confidence
```rust
pub async fn export_local_storage(&self, _url: &str) -> Result<HashMap<String, String>>
```
**Implementation**: Extract JS localStorage loop from existing export_session.
**Requires**: `allow_export_session`
**Feasibility**: Already have working code in export_session, just separate it.
**Risk**: None - code already proven.

#### `import_local_storage()` - ✅ 95% Confidence
```rust
pub async fn import_local_storage(&self, _url: &str, data: &HashMap<String, String>) -> Result<()>
```
**Implementation**: Extract JS localStorage.setItem loop from import_session.
**Requires**: `allow_import_session`
**Feasibility**: Code already works in import_session.
**Risk**: None - code already proven.

#### `validate_session_data()` - ✅ 98% Confidence
```rust
pub fn validate_session_data(&self, data: &SessionData) -> Result<Vec<String>>
```
**Implementation**: Check JSON structure, required fields, data types.
**Requires**: None (read-only validation)
**Feasibility**: Pure Rust validation, no browser needed.
**Risk**: None - just struct validation.
**Returns**: Vec of warning strings (empty if valid).

#### `is_session_valid()` - ⚠️ 75% Confidence
```rust
pub async fn is_session_valid(&self, url: &str) -> Result<bool>
```
**Implementation**: Export cookies, check if any session cookies expired.
**Requires**: `allow_export_session`
**Feasibility**: Need to parse cookie expiry dates, compare with current time.
**Risk**: Medium - cookie date parsing can be tricky (multiple formats).
**Note**: May not be 100% accurate - just checks cookie expiry, not server-side session.

---

### 3. Clipboard Management (Enhanced)

#### `clear_clipboard()` - ✅ 95% Confidence
```rust
pub fn clear_clipboard(&self) -> Result<()>
```
**Implementation**: Call existing write_clipboard with empty string.
**Requires**: `allow_session_clipboard`
**Feasibility**: One-line wrapper.
**Risk**: None.

#### `read_clipboard_with_timeout()` - 🔍 60% Confidence
```rust
pub async fn read_clipboard_with_timeout(&self, timeout_ms: u64) -> Result<Option<String>>
```
**Implementation**: Poll clipboard every 100ms until content changes or timeout.
**Requires**: `allow_session_clipboard`
**Feasibility**: Tricky - clipboard module may not support change detection.
**Risk**: High - could miss rapid changes, inefficient polling.
**Alternative**: Could skip this and just use read_clipboard for now.
**Platform Issues**: Clipboard watching is platform-specific.

#### `has_clipboard_content()` - ✅ 95% Confidence
```rust
pub fn has_clipboard_content(&self) -> Result<bool>
```
**Implementation**: Call read_clipboard, check if result is non-empty.
**Requires**: `allow_session_clipboard`
**Feasibility**: Trivial wrapper.
**Risk**: None.

#### `append_clipboard()` - ✅ 95% Confidence
```rust
pub fn append_clipboard(&self, text: &str, separator: &str) -> Result<()>
```
**Implementation**: Read current, append with separator, write back.
**Requires**: `allow_session_clipboard`
**Feasibility**: Combine existing read + write.
**Risk**: Low - race condition if clipboard changes between read/write.

---

### 4. Data File Management (Enhanced)

All data file operations are **✅ 95-100% confidence** - they use standard Rust std::fs.

#### `list_data_files()` - ✅ 98% Confidence
```rust
pub fn list_data_files(&self, subdir: &str) -> Result<Vec<String>>
```
**Implementation**: std::fs::read_dir, filter to files, return relative paths.
**Requires**: `allow_read_data`
**Feasibility**: Standard Rust.
**Risk**: None.

#### `append_data_file()` - ✅ 98% Confidence
```rust
pub fn append_data_file(&self, relative_path: &str, content: &[u8]) -> Result<()>
```
**Implementation**: OpenOptions::append().open(path), write content.
**Requires**: `allow_write_data`
**Feasibility**: Standard Rust.
**Risk**: None.

#### `data_file_exists()` - ✅ 100% Confidence
```rust
pub fn data_file_exists(&self, relative_path: &str) -> Result<bool>
```
**Implementation**: std::path::Path::exists()
**Requires**: `allow_read_data`
**Feasibility**: Trivial.
**Risk**: None.

#### `delete_data_file()` - ✅ 98% Confidence
```rust
pub fn delete_data_file(&self, relative_path: &str) -> Result<()>
```
**Implementation**: std::fs::remove_file() after path validation.
**Requires**: `allow_write_data`
**Feasibility**: Standard Rust.
**Risk**: None.

#### `read_json_data()` - ✅ 95% Confidence
```rust
pub fn read_json_data<T: DeserializeOwned>(&self, relative_path: &str) -> Result<T>
```
**Implementation**: Read file as string, serde_json::from_str.
**Requires**: `allow_read_data`
**Feasibility**: Serde already used in codebase.
**Risk**: Low - type T must match JSON structure.

#### `write_json_data()` - ✅ 95% Confidence
```rust
pub fn write_json_data<T: Serialize>(&self, relative_path: &str, data: &T) -> Result<()>
```
**Implementation**: serde_json::to_string_pretty, then write_data_file.
**Requires**: `allow_write_data`
**Feasibility**: Serde already used.
**Risk**: None.

#### `data_file_metadata()` - ✅ 98% Confidence
```rust
pub fn data_file_metadata(&self, relative_path: &str) -> Result<FileMetadata>
```
**Implementation**: std::fs::metadata(), extract size, modified time.
**Requires**: `allow_read_data`
**Feasibility**: Standard Rust.
**Risk**: None.
**Returns**:
```rust
pub struct FileMetadata {
    pub size: u64,
    pub modified: SystemTime,
    pub created: SystemTime,
}
```

---

### 5. Network/HTTP (New Permission: `allow_http_requests`)

#### `http_get()` - ✅ 90% Confidence
```rust
pub async fn http_get(&self, url: &str) -> Result<HttpResponse>
```
**Implementation**: reqwest::get(url), return status + body.
**Requires**: `allow_http_requests`
**Feasibility**: reqwest already in dependencies (v0.11).
**Risk**: Low - standard HTTP client.
**Returns**:
```rust
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}
```

#### `http_post_json()` - ✅ 90% Confidence
```rust
pub async fn http_post_json<T: Serialize>(&self, url: &str, body: &T) -> Result<HttpResponse>
```
**Implementation**: reqwest::Client::post().json(body).send().
**Requires**: `allow_http_requests`
**Feasibility**: reqwest has json() method.
**Risk**: Low.

#### `download_file()` - ✅ 90% Confidence
```rust
pub async fn download_file(&self, url: &str, relative_path: &str) -> Result<u64>
```
**Implementation**: reqwest GET, stream to file, return bytes downloaded.
**Requires**: `allow_http_requests` + `allow_write_data` (implied)
**Feasibility**: reqwest supports streaming.
**Risk**: Low - just combining reqwest + write_data_file.
**Returns**: Number of bytes downloaded.

---

### 6. DOM Inspection (New Permission: `allow_dom_inspection`)

All DOM inspection methods use `page.evaluate()` with JavaScript.

#### `get_computed_style()` - ✅ 95% Confidence
```rust
pub async fn get_computed_style(&self, selector: &str, property: &str) -> Result<String>
```
**Implementation**: JS: `window.getComputedStyle(document.querySelector(sel))[prop]`
**Requires**: `allow_dom_inspection`
**Feasibility**: Standard browser API via evaluate.
**Risk**: None.

#### `get_element_rect()` - ✅ 95% Confidence
```rust
pub async fn get_element_rect(&self, selector: &str) -> Result<Rect>
```
**Implementation**: JS: `document.querySelector(sel).getBoundingClientRect()`
**Requires**: `allow_dom_inspection`
**Feasibility**: Already using this pattern in twitter tasks.
**Risk**: None.
**Returns**:
```rust
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
```

#### `get_scroll_position()` - ✅ 95% Confidence
```rust
pub async fn get_scroll_position(&self) -> Result<(u32, u32)>
```
**Implementation**: JS: `({x: window.scrollX, y: window.scrollY})`
**Requires**: `allow_dom_inspection`
**Feasibility**: Simple evaluate call.
**Risk**: None.
**Returns**: (scrollX, scrollY) in pixels.

#### `count_elements()` - ✅ 95% Confidence
```rust
pub async fn count_elements(&self, selector: &str) -> Result<usize>
```
**Implementation**: JS: `document.querySelectorAll(sel).length`
**Requires**: `allow_dom_inspection`
**Feasibility**: Simple evaluate.
**Risk**: None.

#### `is_in_viewport()` - ✅ 90% Confidence
```rust
pub async fn is_in_viewport(&self, selector: &str) -> Result<bool>
```
**Implementation**: 
```javascript
const rect = el.getBoundingClientRect();
return rect.top >= 0 && rect.left >= 0 && 
       rect.bottom <= window.innerHeight && 
       rect.right <= window.innerWidth;
```
**Requires**: `allow_dom_inspection`
**Feasibility**: Combine getBoundingClientRect with window dimensions.
**Risk**: Low - standard viewport calculation.

---

### 7. Browser Management (New Permissions: `allow_browser_export`, `allow_browser_import`)

#### `export_browser()` - 🔍 65% Confidence
```rust
pub async fn export_browser(&self) -> Result<BrowserData>
```
**Implementation**: Combine multiple CDP calls:
1. Network.getAllCookies (all domains)
2. For each frame: DOMStorage.getDOMStorageItems (localStorage)
3. For each frame: DOMStorage.getDOMStorageItems (sessionStorage)
**Requires**: `allow_browser_export`
**Feasibility**: Complex - need to enumerate all frames/storage areas.
**Risk**: High - may miss some storage areas, slow on browsers with many tabs.
**Alternative**: Start with export_browser_for_domain which is simpler.

#### `export_browser_for_domain()` - ⚠️ 85% Confidence
```rust
pub async fn export_browser_for_domain(&self, domain: &str) -> Result<BrowserData>
```
**Implementation**: 
1. Filter cookies by domain (existing pattern)
2. Navigate to domain, export localStorage via JS
3. Export sessionStorage via JS
**Requires**: `allow_browser_export`
**Feasibility**: Easier than full export - just one domain.
**Risk**: Low-medium - need to navigate to domain first if not already there.

#### `import_browser()` - ⚠️ 75% Confidence
```rust
pub async fn import_browser(&self, data: &BrowserData) -> Result<()>
```
**Implementation**: For each domain in data:
1. Import cookies
2. Navigate to domain
3. Import localStorage via JS
4. Import sessionStorage via JS
**Requires**: `allow_browser_import`
**Feasibility**: Multiple operations, may be slow.
**Risk**: Medium - complex multi-step operation, may leave browser in partial state if fails.

#### `clear_browser_data()` - 🔍 60% Confidence
```rust
pub async fn clear_browser_data(&self) -> Result<()>
```
**Implementation**: 
**Option A**: CDP Storage.clearDataForOrigin (if available in chromiumoxide)
**Option B**: Clear cookies via Network.deleteCookies, clear storage via JS
**Requires**: `allow_browser_export` + `allow_browser_import` (implied)
**Feasibility**: Depends on CDP method availability.
**Risk**: High - may not clear all data types (IndexedDB, cache, etc.).
**Research Needed**: Check if chromiumoxide exposes Storage.clearDataForOrigin.

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
- `list_*` - Enumerate items
- `delete_*` - Remove single item

### Nouns (Resources)
- `*_cookies` - Browser cookies
- `*_session` - Cookies + localStorage
- `*_local_storage` - localStorage only
- `*_clipboard` - System clipboard
- `*_data_file` / `*_data` - File system (data/ config/)
- `*_storage` / `*_cache` - Future: IndexedDB, etc.
- `*_browser` - Complete browser state
- `*_json` - JSON data
- `*_metadata` - File information
- `*_element` / `*_rect` - DOM elements
- `*_style` - CSS properties
- `*_position` / `*_scroll` - Viewport information

---

## Return Type Patterns

### Success Indicators
```rust
Result<()>              // Operation succeeded, no return value
Result<bool>            // Yes/no answer
Result<usize>           // Count of items affected
Result<u64>             // Large count (bytes downloaded)
Result<String>          // Single text value
Result<Vec<T>>          // List of items
Result<HashMap<K, V>>   // Key-value data
Result<Option<T>>       // May or may not exist
Result<T>               // Specific struct (SessionData, FileMetadata, etc.)
Result<(u32, u32)>      // Tuple returns (coordinates, scroll position)
```

### Error Handling
All methods return `anyhow::Result<T>` with descriptive errors:
- `PermissionDenied` - Missing required permission
- `InvalidPath` - Path validation failed
- `CdpError` - Browser/CDP operation failed
- `NotFound` - Element/file not found
- `Timeout` - Operation timed out
- `HttpError` - Network request failed
- `ValidationError` - Data validation failed

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

## Open Questions & Risks

### High Risk / Needs Research
1. **Browser Management**: Complex multi-domain operations
   - Research: CDP Storage.clearDataForOrigin availability
   - Alternative: Implement single-domain versions first

2. **Clipboard Polling**: `read_clipboard_with_timeout()`
   - Problem: Platform-specific, inefficient polling
   - Alternative: Skip for now, use simple read_clipboard

3. **Session Validity**: `is_session_valid()`
   - Problem: Cookie date parsing complexity
   - Alternative: Document limitations clearly

### Medium Risk
1. **Cookie Clearing**: `clear_cookies_for_domain()`
   - May need JS workaround if CDP method unavailable
   - Can be implemented but may be fragile

2. **HTTP Operations**: reqwest dependency already present
   - Need good error handling design
   - Consider timeout configuration

### Low Risk
1. **Data File Operations**: Standard Rust
2. **DOM Inspection**: Already doing this in tasks
3. **JSON Data**: Serde already used

---

## Summary

### v0.0.3 Implementation Scope

**Total New APIs**: 28 methods

**By Confidence Level**:
- ✅ **90-100%** (High Confidence): 17 methods - **Implement first**
- ⚠️ **70-89%** (Medium Confidence): 6 methods - **Phase 2**
- 🔍 **50-69%** (Low Confidence): 5 methods - **Research first**

**By Category**:
- Data File Management: 7 methods (all high confidence)
- DOM Inspection: 5 methods (all high confidence)
- Cookie Management: 4 methods (mostly high)
- Session Management: 4 methods (mostly high)
- Clipboard Management: 4 methods (1 low confidence)
- Network/HTTP: 3 methods (medium confidence)
- Browser Management: 4 methods (mostly low confidence)

**Recommendation**: Start with Phase 1 (Data Files + DOM Inspection) for quick wins, then tackle Cookie/Session enhancement, defer Browser Management until more research done.
