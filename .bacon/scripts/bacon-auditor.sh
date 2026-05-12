#!/bin/bash
# .bacon/scripts/bacon-auditor.sh - Enhanced Auditor Agent
# Performs comprehensive code audits and security checks

set -euo pipefail
IFS=$'\n\t'

# Configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && (pwd -W 2>/dev/null || pwd))"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && (pwd -W 2>/dev/null || pwd))"
readonly ROLES_DIR="${PROJECT_ROOT}/.bacon/roles"
readonly SESSIONS_DIR="${PROJECT_ROOT}/.bacon/sessions"
readonly LOG_FILE="${PROJECT_ROOT}/.bacon/sessions/auditor.log"

# Load configuration
source "${SCRIPT_DIR}/bacon-config.sh" 2>/dev/null || true

# Logging utilities
log() {
    local level="$1"
    shift
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [$level] $*" | tee -a "$LOG_FILE"
}

log_error() { log "ERROR" "$@"; }
log_warn() { log "WARN" "$@"; }
log_info() { log "INFO" "$@"; }
log_debug() { log "DEBUG" "$@"; }

json_array() {
    if [[ "$#" -eq 0 ]]; then
        echo "[]"
    else
        printf '%s\n' "$@" | jq -R . | jq -s .
    fi
}

# Security checks
security_audit() {
    local patch_file="$1"
    local audit_result="$2"

    log_info "Performing security audit"

    local security_issues=0
    local security_warnings=()

    # Check for dangerous patterns
    local dangerous_patterns=(
        "unsafe"
        "transmute"
        "ptr::"
        "mem::transmute"
        "std::mem::forget"
        "std::ptr::"
        "libc::"
        "eval("
        "exec("
        "system("
    )

    for pattern in "${dangerous_patterns[@]}"; do
        if grep -q "$pattern" "$patch_file" 2>/dev/null; then
            security_issues=$((security_issues + 1))
            security_warnings+=("Dangerous pattern detected: $pattern")
        fi
    done

    # Check for hardcoded secrets
    local secret_patterns=(
        "password"
        "secret"
        "token"
        "api_key"
        "private_key"
        "credential"
    )

    for pattern in "${secret_patterns[@]}"; do
        if grep -i "$pattern" "$patch_file" 2>/dev/null | grep -q "="; then
            security_issues=$((security_issues + 1))
            security_warnings+=("Potential hardcoded secret: $pattern")
        fi
    done

    # Check for network operations
    if grep -q "http::\|reqwest::\|curl::" "$patch_file" 2>/dev/null; then
        security_warnings+=("Network operations detected - review security implications")
    fi

    local risk_level="low"
    if [[ "$security_issues" -gt 2 ]]; then
        risk_level="high"
    elif [[ "$security_issues" -gt 0 ]]; then
        risk_level="medium"
    fi
    local security_warnings_json
    security_warnings_json=$(json_array "${security_warnings[@]}")

    # Write security audit results
    cat > "${audit_result}.security" << EOF
{
  "security_issues": $security_issues,
  "warnings": $security_warnings_json,
  "risk_level": "$risk_level"
}
EOF

    if [[ "$security_issues" -eq 0 ]]; then
        log_info "Security audit passed"
        return 0
    else
        log_warn "Security audit found $security_issues issues"
        return 1
    fi
}

