#!/bin/bash
# .bacon/scripts/bacon-orchestrate.sh - Enhanced Main Agentic Loop
# Improved with error handling, logging, and safety mechanisms

set -euo pipefail  # Strict error handling
IFS=$'\n\t'      # Safer IFS

# Configuration and Environment
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && (pwd -W 2>/dev/null || pwd))"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && (pwd -W 2>/dev/null || pwd))"
readonly ROLES_DIR="${PROJECT_ROOT}/.bacon/roles"
readonly SESSIONS_DIR="${PROJECT_ROOT}/.bacon/sessions"
readonly CONFIG_FILE="${PROJECT_ROOT}/.bacon/bacon.toml"
readonly LOG_FILE="${SESSIONS_DIR}/bacon_orchestrate.log"
readonly METRICS_FILE="${SESSIONS_DIR}/metrics.json"
readonly PID_FILE="${SESSIONS_DIR}/orchestrate.pid"

# Load configuration with environment variable support
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

# Health check utilities
check_prerequisites() {
    log_info "Checking prerequisites..."

    # Check required commands
    local required_commands=("cargo" "git" "jq")
    for cmd in "${required_commands[@]}"; do
        if ! command -v "$cmd" >/dev/null 2>&1; then
            log_error "Required command not found: $cmd"
            return 1
        fi
    done

    # Check directories
    for dir in "$ROLES_DIR" "$SESSIONS_DIR"; do
        if [[ ! -d "$dir" ]]; then
            log_error "Required directory not found: $dir"
            return 1
        fi
    done

    # Check config file
    if [[ ! -f "$CONFIG_FILE" ]]; then
        log_error "Configuration file not found: $CONFIG_FILE"
        return 1
    fi

    log_info "Prerequisites check passed"
}

# Cleanup function
cleanup() {
    local exit_code=$?
    log_info "Cleaning up... (exit code: $exit_code)"

    # Remove shadow workspaces
    if [[ -n "${SHADOW_CLEANUP:-}" ]]; then
        find /tmp -name "norino_shadow_*" -type d -mtime +1 -exec rm -rf {} + 2>/dev/null || true
    fi

    # Remove PID file
    rm -f "$PID_FILE"

    log_info "Cleanup completed"
}

# Enhanced signal handling
shutdown_requested=false
graceful_shutdown() {
    log_info "Shutdown signal received, initiating graceful shutdown..."
    shutdown_requested=true
}

# Signal handlers
trap graceful_shutdown INT TERM
trap cleanup EXIT

# Metrics collection
update_metrics() {
    local event_type="$1"
    local status="$2"
    local timestamp="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

    local metric_entry="{\"timestamp\":\"$timestamp\",\"event\":\"$event_type\",\"status\":\"$status\"}"

    # Create metrics file if it doesn't exist
    if [[ ! -f "$METRICS_FILE" ]]; then
        echo "[]" > "$METRICS_FILE"
    fi

    # Append new metric (simple JSON array append)
    if command -v jq >/dev/null 2>&1; then
        jq --argjson new "$metric_entry" '. += [$new]' "$METRICS_FILE" > "${METRICS_FILE}.tmp" && mv "${METRICS_FILE}.tmp" "$METRICS_FILE"
    else
        # Fallback without jq
        echo "$metric_entry" >> "$METRICS_FILE.raw"
    fi
}

