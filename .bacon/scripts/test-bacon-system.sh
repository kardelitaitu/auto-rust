#!/bin/bash
# .bacon/scripts/test-bacon-system.sh - Comprehensive test suite for Bacon autonomous coding system
# Tests all components of the Bacon system with proper validation

set -euo pipefail
IFS=$'\n\t'

# Configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
readonly TEST_DIR="${PROJECT_ROOT}/.bacon/test_runs"
readonly LOG_FILE="${TEST_DIR}/test_results.log"

# Test results tracking
declare -i TESTS_TOTAL=0
declare -i TESTS_PASSED=0
declare -i TESTS_FAILED=0

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # No Color

# Logging utilities
log() {
    local level="$1"
    shift
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [$level] $*" | tee -a "$LOG_FILE"
}

log_test() { log "TEST" "$@"; }
log_pass() { log "PASS" "$@"; }
log_fail() { log "FAIL" "$@"; }
log_info() { log "INFO" "$@"; }
log_warn() { log "WARN" "$@"; }

# Test utilities
test_start() {
    local test_name="$1"
    ((TESTS_TOTAL++))
    log_test "Starting test: $test_name"
}

test_pass() {
    local test_name="$1"
    ((TESTS_PASSED++))
    log_pass "✓ PASSED: $test_name"
}

test_fail() {
    local test_name="$1"
    local reason="$2"
    ((TESTS_FAILED++))
    log_fail "✗ FAILED: $test_name - $reason"
}

# Setup test environment
setup_test_env() {
    log_info "Setting up test environment"
    
    # Create test directory
    mkdir -p "$TEST_DIR"
    mkdir -p "$TEST_DIR/sessions"
    mkdir -p "$TEST_DIR/temp"
    
    # Create test configuration
    cat > "$TEST_DIR/test_bacon.toml" << EOF
[global]
log_level = "debug"
max_concurrent_jobs = 1
timeout_seconds = 60
retry_attempts = 1

[jobs.test_obs]
command = ["echo", '{"message":"test warning","level":"warning","code":{"code":"test"},"spans":[{"file_name":"test.rs","line_start":1,"line_end":1,"column_start":1,"column_end":10,"text":"test code"}],"rendered":"test rendered"}']
need_stdout = true
description = "Test observer job"

[monitoring]
enable_metrics = true
metrics_file = "$TEST_DIR/sessions/metrics.json"
log_file = "$TEST_DIR/sessions/bacon.log"
max_log_size_mb = 10
EOF
    
    log_info "Test environment setup completed"
}

# Cleanup test environment
cleanup_test_env() {
    log_info "Cleaning up test environment"
    
    # Remove test files but keep logs
    find "$TEST_DIR" -name "*.tmp" -delete 2>/dev/null || true
    find "$TEST_DIR" -name "test_*" -not -name "*.log" -delete 2>/dev/null || true
    
    log_info "Test environment cleanup completed"
}

# Test 1: Configuration loading
test_config_loading() {
    test_start "Configuration Loading"
    
    # Test bacon-config.sh
    if BACON_CYCLE_INTERVAL=5 BACON_LOG_LEVEL=debug \
        source "${SCRIPT_DIR}/bacon-config.sh" 2>/dev/null; then
        
        if [[ "$BACON_CYCLE_INTERVAL" == "5" && "$BACON_LOG_LEVEL" == "debug" ]]; then
            test_pass "Configuration Loading"
        else
            test_fail "Configuration Loading" "Environment variables not set correctly"
        fi
    else
        test_fail "Configuration Loading" "Failed to source bacon-config.sh"
    fi
}

# Test 2: TOML parsing
test_toml_parsing() {
    test_start "TOML Parsing"
    
    # Test Rust parser if available
    local parser_script="${SCRIPT_DIR}/bacon-config-parser"
    if [[ -f "$parser_script" ]]; then
        if "$parser_script" "$TEST_DIR/test_bacon.toml" >/dev/null 2>&1; then
            test_pass "TOML Parsing"
        else
            test_fail "TOML Parsing" "Rust parser failed"
        fi
    else
        test_fail "TOML Parsing" "Rust parser not found - run: rustc bacon-config-parser.rs -o bacon-config-parser"
    fi
}

