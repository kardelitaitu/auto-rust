# Release v0.0.3 - Browser Management APIs

**Release Date:** April 27, 2026  
**Tag:** `v0.0.3`

## Overview

This release introduces **26 new TaskContext APIs** for browser management, session handling, DOM inspection, and data operations. All APIs feature permission-based security with comprehensive documentation and examples.

---

## New Features

### 26 TaskContext APIs (v0.0.3)

| Category | APIs | Count |
|----------|------|-------|
| **Cookie Management** | `export_cookies_for_domain`, `export_session_cookies`, `has_cookie` | 3 |
| **Session Management** | `export_local_storage`, `import_local_storage`, `validate_session_data` | 3 |
| **Clipboard Management** | `clear_clipboard`, `has_clipboard_content`, `append_clipboard` | 3 |
| **Data File Management** | `list_data_files`, `data_file_exists`, `delete_data_file`, `read_json_data`, `write_json_data`, `append_data_file`, `data_file_metadata` | 7 |
| **Network/HTTP** | `http_get`, `http_post_json`, `download_file` | 3 |
| **DOM Inspection** | `get_computed_style`, `get_element_rect`, `get_scroll_position`, `count_elements`, `is_in_viewport` | 5 |
| **Browser Management** | `export_browser`, `import_browser` | 2 |

**Total: 26 new APIs**

---

## Documentation (975+ Lines)

### Rustdoc Examples
Every new API includes comprehensive `# Examples` sections in `src/runtime/task_context.rs`:
- Realistic usage patterns
- Permission requirements documented
- Error handling examples
- IDE auto-complete support

### New Documentation Files
| File | Lines | Content |
|------|-------|---------|
| `docs/API_USAGE_GUIDE.md` | 200+ | 8 sections with practical recipes |
| `docs/API_REFERENCE.md` | 175+ | Complete API signatures & permissions |
| `docs/TASK_AUTHORING_GUIDE.md` | 250+ | Task examples & patterns |
| `README.md` updates | 50+ | v0.0.3 features & quick examples |

### Documentation Highlights
- **Cookie Management Recipes:** Login check, persistence, validation
- **API Selection Guide:** Which API to use for common tasks
- **Error Handling Patterns:** Permission errors, graceful degradation, timeouts
- **Testing Patterns:** Unit and integration test examples
- **Complete API Reference:** All 26 APIs with permissions table

---

## Security

### Permission-Based Access Control
All new APIs require explicit `TaskPolicy` permissions:

| Category | Read Permission | Write Permission |
|----------|-----------------|------------------|
| Cookie Management | `allow_export_cookies` | `allow_import_cookies` |
| Session Management | `allow_export_session` | `allow_import_session` |
| Clipboard | - | `allow_session_clipboard` |
| Data File | `allow_read_data` | `allow_write_data` |
| Network/HTTP | - | `allow_http_requests` |
| DOM Inspection | `allow_dom_inspection` | - |
| Browser Mgmt | `allow_browser_export` | `allow_browser_import` |

**12 new permission types** for fine-grained access control.

---

## Testing

### Test Coverage
- **1743 unit tests** passing
- **38 mock-based integration tests** for v0.0.3 APIs
- Comprehensive tests in `tests/api_mock_integration.rs`

### Test Infrastructure
- Mock browser contexts for isolated testing
- Local HTML fixtures
- Parallel test execution support

---

## Breaking Changes

None. This is a backward-compatible feature release.

---

## Quick Start

```rust
// Check if user is logged in
let has_session = ctx.has_cookie("session_id", Some("example.com")).await?;

// Export browser state
let browser_data = ctx.export_browser("https://example.com").await?;

// Call REST API
let response = ctx.http_get("https://api.example.com/data").await?;

// Read configuration
let config: serde_json::Value = ctx.read_json_data("config/app.json")?;
```

---

## Files Changed

- `Cargo.toml` - Version bump to 0.0.3
- `src/runtime/task_context.rs` - 26 new APIs with rustdoc examples
- `docs/API_USAGE_GUIDE.md` - NEW: Practical recipes
- `docs/API_REFERENCE.md` - UPDATED: Complete API reference
- `docs/TASK_AUTHORING_GUIDE.md` - ENHANCED: Task authoring examples
- `README.md` - UPDATED: v0.0.3 features
- `tests/api_mock_integration.rs` - 38 integration tests

---

## Full Changelog

See [journal.md](journal.md) for detailed development history.

---

## Contributors

- Ganteng Maksimal

---

## License

MIT