# Process hotspot with enhanced error handling
process_hotspot() {
    local hotspot_file="$SESSIONS_DIR/last_hotspot.json"

    if [[ ! -f "$hotspot_file" ]]; then
        log_debug "No hotspot file found"
        return 0
    fi

    log_info "Processing hotspot: $(basename "$hotspot_file")"

    # Validate hotspot file
    if [[ ! -s "$hotspot_file" ]]; then
        log_warn "Hotspot file is empty, removing"
        rm -f "$hotspot_file"
        return 0
    fi

    # Create working directory for this cycle
    local work_dir="${SESSIONS_DIR}/cycle_$(date +%s)"
    mkdir -p "$work_dir"

    # 1. Observer phase
    log_info "Phase 1: Observer analysis"
    if ! "${SCRIPT_DIR}/bacon-observer.sh" "$hotspot_file" "$work_dir/observer_output.json"; then
        log_error "Observer phase failed"
        update_metrics "observer" "failed"
        return 1
    fi
    update_metrics "observer" "success"

    # 2. Strategy phase
    log_info "Phase 2: Strategy planning"
    if ! "${SCRIPT_DIR}/bacon-strategist.sh" "$work_dir/observer_output.json" "$work_dir/strategy_plan.json"; then
        log_error "Strategy phase failed"
        update_metrics "strategist" "failed"
        return 1
    fi
    update_metrics "strategist" "success"

    # 3. Coder phase
    log_info "Phase 3: Code generation"
    if ! "${SCRIPT_DIR}/bacon-coder.sh" "$work_dir/strategy_plan.json" "$work_dir/patch.diff"; then
        log_error "Coder phase failed"
        update_metrics "coder" "failed"
        return 1
    fi
    update_metrics "coder" "success"

    # 4. Auditor phase
    log_info "Phase 4: Code audit"
    if ! "${SCRIPT_DIR}/bacon-auditor.sh" "$work_dir/patch.diff" "$work_dir/audit_result.json"; then
        log_error "Auditor phase failed"
        update_metrics "auditor" "failed"
        return 1
    fi
    update_metrics "auditor" "success"

    # 5. Apply phase (if audit passed)
    local audit_result
    audit_result=$(jq -r '.result // "FAIL"' "$work_dir/audit_result.json" 2>/dev/null || echo "FAIL")

    if [[ "$audit_result" == "PASS" ]]; then
        log_info "Audit passed, verifying patch in shadow workspace"
        if "${SCRIPT_DIR}/bacon-apply-shadow.sh" "$work_dir/patch.diff"; then
            log_info "Patch verified and queued for manual application"
            update_metrics "apply" "approved"
            mv "$hotspot_file" "${SESSIONS_DIR}/approved_$(date +%s).json"
        else
            log_error "Failed to apply patch"
            update_metrics "apply" "failed"
        fi
    else
        log_warn "Audit failed: $(jq -r '.reason // "Unknown"' "$work_dir/audit_result.json" 2>/dev/null || echo "Unknown")"
        update_metrics "apply" "rejected"
    fi

    # Cleanup work directory
    rm -rf "$work_dir"
}

# Main orchestration loop
main_loop() {
    local cycle_interval="${BACON_CYCLE_INTERVAL:-10}"
    local max_cycles="${BACON_MAX_CYCLES:-0}"  # 0 = infinite
    local cycle_count=0

    log_info "Starting bacon orchestration loop (interval: ${cycle_interval}s)"

    while [[ "$shutdown_requested" == "false" ]]; do
        cycle_count=$((cycle_count + 1))
        log_info "=== Cycle $cycle_count ==="

        # Check max cycles
        if [[ "$max_cycles" -gt 0 && "$cycle_count" -gt "$max_cycles" ]]; then
            log_info "Reached maximum cycles ($max_cycles), exiting"
            break
        fi

        # Check for shutdown request before processing
        if [[ "$shutdown_requested" == "true" ]]; then
            log_info "Shutdown requested, stopping after cycle $cycle_count"
            break
        fi

        # Process any hotspots
        if process_hotspot; then
            log_info "Cycle $cycle_count completed successfully"
        else
            log_error "Cycle $cycle_count encountered errors"
        fi

        # Check for shutdown request before sleeping
        if [[ "$shutdown_requested" == "true" ]]; then
            log_info "Shutdown requested, not sleeping after cycle $cycle_count"
            break
        fi

        # Sleep until next cycle with interrupt handling
        log_debug "Sleeping for $cycle_interval seconds..."
        local sleep_remaining=$cycle_interval
        while [[ $sleep_remaining -gt 0 && "$shutdown_requested" == "false" ]]; do
            sleep 1
            ((sleep_remaining--))
        done
    done

    if [[ "$shutdown_requested" == "true" ]]; then
        log_info "Graceful shutdown completed after $cycle_count cycles"
    else
        log_info "Orchestration loop completed after $cycle_count cycles"
    fi
}

# Check if already running
if [[ -f "$PID_FILE" ]] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    log_error "Bacon orchestration is already running (PID: $(cat "$PID_FILE"))"
    exit 1
fi

# Write PID file
echo "$$" > "$PID_FILE"

# Ensure sessions directory exists
mkdir -p "$SESSIONS_DIR"

# Main execution
if check_prerequisites; then
    main_loop
else
    log_error "Prerequisites check failed, exiting"
    exit 1
fi