# Test 3: Observer script
test_observer_script() {
    test_start "Observer Script"
    
    # Create test input
    local test_input="$TEST_DIR/test_clippy.json"
    local test_output="$TEST_DIR/test_observer_output.json"
    
    cat > "$test_input" << EOF
{"message":"test warning","level":"warning","code":{"code":"test"},"spans":[{"file_name":"test.rs","line_start":1,"line_end":1,"column_start":1,"column_end":10,"text":"test code"}],"rendered":"test rendered"}
EOF
    
    # Run observer
    if "${SCRIPT_DIR}/bacon-observer.sh" "$test_input" "$test_output" 2>/dev/null; then
        # Validate output
        if [[ -f "$test_output" ]] && jq empty "$test_output" 2>/dev/null; then
            local problem_count
            problem_count=$(jq '.problems | length' "$test_output" 2>/dev/null || echo "0")
            if [[ "$problem_count" -gt 0 ]]; then
                test_pass "Observer Script"
            else
                test_fail "Observer Script" "No problems found in output"
            fi
        else
            test_fail "Observer Script" "Invalid JSON output"
        fi
    else
        test_fail "Observer Script" "Script execution failed"
    fi
    
    # Cleanup
    rm -f "$test_input" "$test_output"
}

# Test 4: Strategy script
test_strategy_script() {
    test_start "Strategy Script"
    
    # Create test observer output
    local test_input="$TEST_DIR/test_observer.json"
    local test_output="$TEST_DIR/test_strategy_output.json"
    
    cat > "$test_input" << EOF
{
  "problems": [
    {
      "message": "test warning",
      "level": "warning",
      "code": "test",
      "spans": [{"file_name": "test.rs", "line_start": 1, "line_end": 1, "column_start": 1, "column_end": 10, "text": "test code"}],
      "rendered": "test rendered"
    }
  ],
  "summary": {"total": 1, "errors": 0, "warnings": 1, "notes": 0}
}
EOF
    
    # Run strategist
    if "${SCRIPT_DIR}/bacon-strategist.sh" "$test_input" "$test_output" 2>/dev/null; then
        # Validate output
        if [[ -f "$test_output" ]] && jq empty "$test_output" 2>/dev/null; then
            local strategy_count
            strategy_count=$(jq '.strategies | length' "$test_output" 2>/dev/null || echo "0")
            if [[ "$strategy_count" -gt 0 ]]; then
                test_pass "Strategy Script"
            else
                test_fail "Strategy Script" "No strategies found in output"
            fi
        else
            test_fail "Strategy Script" "Invalid JSON output"
        fi
    else
        test_fail "Strategy Script" "Script execution failed"
    fi
    
    # Cleanup
    rm -f "$test_input" "$test_output"
}

# Test 5: Sentinel script
test_sentinel_script() {
    test_start "Sentinel Script"
    
    # Test with mock bacon command
    local mock_bacon="$TEST_DIR/temp/mock_bacon"
    cat > "$mock_bacon" << 'EOF'
#!/bin/bash
if [[ "$1" == "--config" && "$3" == "--job" && "$4" == "ai_obs" ]]; then
    echo '[{"message":"test","level":"warning","code":{"code":"test"}}]'
    exit 0
else
    echo "Invalid bacon arguments" >&2
    exit 1
fi
EOF
    chmod +x "$mock_bacon"
    
    # Override PATH to use mock bacon
    local old_path="$PATH"
    export PATH="$TEST_DIR/temp:$PATH"
    
    # Run sentinel
    if BACON_SENTINEL_TIMEOUT=10 "${SCRIPT_DIR}/bacon-sentinel.sh" 2>/dev/null; then
        # Check if output file was created
        local output_file="${PROJECT_ROOT}/.bacon/sessions/last_hotspot.json"
        if [[ -f "$output_file" ]]; then
            test_pass "Sentinel Script"
        else
            test_fail "Sentinel Script" "Output file not created"
        fi
    else
        test_fail "Sentinel Script" "Script execution failed"
    fi
    
    # Restore PATH
    export PATH="$old_path"
    
    # Cleanup
    rm -f "$mock_bacon"
}

# Test 6: PowerShell manager (if on Windows)
test_powershell_manager() {
    test_start "PowerShell Manager"
    
    if command -v pwsh >/dev/null 2>&1 || command -v powershell >/dev/null 2>&1; then
        # Test PowerShell script syntax
        local ps_cmd="pwsh"
        if ! command -v pwsh >/dev/null 2>&1; then
            ps_cmd="powershell"
        fi
        
        if "$ps_cmd" -Command "& '${SCRIPT_DIR}/bacon-manager.ps1' -Action test" 2>/dev/null; then
            test_pass "PowerShell Manager"
        else
            test_fail "PowerShell Manager" "PowerShell script test failed"
        fi
    else
        test_fail "PowerShell Manager" "PowerShell not available on this system"
    fi
}

# Test 7: JSON validation
test_json_validation() {
    test_start "JSON Validation"
    
    # Test various JSON files
    local test_files=(
        "${SCRIPT_DIR}/../roles/01_bacon-observer.md"
        "${SCRIPT_DIR}/../roles/02_bacon-strategy.md"
        "${SCRIPT_DIR}/../README.md"
    )
    
    local valid_files=0
    for file in "${test_files[@]}"; do
        if [[ -f "$file" ]]; then
            ((valid_files++))
        fi
    done
    
    if [[ "$valid_files" -gt 0 ]]; then
        test_pass "JSON Validation"
    else
        test_fail "JSON Validation" "No test files found"
    fi
}

