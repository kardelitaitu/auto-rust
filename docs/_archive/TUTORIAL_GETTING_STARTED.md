# Tutorial: Getting Started with Auto-Rust

last audited 26-06-26 by Buffy

A comprehensive video tutorial script for new users.

## Video Metadata
- **Duration**: 15-20 minutes
- **Target Audience**: Developers new to browser automation
- **Prerequisites**: Basic Rust knowledge, terminal familiarity
- **Outcome**: User can run their first automation task

---

## Section 1: Introduction (2 minutes)

### Opening Hook
"What if you could automate any browser interaction with just a few lines of Rust code? Today I'll show you Auto-Rust - a powerful multi-browser automation framework that makes complex browser automation simple, reliable, and type-safe."

### What is Auto-Rust?
- **Definition**: Multi-browser automation orchestrator built in Rust
- **Key Features**:
  - Connect to multiple browser instances simultaneously
  - Run tasks across browsers in parallel
  - Built-in retry logic and circuit breakers
  - Type-safe API with comprehensive error handling
  - Permission-based security model

### Use Cases
- End-to-end testing across multiple browsers
- Data extraction and web scraping
- Automated form filling and interaction
- Session management and cookie handling
- API integration and webhook testing

---

## Section 2: Installation & Setup (3 minutes)

### Prerequisites Check
```bash
# Check Rust installation
rustc --version
cargo --version

# Expected output: rustc 1.70+ and cargo 1.70+
```

### Clone and Build
```bash
# Clone the repository
git clone https://github.com/kardelitaitu/auto-rust.git
cd auto-rust

# Build the project
cargo build --release

# Run tests to verify setup
cargo test
```

### Project Structure Overview
```
auto-rust/
├── src/           # Core source code
├── docs/          # Documentation
├── config/        # Configuration files
├── tests/         # Integration tests
└── Cargo.toml     # Project manifest
```

---

## Section 3: First Task - Cookie Bot (4 minutes)

### What is Cookiebot?
"The cookiebot task demonstrates the basic interaction pattern: navigate to a site, handle cookie consent dialogs, and verify the page loaded correctly."

### Running the Task
```bash
# Run cookiebot on default browser
cargo run cookiebot

# Run with specific URL
cargo run cookiebot=https://example.com
```

### Understanding the Output
```
[2026-01-15T10:30:00Z INFO auto::task] Running task: cookiebot
[2026-01-15T10:30:02Z INFO auto::task] Navigated to https://example.com
[2026-01-15T10:30:03Z INFO auto::task] Clicked cookie consent button
[2026-01-15T10:30:05Z INFO auto::task] Task completed successfully
```

### Key Concepts Demonstrated
1. **Task Execution**: Single task running on connected browser
2. **Navigation**: Loading pages with timeout handling
3. **Interaction**: Clicking elements via CSS selectors
4. **Verification**: Checking page state after actions

---

## Section 4: Multi-Browser Setup (3 minutes)

### Starting Multiple Browsers
"Auto-Rust can orchestrate tasks across multiple browser instances. Let's connect Chrome and Firefox simultaneously."

### Browser Configuration
```bash
# Connect to specific browser
cargo run -- --browsers brave cookiebot

# All connected browsers will run the task in parallel
```

### Browser Connection Flow
1. Auto-Rust discovers browsers via CDP (Chrome DevTools Protocol)
2. Establishes WebSocket connections to each browser
3. Creates isolated sessions for each browser
4. Broadcasts tasks to all connected browsers
5. Collects results from each session

### Supported Browsers
- **Brave**: `--remote-debugging-port=9001`
- **RoxyBrowser**: API integration

### Verifying Multiple Connections
```bash
# Check browser connections in logs
[2026-01-15T10:30:00Z INFO auto::orchestrator] Connected to browser: brave
```

---

## Section 5: Task Chaining with "then" (3 minutes)