# Code quality checks
quality_audit() {
    local patch_file="$1"
    local audit_result="$2"

    log_info "Performing code quality audit"

    local quality_issues=0
    local quality_warnings=()

    # Check for TODO/FIXME comments
    if grep -q "TODO\|FIXME\|HACK\|XXX" "$patch_file" 2>/dev/null; then
        quality_issues=$((quality_issues + 1))
        quality_warnings+=("Contains TODO/FIXME comments")
    fi

    # Check for debug prints
    if grep -q "println!\|dbg!\|debug!\|eprintln!" "$patch_file" 2>/dev/null; then
        quality_issues=$((quality_issues + 1))
        quality_warnings+=("Contains debug prints")
    fi

    # Check for panic statements
    if grep -q "panic!\|unwrap()" "$patch_file" 2>/dev/null; then
        quality_issues=$((quality_issues + 1))
        quality_warnings+=("Contains panic statements or unwrap() calls")
    fi

    # Check for long lines (basic check)
    while IFS= read -r line; do
        if [[ ${#line} -gt 100 ]]; then
        quality_issues=$((quality_issues + 1))
            quality_warnings+=("Contains long lines (>100 chars)")
            break
        fi
    done < "$patch_file"

    local score=100
    if [[ "$quality_issues" -gt 2 ]]; then
        score=60
    elif [[ "$quality_issues" -gt 0 ]]; then
        score=80
    fi
    local quality_warnings_json
    quality_warnings_json=$(json_array "${quality_warnings[@]}")

    # Write quality audit results
    cat > "${audit_result}.quality" << EOF
{
  "quality_issues": $quality_issues,
  "warnings": $quality_warnings_json,
  "score": $score
}
EOF

    if [[ "$quality_issues" -le 2 ]]; then
        log_info "Code quality audit passed"
        return 0
    else
        log_warn "Code quality audit found $quality_issues issues"
        return 1
    fi
}

# Performance checks
performance_audit() {
    local patch_file="$1"
    local audit_result="$2"

    log_info "Performing performance audit"

    local performance_issues=0
    local performance_warnings=()

    # Check for potential performance issues
    local performance_patterns=(
        "clone()"  # Unnecessary cloning
        "collect()"  # Unnecessary collection
        "lock()"  # Potential lock contention
        "sleep("  # Blocking sleep
        "thread::sleep"  # Blocking sleep
    )

    for pattern in "${performance_patterns[@]}"; do
        if grep -q "$pattern" "$patch_file" 2>/dev/null; then
        performance_issues=$((performance_issues + 1))
            performance_warnings+=("Performance concern: $pattern")
        fi
    done

    # Check for allocation patterns
    if grep -q "Vec::new()\|String::new()\|Box::new()" "$patch_file" 2>/dev/null; then
        performance_warnings+=("Contains allocations - review for optimization")
    fi

    local impact="low"
    if [[ "$performance_issues" -gt 2 ]]; then
        impact="high"
    elif [[ "$performance_issues" -gt 0 ]]; then
        impact="medium"
    fi
    local performance_warnings_json
    performance_warnings_json=$(json_array "${performance_warnings[@]}")

    # Write performance audit results
    cat > "${audit_result}.performance" << EOF
{
  "performance_issues": $performance_issues,
  "warnings": $performance_warnings_json,
  "impact": "$impact"
}
EOF

    if [[ "$performance_issues" -le 1 ]]; then
        log_info "Performance audit passed"
        return 0
    else
        log_warn "Performance audit found $performance_issues issues"
        return 1
    fi
}

# Browser-specific checks (for Auto-Rust)
browser_audit() {
    local patch_file="$1"
    local audit_result="$2"

    log_info "Performing browser-specific audit"

    local browser_issues=0
    local browser_warnings=()

    # Check for User-Agent modifications
    if grep -q "user.agent\|User-Agent\|user_agent" "$patch_file" 2>/dev/null; then
        browser_issues=$((browser_issues + 1))
        browser_warnings+=("User-Agent modification detected")
    fi

    # Check for fingerprinting-related code
    local fingerprinting_patterns=(
        "fingerprint"
        "canvas"
        "webgl"
        "timezone"
        "language"
        "screen"
        "resolution"
    )

    for pattern in "${fingerprinting_patterns[@]}"; do
        if grep -i "$pattern" "$patch_file" 2>/dev/null; then
        browser_issues=$((browser_issues + 1))
            browser_warnings+=("Fingerprinting-related code: $pattern")
        fi
    done

    # Check for memory leaks in browser sessions
    if grep -q "session\|browser\|tab" "$patch_file" 2>/dev/null; then
        if ! grep -q "drop\|cleanup\|close" "$patch_file" 2>/dev/null; then
        browser_issues=$((browser_issues + 1))
            browser_warnings+=("Browser session code without cleanup")
        fi
    fi

    local compatibility="compatible"
    if [[ "$browser_issues" -gt 0 ]]; then
        compatibility="needs_review"
    fi
    local browser_warnings_json
    browser_warnings_json=$(json_array "${browser_warnings[@]}")

    # Write browser audit results
    cat > "${audit_result}.browser" << EOF
{
  "browser_issues": $browser_issues,
  "warnings": $browser_warnings_json,
  "compatibility": "$compatibility"
}
EOF

    if [[ "$browser_issues" -eq 0 ]]; then
        log_info "Browser audit passed"
        return 0
    else
        log_warn "Browser audit found $browser_issues issues"
        return 1
    fi
}

# Compile and test the patch
compile_test() {
    local patch_file="$1"
    local audit_result="$2"

    log_info "Performing compile test"

    # Create temporary clone for testing.
    local test_base="${SESSIONS_DIR}/audit_workspaces"
    mkdir -p "$test_base"
    local test_dir="${test_base}/bacon_audit_test_$(date +%s)_${BASHPID}"
    if ! git clone "$PROJECT_ROOT" "$test_dir" 2>/dev/null; then
        log_error "Failed to clone repository for testing"
        rm -rf "$test_dir"
        return 1
    fi

    cd "$test_dir"

    # Apply patch
    if ! git apply "$patch_file" 2>/dev/null; then
        log_error "Failed to apply patch for testing"
        rm -rf "$test_dir"
        return 1
    fi

    # Try to compile
    local compile_result="failed"
    local cargo_target_dir="${PROJECT_ROOT}/target/bacon-audit"
    mkdir -p "$cargo_target_dir"
    if CARGO_TARGET_DIR="$cargo_target_dir" cargo check 2>/dev/null >/dev/null; then
        compile_result="success"
        log_info "Compilation successful"
    else
        log_warn "Compilation failed"
    fi

    # Try to run tests (quick)
    local test_result="skipped"
    if [[ "$compile_result" == "success" ]]; then
        if [[ "${BACON_AUDIT_COMPILE_ONLY:-false}" == "true" ]]; then
            log_info "Tests skipped by BACON_AUDIT_COMPILE_ONLY"
        elif CARGO_TARGET_DIR="$cargo_target_dir" cargo test --lib 2>/dev/null >/dev/null; then
            test_result="success"
            log_info "Tests passed"
        else
            test_result="failed"
            log_warn "Tests failed"
        fi
    fi

    # Write compile test results
    cat > "${audit_result}.compile" << EOF
{
  "compile_result": "$compile_result",
  "test_result": "$test_result",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

    # Cleanup
    cd "$PROJECT_ROOT"
    rm -rf "$test_dir"

    if [[ "$compile_result" == "success" && "$test_result" != "failed" ]]; then
        return 0
    else
        return 1
    fi
}

# Generate final audit report
generate_audit_report() {
    local audit_result="$1"
    local output_file="$2"

    log_info "Generating final audit report"

    # Read individual audit results
    local security_result quality_result performance_result browser_result compile_result

    if [[ -f "${audit_result}.security" ]]; then
        security_result=$(cat "${audit_result}.security")
    else
        security_result='{"security_issues": 0, "warnings": [], "risk_level": "low"}'
    fi

    if [[ -f "${audit_result}.quality" ]]; then
        quality_result=$(cat "${audit_result}.quality")
    else
        quality_result='{"quality_issues": 0, "warnings": [], "score": 100}'
    fi

    if [[ -f "${audit_result}.performance" ]]; then
        performance_result=$(cat "${audit_result}.performance")
    else
        performance_result='{"performance_issues": 0, "warnings": [], "impact": "low"}'
    fi

    if [[ -f "${audit_result}.browser" ]]; then
        browser_result=$(cat "${audit_result}.browser")
    else
        browser_result='{"browser_issues": 0, "warnings": [], "compatibility": "compatible"}'
    fi

    if [[ -f "${audit_result}.compile" ]]; then
        compile_result=$(cat "${audit_result}.compile")
    else
        compile_result='{"compile_result": "skipped", "test_result": "skipped"}'
    fi

    # Calculate overall result
    local overall_result="PASS"
    local reason=""

    # Check security
    local security_issues
    security_issues=$(echo "$security_result" | jq -r '.security_issues // 0')
    if [[ "$security_issues" -gt 0 ]]; then
        overall_result="FAIL"
        reason="Security issues detected"
    fi

    # Check browser compatibility
    local browser_issues
    browser_issues=$(echo "$browser_result" | jq -r '.browser_issues // 0')
    if [[ "$browser_issues" -gt 0 ]]; then
        overall_result="FAIL"
        reason="Browser compatibility issues detected"
    fi

    # Check compilation
    local compile_status
    compile_status=$(echo "$compile_result" | jq -r '.compile_result // "failed"')
    if [[ "$compile_status" != "success" ]]; then
        overall_result="FAIL"
        reason="Compilation failed"
    fi

    # Generate final report
    cat > "$output_file" << EOF
{
  "result": "$overall_result",
  "reason": "$reason",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "agent": "bacon-auditor",
  "audits": {
    "security": $security_result,
    "quality": $quality_result,
    "performance": $performance_result,
    "browser": $browser_result,
    "compile": $compile_result
  },
  "summary": {
    "total_issues": $((security_issues + browser_issues)),
    "critical_issues": $([ "$overall_result" == "FAIL" ] && echo "1" || echo "0"),
    "recommendation": $([ "$overall_result" == "PASS" ] && echo "\"APPROVED\"" || echo "\"REJECTED\"")
  }
}
EOF

    log_info "Audit report generated: $overall_result"
    if [[ "$overall_result" == "PASS" ]]; then
        log_info "Patch approved for application"
    else
        log_warn "Patch rejected: $reason"
    fi
}

# Main function
main() {
    local patch_file="$1"
    local output_file="$2"

    if [[ -z "$patch_file" || -z "$output_file" ]]; then
        log_error "Usage: $0 <patch_file> <output_file>"
        return 1
    fi

    log_info "Starting comprehensive audit"

    # Check if patch file exists
    if [[ ! -f "$patch_file" ]]; then
        log_error "Patch file not found: $patch_file"
        return 1
    fi

    # Create audit result directory
    local audit_result="${output_file}.audit"
    mkdir -p "$(dirname "$audit_result")"

    # Run all audits
    local audit_passed=0
    local total_audits=0

    # Security audit (critical)
    total_audits=$((total_audits + 1))
    if security_audit "$patch_file" "$audit_result"; then
        audit_passed=$((audit_passed + 1))
    fi

    # Browser audit (critical for Auto-Rust)
    total_audits=$((total_audits + 1))
    if browser_audit "$patch_file" "$audit_result"; then
        audit_passed=$((audit_passed + 1))
    fi

    # Compile test (critical)
    total_audits=$((total_audits + 1))
    if compile_test "$patch_file" "$audit_result"; then
        audit_passed=$((audit_passed + 1))
    fi

    # Quality audit (important but not critical)
    total_audits=$((total_audits + 1))
    if quality_audit "$patch_file" "$audit_result"; then
        audit_passed=$((audit_passed + 1))
    fi

    # Performance audit (important but not critical)
    total_audits=$((total_audits + 1))
    if performance_audit "$patch_file" "$audit_result"; then
        audit_passed=$((audit_passed + 1))
    fi

    # Generate final report
    generate_audit_report "$audit_result" "$output_file"

    log_info "Audit completed: $audit_passed/$total_audits audits passed"

    # Cleanup temporary files
    rm -f "${audit_result}.security" "${audit_result}.quality" "${audit_result}.performance" "${audit_result}.browser" "${audit_result}.compile"

    # Return success if critical audits passed
    if [[ "$audit_passed" -ge 3 ]]; then  # Security, browser, and compile are critical
        return 0
    else
        return 1
    fi
}

# Execute main function
main "$@"