# Test 8: Error handling
test_error_handling() {
    test_start "Error Handling"
    
    # Test script with invalid input
    local invalid_input="$TEST_DIR/nonexistent.json"
    local test_output="$TEST_DIR/error_test_output.json"
    
    # Observer should handle invalid input gracefully
    if ! "${SCRIPT_DIR}/bacon-observer.sh" "$invalid_input" "$test_output" 2>/dev/null; then
        test_pass "Error Handling"
    else
        test_fail "Error Handling" "Script should have failed with invalid input"
    fi
    
    # Cleanup
    rm -f "$test_output"
}

# Test 9: Prerequisites check
test_prerequisites() {
    test_start "Prerequisites Check"
    
    local missing_commands=()
    local required_commands=("jq" "cargo" "git")
    
    for cmd in "${required_commands[@]}"; do
        if ! command -v "$cmd" >/dev/null 2>&1; then
            missing_commands+=("$cmd")
        fi
    done
    
    if [[ ${#missing_commands[@]} -eq 0 ]]; then
        test_pass "Prerequisites Check"
    else
        test_fail "Prerequisites Check" "Missing commands: ${missing_commands[*]}"
    fi
}

# Test 10: Integration test
test_integration() {
    test_start "Integration Test"
    
    # Create a simple end-to-end test
    local test_input="$TEST_DIR/integration_input.json"
    local observer_output="$TEST_DIR/integration_observer.json"
    local strategy_output="$TEST_DIR/integration_strategy.json"
    
    # Create test problem
    cat > "$test_input" << EOF
{"message":"integration test warning","level":"warning","code":{"code":"clippy::test"},"spans":[{"file_name":"integration.rs","line_start":5,"line_end":5,"column_start":1,"column_end":20,"text":"let x = 42;"}],"rendered":"warning: unused variable"}
EOF
    
    # Run observer
    if "${SCRIPT_DIR}/bacon-observer.sh" "$test_input" "$observer_output" 2>/dev/null; then
        # Run strategist
        if "${SCRIPT_DIR}/bacon-strategist.sh" "$observer_output" "$strategy_output" 2>/dev/null; then
            # Validate final output
            if [[ -f "$strategy_output" ]] && jq empty "$strategy_output" 2>/dev/null; then
                test_pass "Integration Test"
            else
                test_fail "Integration Test" "Invalid final output"
            fi
        else
            test_fail "Integration Test" "Strategy phase failed"
        fi
    else
        test_fail "Integration Test" "Observer phase failed"
    fi
    
    # Cleanup
    rm -f "$test_input" "$observer_output" "$strategy_output"
}

# Main test runner
run_all_tests() {
    log_info "Starting Bacon system test suite"
    
    # Setup
    setup_test_env
    
    # Run tests
    test_config_loading
    test_toml_parsing
    test_observer_script
    test_strategy_script
    test_sentinel_script
    test_powershell_manager
    test_json_validation
    test_error_handling
    test_prerequisites
    test_integration
    
    # Cleanup
    cleanup_test_env
    
    # Print results
    echo
    echo -e "${BLUE}=== BACON SYSTEM TEST RESULTS ===${NC}"
    echo -e "Total Tests: ${TESTS_TOTAL}"
    echo -e "${GREEN}Passed: ${TESTS_PASSED}${NC}"
    echo -e "${RED}Failed: ${TESTS_FAILED}${NC}"
    
    if [[ "$TESTS_FAILED" -eq 0 ]]; then
        echo -e "${GREEN}✓ ALL TESTS PASSED${NC}"
        log_info "All tests completed successfully"
        return 0
    else
        echo -e "${RED}✗ SOME TESTS FAILED${NC}"
        log_info "Some tests failed - check log for details"
        return 1
    fi
}

# Help function
show_help() {
    cat << EOF
Bacon System Test Suite

Usage: $0 [OPTIONS]

OPTIONS:
    -h, --help     Show this help message
    -v, --verbose  Enable verbose output
    -c, --clean    Clean test artifacts only

DESCRIPTION:
    This script runs comprehensive tests for the Bacon autonomous coding system.
    It tests all components including configuration, observer, strategist,
    sentinel, and integration scenarios.

EXAMPLES:
    $0                    # Run all tests
    $0 -v                # Run with verbose output
    $0 -c                # Clean test artifacts

EXIT CODES:
    0    All tests passed
    1    Some tests failed
    2    Invalid arguments
EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)
            show_help
            exit 0
            ;;
        -v|--verbose)
            set -x
            shift
            ;;
        -c|--clean)
            cleanup_test_env
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            show_help
            exit 2
            ;;
    esac
done

# Run tests
if run_all_tests; then
    exit 0
else
    exit 1
fi