### Sequential Task Execution
"Sometimes you need tasks to run in sequence. The 'then' keyword creates sequential task groups."

### Example: Login then Navigate
```bash
# Run login task, then navigate to dashboard
cargo run login then pageview=dashboard

# Multiple tasks per group
cargo run cookiebot pageview then screenshot logout
```

### Task Group Semantics
- **Within a group**: Tasks run in parallel across all browsers
- **Across groups**: Groups run sequentially (group N+1 starts after group N completes)
- **Use case**: Login (group 1) → Perform actions (group 2) → Logout (group 3)

### Visual Representation
```
Group 1: [login] ──parallel──> [Browser A, Browser B, Browser C]
   │
   ▼ (after all complete)
Group 2: [dashboard] ──parallel──> [Browser A, Browser B, Browser C]
   │
   ▼ (after all complete)
Group 3: [logout] ──parallel──> [Browser A, Browser B, Browser C]
```

---

## Section 6: Configuration (2 minutes)

### Config File Location
```
config/default.toml
```

### Basic Configuration
```toml
[orchestrator]
timeout_ms = 30000
max_global_concurrency = 10

[browser]
max_discovery_retries = 3
discovery_retry_delay_ms = 5000

[[browser.profiles]]
name = "brave-local"
type = "brave"
```

### Environment Variables
```bash
# RoxyBrowser
export ROXYBROWSER_API_URL="https://api.roxybrowser.com/"
export ROXYBROWSER_API_KEY="your-api-key"

# Orchestrator
export MAX_GLOBAL_CONCURRENCY="10"
export TASK_TIMEOUT_MS="300000"

# Logging
export RUST_LOG="info,orchestrator=debug"
```

---

## Section 7: Troubleshooting (2 minutes)

### Common Issues

#### Browser Not Found
```
Error: No browsers available
```
**Solution**: Start Brave with remote debugging enabled:
```bash
brave --remote-debugging-port=9001
```

#### Task Timeout
```
Error: Task timed out after 30000ms
```
**Solution**: Increase timeout in config or use `--timeout` flag

#### Permission Denied
```
Error: Permission denied: task lacks 'allow_screenshot' permission
```
**Solution**: Check task policy in `src/task/policy.rs`

### Debug Mode
```bash
# Enable debug logging
RUST_LOG=debug cargo run cookiebot
```

---

## Section 8: Next Steps (1 minute)

### What You've Learned
✅ How to build and run Auto-Rust
✅ Running single and multiple tasks
✅ Multi-browser orchestration
✅ Task chaining with "then"
✅ Basic configuration and troubleshooting

### Recommended Next Steps
1. **Read the API Reference**: Explore all available TaskContext APIs
2. **Build Your First Task**: Follow the "Building Your First Task" tutorial
3. **Explore Examples**: Check `src/task/` for more task implementations
4. **Join Community**: GitHub discussions for questions and sharing

### Resources
- **Documentation**: `docs/API_REFERENCE.md`
- **Task Authoring Guide**: `docs/TASK_AUTHORING_GUIDE.md`
- **API Usage Guide**: `docs/API_USAGE_GUIDE.md`
- **GitHub**: https://github.com/kardelitaitu/auto-rust

---

## Production Notes for Video Creator

### Visual Aids Needed
- [ ] Terminal recordings with syntax highlighting
- [ ] Browser window showing cookie consent interaction
- [ ] Split-screen showing multiple browsers running simultaneously
- [ ] Diagram animations for task group flow
- [ ] Config file editing with toml syntax highlighting

### Code Samples to Have Ready
- Pre-written task files for smooth demonstration
- Config files with different settings
- Error scenarios for troubleshooting section

### Timing Tips
- Pause after commands to let viewers see output
- Use zoom for small text (config files, error messages)
- Speed up build process with pre-compiled binary
- Have browser windows pre-sized for recording

---

*End of Tutorial Script*
