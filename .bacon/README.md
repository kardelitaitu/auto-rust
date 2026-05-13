# Bacon Autonomous Coding System v2.0

An enhanced autonomous coding system for the Auto-Rust browser automation framework. Bacon monitors, analyzes, and prepares verified patch candidates for review.

## 🚀 Overview

Bacon implements a 4-agent pipeline that automatically:
- **Observes**: Extracts compiler warnings and errors, or reads pending specs.
- **Strategizes**: Analyzes problems and creates technical specifications in Markdown.
- **Codes**: Generates minimal, audit-ready SEARCH/REPLACE blocks.
- **Audits**: Validates changes for safety and semantic compliance, then archives verified patches.

## 📁 Enhanced Architecture

```
.bacon/
├── bacon.toml                    # Enhanced configuration with monitoring & safety
├── roles/                         # Enhanced agent definitions
│   ├── 01_bacon-observer.md      # Structured problem analysis
│   ├── 02_bacon-strategy.md      # Technical specifications
│   ├── 03_bacon-coder.md         # SEARCH/REPLACE block generation
│   └── 04_bacon-auditor.md       # Comprehensive semantic validation
├── scripts/                       # Orchestration utilities
│   └── bacon-manager.ps1        # PowerShell management dashboard
└── sessions/                      # Working directory and logs
```

## 🛡️ Safety & Security Features

### Multi-Layer Validation
- **Sandboxed Compilation**: The pipeline validates code compilation (`cargo check`) and formatting in an isolated shadow worktree before applying.
- **Retry with Feedback**: If tests or compilation fail, compiler errors are fed back to the Coder LLM to self-correct up to 3 times.
- **Semantic Auditing**: An Auditor LLM checks the generated code against the original spec to prevent hallucinated logic.
- **Code Quality**: Validates style, documentation, and error handling (`cargo clippy`, `cargo fmt`).

### Shadow Workspace Testing
- **Isolated Environment**: Changes are generated as patches and tested in temporary git clones.
- **Review Queue**: Verified patches are stored under `.bacon/sessions/approved_patches/`.
- **Comprehensive Verification**: Full integration checks are passed before the human confirmation gate.

### Production Safeguards
- **Context Isolation**: Maintains browser profile separation.
- **Memory Safety**: Prevents leaks in long-running sessions.
- **Fingerprinting Protection**: Never modifies browser detection mechanisms.

## 📊 Enhanced Monitoring & Metrics

### Real-time Metrics
- **Event Tracking**: All agent actions are tracked within the pipeline.
- **Health Monitoring**: System status with health scoring (0-100).
- **Performance Metrics**: Stage duration, retry counts, success rates.
- **Resource Usage**: Memory, disk, and process monitoring.
- **Code Impact**: Lines changed, files modified, coverage impact.

### Alert System
- **Configurable Thresholds**: Set alerts for success rates, error rates, queue depth.
- **Health Scoring**: Automatic health assessment with recommendations.
- **Proactive Monitoring**: Early warning for system issues.

### Logging System
- **Structured Logs**: Native rust `log` and `env_logger` integration.
- **Multiple Levels**: DEBUG, INFO, WARN, ERROR, CRITICAL outputs.
- **Security Audit Trail**: Separate logging for security-relevant actions.
- **JSON Metrics**: Structured metrics in JSON Lines format.

### Reporting & Dashboards
- **HTML Reports**: Professional system reports with health metrics.
- **Web Dashboard**: Real-time monitoring via HTTP dashboard.
- **Historical Archives**: Daily metric archives for trend analysis.

## 🎛️ Management Interface

### Bacon Supervisor (`bacon`)
The primary engine for the autonomous workflow is the native Rust `bacon` binary.

```bash
# Start the autonomous system (default)
cargo run --bin bacon

# Start with a specific guided prompt
cargo run --bin bacon -- -p "Fix the deprecated unused variables in mod.rs"

# Skip strategist and auditor for fast, trivial fixes
cargo run --bin bacon -- --fast

# Automatic mode (skip confirmation gates)
cargo run --bin bacon -- --auto

# Auto-apply approved patches to the working tree
cargo run --bin bacon -- --auto-apply

# Dry run (no changes applied)
cargo run --bin bacon -- --dry-run

# Run the test harness
cargo run --bin bacon -- test
```

### PowerShell Dashboard
```powershell
# Check system status
.\.bacon\scripts\bacon-manager.ps1 status

# Start autonomous coding
.\.bacon\scripts\bacon-manager.ps1 start

# View metrics
.\.bacon\scripts\bacon-manager.ps1 metrics

# Run system tests
.\.bacon\scripts\bacon-manager.ps1 test

# Generate HTML report
.\.bacon\scripts\bacon-manager.ps1 report

# Rotate API keys securely
.\.bacon\scripts\bacon-manager.ps1 rotate-keys

# Start web dashboard
.\.bacon\scripts\bacon-manager.ps1 dashboard

# Clean up old files
.\.bacon\scripts\bacon-manager.ps1 cleanup

# Apply approved patches
.\.bacon\scripts\bacon-manager.ps1 apply-approved
```

