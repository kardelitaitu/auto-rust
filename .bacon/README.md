# Bacon Autonomous Coding System v2.0

An enhanced autonomous coding system for the Auto-Rust browser automation framework. Bacon monitors, analyzes, and prepares verified patch candidates for review.

## 🚀 Overview

Bacon implements a 4-agent pipeline that automatically:
- **Observes**: Extracts compiler warnings and errors
- **Strategizes**: Analyzes problems and creates technical specifications
- **Codes**: Generates minimal, audit-ready patch files
- **Audits**: Validates changes for safety and compliance, then queues verified patches

## 📁 Enhanced Architecture

```
.bacon/
├── bacon.toml                    # Enhanced configuration with monitoring & safety
├── roles/                         # Enhanced agent definitions
│   ├── 01_bacon-observer.md      # Structured problem analysis
│   ├── 02_bacon-strategy.md      # Technical specifications
│   ├── 03_bacon-coder.md         # Minimal code generation
│   └── 04_bacon-auditor.md       # Comprehensive validation
├── scripts/                       # Enhanced orchestration scripts
│   ├── bacon-config.sh          # Configuration management
│   ├── bacon-orchestrate.sh     # Main agentic loop with error handling
│   ├── bacon-apply-shadow.sh    # Safe shadow workspace management
│   ├── bacon-sentinel.sh        # Hotspot detection
│   ├── bacon-observer.sh        # Enhanced problem analysis
│   ├── bacon-strategist.sh      # Strategy generation
│   ├── bacon-coder.sh           # Code generation
│   ├── bacon-auditor.sh         # Comprehensive auditing
│   └── bacon-manager.ps1        # PowerShell management dashboard
└── sessions/                      # Working directory and logs
```

## 🛡️ Safety & Security Features

### Multi-Layer Validation
- **Security Audit**: Detects dangerous patterns, hardcoded secrets, unsafe blocks
- **Browser Compatibility**: Ensures no fingerprinting or User-Agent modifications
- **Performance Validation**: Checks for regressions and memory leaks
- **Code Quality**: Validates style, documentation, and error handling

### Shadow Workspace Testing
- **Isolated Environment**: Changes tested in temporary git clones
- **Review Queue**: Verified patches are stored under `.bacon/sessions/approved_patches/`
- **Comprehensive Verification**: Compilation, testing, and integration checks

### Production Safeguards
- **Context Isolation**: Maintains browser profile separation
- **Memory Safety**: Prevents leaks in long-running sessions
- **Fingerprinting Protection**: Never modifies detection mechanisms

## 📊 Monitoring & Metrics

### Real-time Metrics
- **Event Tracking**: All agent actions logged with timestamps
- **Success Rates**: Track effectiveness of each agent
- **Performance Metrics**: Compilation times, test results
- **Health Monitoring**: System status and degradation warnings

### Logging System
- **Structured Logs**: JSON-formatted logs for easy analysis
- **Multiple Levels**: DEBUG, INFO, WARN, ERROR with rotation
- **Agent-Specific Logs**: Separate logs for each agent component

## 🎛️ Management Interface

### PowerShell Dashboard
```powershell
# Check system status
.\.bacon\scripts\bacon-manager.ps1 -Action status

# Start autonomous coding
.\.bacon\scripts\bacon-manager.ps1 -Action start

# View metrics
.\.bacon\scripts\bacon-manager.ps1 -Action metrics

# Run system tests
.\.bacon\scripts\bacon-manager.ps1 -Action test

# Apply newest approved patch candidate
.\.bacon\scripts\bacon-manager.ps1 -Action apply-approved -RunCheck
```

### Environment Variables
```bash
# Configuration overrides
export BACON_CYCLE_INTERVAL=15        # Check interval (seconds)
export BACON_MAX_CYCLES=100           # Maximum cycles (0 = infinite)
export BACON_LOG_LEVEL=debug          # Logging verbosity
export BACON_ENABLE_METRICS=true      # Metrics collection
export BACON_SHADOW_CLEANUP=true     # Automatic cleanup
export BACON_AUTO_APPLY=false         # Auto-apply is disabled by default
export BACON_REQUIRE_FULL_CHECK=true  # Auto-apply must pass .\check.ps1
```

## 🔧 Configuration

### Enhanced bacon.toml
```toml
[global]
log_level = "info"
max_concurrent_jobs = 3
timeout_seconds = 300
retry_attempts = 2

[monitoring]
enable_metrics = true
metrics_file = ".bacon/sessions/metrics.json"
log_file = ".bacon/sessions/bacon.log"
max_log_size_mb = 100

[safety]
enable_shadow_testing = true
shadow_workspace_dir = "/tmp/norino_shadow_"
max_shadow_age_hours = 24
enable_rollback = true
rollback_depth = 10
enable_auto_apply = false
require_full_check_for_auto_apply = true

[ai_providers]
gemini_model = "gemini-pro"
codex_model = "codex-5.5"
audit_model = "codex-5.4mini"
request_timeout_seconds = 30
max_tokens_per_request = 4000
```

## 🚦 Usage Examples

