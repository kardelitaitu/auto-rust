#!/bin/bash
# .bacon/scripts/bacon-sentinel.sh - Enhanced Sentinel Agent
# Monitors codebase for hotspots and generates structured JSON output

set -euo pipefail
IFS=$'\n\t'

# Configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && (pwd -W 2>/dev/null || pwd))"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && (pwd -W 2>/dev/null || pwd))"
readonly SESSIONS_DIR="${PROJECT_ROOT}/.bacon/sessions"
readonly CONFIG_FILE="${PROJECT_ROOT}/.bacon/bacon.toml"
readonly LOG_FILE="${SESSIONS_DIR}/sentinel.log"
readonly OUTPUT_FILE="${SESSIONS_DIR}/last_hotspot.json"

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

# Validate JSON output
validate_json() {
    local file="$1"

    if [[ ! -f "$file" ]]; then
        log_error "Output file not found: $file"
        return 1
    fi

    if [[ ! -s "$file" ]]; then
        log_warn "Output file is empty: $file"
        return 1
    fi

    # Try to validate JSON with jq
    if command -v jq >/dev/null 2>&1; then
        if ! jq empty "$file" 2>/dev/null; then
            log_error "Invalid JSON output in: $file"
            return 1
        fi

        # Check if it's the expected format (array of messages)
        local json_type
        json_type=$(jq -r 'if type == "array" then "array" elif type == "object" then "object" else "unknown" end' "$file" 2>/dev/null || echo "unknown")

        case "$json_type" in
            "array")
                local count
                count=$(jq 'length' "$file" 2>/dev/null || echo "0")
                log_info "Valid JSON array with $count entries"
                ;;
            "object")
                log_info "Valid JSON object"
                ;;
            *)
                log_warn "Unexpected JSON type: $json_type"
                ;;
        esac
    else
        log_warn "jq not available, skipping JSON validation"
    fi

    return 0
}

# Check prerequisites
check_prerequisites() {
    log_debug "Checking sentinel prerequisites..."

    # Check required commands
    local required_commands=("cargo" "jq")
    for cmd in "${required_commands[@]}"; do
        if ! command -v "$cmd" >/dev/null 2>&1; then
            log_error "Required command not found: $cmd"
            return 1
        fi
    done

    # Check directories
    if [[ ! -d "$SESSIONS_DIR" ]]; then
        log_info "Creating sessions directory: $SESSIONS_DIR"
        mkdir -p "$SESSIONS_DIR"
    fi

    # Check config file
    if [[ ! -f "$CONFIG_FILE" ]]; then
        log_warn "Configuration file not found: $CONFIG_FILE, using defaults"
    fi

    log_debug "Prerequisites check passed"
    return 0
}

# Run one-shot clippy scan with enhanced error handling
run_bacon_scan() {
    log_info "Starting clippy scan for hotspots"

    # Create temporary output file
    local temp_output="${OUTPUT_FILE}.tmp"
    local raw_output="${OUTPUT_FILE}.raw"
    local stderr_output="${OUTPUT_FILE}.stderr"

    local cargo_args=("clippy" "--message-format=json" "--" "-D" "warnings")
    local timeout_seconds="${BACON_SENTINEL_TIMEOUT:-300}"
    local exit_code=0

    log_debug "Running: cargo ${cargo_args[*]}"

    if command -v timeout >/dev/null 2>&1; then
        timeout "$timeout_seconds" cargo "${cargo_args[@]}" > "$raw_output" 2> "$stderr_output" || exit_code=$?
    else
        cargo "${cargo_args[@]}" > "$raw_output" 2> "$stderr_output" || exit_code=$?
    fi

    if [[ "$exit_code" -eq 124 ]]; then
        log_error "Clippy scan timed out after ${timeout_seconds} seconds"
        rm -f "$temp_output" "$raw_output" "$stderr_output"
        return 1
    fi

    # Normalize Cargo JSON lines into an array of compiler diagnostics.
    if ! jq -s '[.[] | select(.reason == "compiler-message") | .message | select(.level == "error" or .level == "warning" or .level == "note")]' \
        "$raw_output" > "$temp_output" 2>/dev/null; then
        log_error "Failed to normalize clippy JSON output"
        if [[ -s "$stderr_output" ]]; then
            tail -10 "$stderr_output" | while read -r line; do
                log_error "  $line"
            done
        fi
        rm -f "$temp_output" "$raw_output" "$stderr_output"
        return 1
    fi

    # Validate output
    if ! validate_json "$temp_output"; then
        log_error "Clippy output validation failed"
        rm -f "$temp_output" "$raw_output" "$stderr_output"
        return 1
    fi

    local entry_count
    entry_count=$(jq 'length' "$temp_output" 2>/dev/null || echo "0")
    if [[ "$entry_count" -eq 0 ]]; then
        log_info "Clippy scan completed with no hotspots"
        rm -f "$OUTPUT_FILE" "$temp_output" "$raw_output" "$stderr_output"
        return 0
    fi

    if [[ "$exit_code" -ne 0 ]]; then
        log_info "Clippy reported hotspots with exit code: $exit_code"
    fi

    # Move temp file to final location
    mv "$temp_output" "$OUTPUT_FILE"
    rm -f "$raw_output" "$stderr_output"

    # Log summary
    if command -v jq >/dev/null 2>&1 && [[ -f "$OUTPUT_FILE" ]]; then
        log_info "Clippy scan completed successfully ($entry_count entries)"
    else
        log_info "Clippy scan completed successfully"
    fi

    return 0
}

# Cleanup old hotspot files
cleanup_old_files() {
    if [[ -n "${BACON_SENTINEL_CLEANUP:-}" ]]; then
        log_debug "Cleaning up old hotspot files"

        # Remove hotspot files older than specified age
        local max_age="${BACON_SENTINEL_MAX_AGE:-24}"
        find "$SESSIONS_DIR" -name "last_hotspot.json.*" -type f -mtime +$max_age -delete 2>/dev/null || true
        find "$SESSIONS_DIR" -name "resolved_*.json" -type f -mtime +$max_age -delete 2>/dev/null || true

        log_debug "Cleanup completed"
    fi
}

# Main function
main() {
    log_info "Starting bacon sentinel"

    # Check prerequisites
    if ! check_prerequisites; then
        log_error "Prerequisites check failed"
        return 1
    fi

    # Cleanup old files
    cleanup_old_files

    # Run bacon scan
    if run_bacon_scan; then
        log_info "Sentinel scan completed successfully"
        return 0
    else
        log_error "Sentinel scan failed"
        return 1
    fi
}

# Execute main function
main "$@"