## 📖 Enhanced Workflow Documentation

For complete workflow details, see `.bacon/.bacon-workflow.md` which includes:

- **Getting Started Guide**: Installation, prerequisites, basic usage
- **Error Recovery**: Crash recovery, network failure handling, timeout management
- **Security Guidelines**: API key management, sensitive code review, audit trails
- **Monitoring & Metrics**: Real-time tracking, alert configuration, dashboards
- **Testing Integration**: Coverage requirements, continuous validation
- **Troubleshooting**: Common issues, debug mode, recovery commands
- **Practical Examples**: Real-world usage scenarios with command examples

## 🔧 Configuration

### `bacon.toml`
Bacon reads `.bacon/bacon.toml` to configure the supervisor and agent routing.

```toml
[workflow]
stages = ["observer", "strategist", "coder", "auditor"]
auto_approve = false
fast_mode = false
dry_run = false
max_retries = 3
timeout_seconds = 300

[global]
log_level = "info"
max_concurrent_jobs = 3
retry_attempts = 2

[monitoring.alerts]
success_rate_below = 80
avg_duration_above = 300
error_rate_above = 20
queue_depth_above = 10
memory_usage_above = 2048

[agents.observer]
type = "local"
provider = "ollama"
model = "llama3.2:3b"
temperature = 0.2
```

## 🔄 Agent Pipeline Flow

### 1. Observer Agent
- **Input**: Project directory tree and active specs.
- **Processing**: Identifies the next actionable improvement or active spec.
- **Output**: Plain-text description of the targeted improvement.

### 2. Strategy Agent
- **Input**: Problem description from the Observer.
- **Processing**: Analyzes root causes and designs a step-by-step solution.
- **Output**: Structured Markdown spec package stored in `docs/specs/_active/`.

### 3. Coder Agent
- **Input**: Markdown spec files (`plan.md`, `baseline.md`, etc.) and compiler errors (if retrying).
- **Processing**: Modifies code via SEARCH/REPLACE blocks. Changes are automatically validated in a cloned temp worktree.
- **Output**: Verified patches applied to the codebase.

### 4. Auditor Agent
- **Input**: The implemented codebase diff and the original Spec.
- **Processing**: Validates that all acceptance criteria are met and no out-of-scope logic was added.
- **Output**: PASS (moves spec to `_done/`) or FAIL (leaves in `_active/` for human review).

## 🛠️ Development & Testing

### System Tests
```bash
# Run comprehensive system tests via the PowerShell dashboard
.\.bacon\scripts\bacon-manager.ps1 -Action test

# Run the Rust-native supervisor test harness
cargo run --bin bacon -- test
```

### Debug Mode
```bash
# Enable verbose logging through standard Rust RUST_LOG
RUST_LOG=debug cargo run --bin bacon
```

## 🔒 Security Considerations

### Threat Model
- **Code Injection**: Shadow workspace isolation prevents untested, hallucinated code execution on the main branch.
- **Supply Chain**: Bacon utilizes local LLMs (like Ollama) preventing source code leakage to external third-party cloud APIs.

### Security Controls
- **Input Validation**: Code changes must match exact SEARCH/REPLACE blocks to apply.
- **Sandboxing**: Changes are fundamentally tested in isolated temporary Git worktrees.
- **Rollback**: Instant revert capability if validation tests (`check-fast.ps1`) fail during auto-apply.
- **API Key Management**: Secure environment variable storage with rotation support.
- **Sensitive Code Review**: Additional validation for security-critical paths.
- **Rate Limiting**: Protection against API abuse for external LLM providers.
- **Security Scanning**: Automatic security checks for sensitive code changes.
- **Audit Trail**: Comprehensive logging of security-relevant actions.

## 📚 Version History

### v2.0 (Current)
- Complete rewrite moving orchestration to the native Rust `bacon` binary.
- Removal of fragile bash scripts and unified diff requirements.
- Implemented shadow workspace validation loop with LLM compilation error feedback.
- Migrated code patching to robust SEARCH/REPLACE blocks.
- PowerShell management dashboard available.

### v1.0 (Original)
- Basic 4-agent pipeline.
- Bash/script-based orchestration.
- Required exact unified git diffs.

## 🤝 Contributing

When modifying Bacon:
1. **Test Thoroughly**: Run `cargo run --bin bacon -- test`.
2. **Security Review**: Ensure no safety regressions or bypassing of the Shadow Worktree gate.
3. **Documentation**: Update this README and the Markdown role prompts in `.bacon/roles/`.

## 📄 License

Part of the Auto-Rust project. See main project LICENSE for details.