### Basic Operations
```bash
# Start the autonomous system
./.bacon/scripts/bacon-orchestrate.sh

# Monitor status
./.bacon/scripts/bacon-manager.ps1 -Action status

# View recent activity
./.bacon/scripts/bacon-manager.ps1 -Action logs

# Cleanup old files
./.bacon/scripts/bacon-manager.ps1 -Action cleanup

# Validate and apply the newest approved patch
./.bacon/scripts/bacon-apply-approved.sh --latest --run-check
```

### Auto-Apply Policy

Auto-apply is allowed only when all of these are true:
- `BACON_AUTO_APPLY=true`
- `BACON_REQUIRE_FULL_CHECK=true`
- `.\\check.ps1` exists and passes after applying the patch
- the working tree is clean before applying

If any condition fails, Bacon keeps the patch in `.bacon/sessions/approved_patches/` for manual review.

### Advanced Configuration
```bash
# Custom cycle interval
BACON_CYCLE_INTERVAL=30 ./bacon/scripts/bacon-orchestrate.sh

# Debug mode
BACON_LOG_LEVEL=debug ./bacon/scripts/bacon-orchestrate.sh

# Limited run (100 cycles)
BACON_MAX_CYCLES=100 ./bacon/scripts/bacon-orchestrate.sh
```

## 🔄 Agent Pipeline Flow

### 1. Observer Agent
- **Input**: Raw compiler output (cargo clippy JSON)
- **Processing**: Extract structured problems with context
- **Output**: JSON problem brief with locations and categories

### 2. Strategy Agent
- **Input**: Structured problem analysis
- **Processing**: Analyze root causes, design solutions
- **Output**: Technical specifications with priorities

### 3. Coder Agent
- **Input**: Technical specifications
- **Processing**: Generate minimal, audit-ready patches
- **Output**: Git diff files with safety checks

### 4. Auditor Agent
- **Input**: Code patches
- **Processing**: Security, performance, and compatibility validation
- **Output**: PASS/FAIL decision with detailed reasoning

## 🛠️ Development & Testing

### System Tests
```bash
# Run comprehensive system tests
./.bacon/scripts/bacon-manager.ps1 -Action test

# Test individual components
./.bacon/scripts/bacon-observer.sh test_input.json test_output.json
./.bacon/scripts/bacon-strategist.sh test_problems.json strategy.json
./.bacon/scripts/bacon-coder.sh strategy.json patch.diff
./.bacon/scripts/bacon-auditor.sh patch.diff audit_result.json
```

### Debug Mode
```bash
# Enable verbose logging
export BACON_LOG_LEVEL=debug

# Run with detailed output
./.bacon/scripts/bacon-orchestrate.sh 2>&1 | tee debug.log
```

## 📈 Performance & Scalability

### Optimized for Ryzen 9 7950X
- **Async Execution**: Tokio-based concurrency
- **Parallel Processing**: Multiple agent instances
- **Resource Management**: Efficient memory and CPU usage
- **Browser Scaling**: Support for 20+ concurrent sessions

### Performance Metrics
- **Startup Time**: <2 seconds including browser discovery
- **Memory Footprint**: ~50-200 MB for full system
- **Throughput**: ~50 tasks/sec with 20 sessions
- **Latency**: <100ms for hotspot detection

## 🔒 Security Considerations

### Threat Model
- **Code Injection**: Shadow workspace isolation prevents malicious code execution
- **Data Leakage**: Strict browser context separation
- **Supply Chain**: Minimal external dependencies, vetted AI providers
- **Persistence**: No credential storage, environment-based configuration

### Security Controls
- **Input Validation**: All patches scanned for dangerous patterns
- **Sandboxing**: Changes tested in isolated environments
- **Audit Trail**: Complete logging of all autonomous actions
- **Rollback**: Instant revert capability for any change

## 🚨 Troubleshooting

### Common Issues
```bash
# Check prerequisites
./.bacon/scripts/bacon-manager.ps1 -Action test

# View detailed logs
./.bacon/scripts/bacon-manager.ps1 -Action logs

# Clean up corrupted state
./.bacon/scripts/bacon-manager.ps1 -Action cleanup

# Reset runtime state
rm -rf .bacon/sessions/* .bacon/test_runs/* && ./.bacon/scripts/bacon-manager.ps1 -Action test
```

### Health Monitoring
```bash
# System health check
./.bacon/scripts/bacon-manager.ps1 -Action status

# Performance metrics
./.bacon/scripts/bacon-manager.ps1 -Action metrics

# Active processes
ps aux | grep bacon
```

## 📚 Version History

### v2.0 (Current)
- Enhanced error handling and logging
- Comprehensive security auditing
- Shadow workspace safety mechanisms
- PowerShell management dashboard
- Metrics and monitoring system
- Rollback and recovery capabilities

### v1.0 (Original)
- Basic 4-agent pipeline
- Simple shell script orchestration
- Minimal configuration

## 🤝 Contributing

When modifying Bacon:
1. **Test Thoroughly**: Run `bacon-manager.ps1 -Action test`
2. **Security Review**: Ensure no fingerprinting or safety regressions
3. **Documentation**: Update this README and role definitions
4. **Backward Compatibility**: Maintain existing configuration format

## 📄 License

Part of the Auto-Rust project. See main project LICENSE for details.
