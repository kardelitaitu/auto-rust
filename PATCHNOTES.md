# Patch Notes

## v0.0.0 - 18 February 2026

### Initial Development
- Project scaffolding with Cargo workspace structure
- Basic CDP (Chrome DevTools Protocol) integration via chromiumoxide
- Core browser automation primitives: navigate, click, type, scroll
- Session management for multiple concurrent browser instances
- Initial task runner architecture with async/await support

## v0.0.1 - 2 April 2026

### Core Framework Completed
- **Task API**: High-level browser automation API (`TaskContext`)
  - Human-like mouse movements with bezier curves
  - Adaptive timing with fatigue modeling
  - Smart element detection and retry logic
- **Orchestrator**: Multi-session task distribution and load balancing
- **Configuration System**: TOML-based config with environment variable overrides
- **Metrics & Observability**: OpenTelemetry tracing, structured logging
- **Test Suite Expansion**:
  - CLI parsing tests
  - Launcher integration tests
  - Task registration validation tests
  - Wiremock-based API mocking for OpenRouter fallback chain

### Infrastructure
- Native interaction support (cursor movement, OS-level clicks)
- Click learning system with per-profile calibration
- Graceful shutdown with signal handling
- Browser discovery and session lifecycle management

## v0.0.2 - 27 April 2026

### Task Policy System Implemented
Comprehensive security and governance framework for task execution:

#### Security Features
- **Permission-based access control**: `allow_screenshot`, `allow_data_access`, `allow_export_cookies`, etc.
- **Path validation**: `validate_data_path()` prevents directory traversal attacks
- **Policy enforcement**: Timeout limits, permission checks at API boundary
- **Audit logging**: All policy violations and gated operations logged

#### API Improvements
- **New permission helpers**:
  - `check_permission()` - runtime permission validation
  - `check_page_connected()` - connection state verification
  - `map_cdp_error()` - standardized error mapping for CDP operations
- **Policy-aware task execution**: Tasks validate permissions before sensitive operations

### Screenshot Auto-Save Feature
New `api.screenshot()` method with automatic file management:

- **WebP format**: 25-35% better compression than JPG
- **Quality control**:
  - Default: 50% quality (optimal size/readability balance)
  - `screenshot_with_quality(quality)` for custom quality (1-100)
- **Automatic file naming**: `yyyy-mm-dd-hh-mm-sessionid.webp`
- **Directory management**: Auto-creates `data/screenshot/` directory
- **Permission gated**: Requires `allow_screenshot` permission

#### Tasks with Screenshot Support
- `twitterintent`: Captures proof screenshot after intent confirmation click

### Developer Experience
- **Documentation**: Comprehensive Task Policy guide for task authors
- **Testing**: Integration tests for screenshot functionality with browser fixtures
- **Git hygiene**: Screenshot directory added to `.gitignore`

### Dependencies
- Added `image` crate with WebP feature for image processing
- Added `webp` crate for lossy WebP encoding with quality control

### Migration Notes
- Tasks using `api.screenshot()` must declare `allow_screenshot: true` in their policy
- Screenshot files are now saved as `.webp` (not `.jpg`)
- Default quality reduced from 75% to 50% for smaller files
