#!/bin/bash
# .bacon/scripts/bacon-coder.sh - Enhanced Code Generation Agent
# Generates minimal patches based on strategy specifications

set -euo pipefail
IFS=$'\n\t'

# Configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && (pwd -W 2>/dev/null || pwd))"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && (pwd -W 2>/dev/null || pwd))"
readonly ROLES_DIR="${PROJECT_ROOT}/.bacon/roles"
readonly LOG_FILE="${PROJECT_ROOT}/.bacon/sessions/coder.log"

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

# Validate strategy file
validate_strategy_file() {
    local strategy_file="$1"

    if [[ ! -f "$strategy_file" ]]; then
        log_error "Strategy file not found: $strategy_file"
        return 1
    fi

    if ! command -v jq >/dev/null 2>&1; then
        log_error "jq is required for strategy validation"
        return 1
    fi

    # Check if strategy file has required structure
    if ! jq -e '.strategies and .implementation_plan.high_priority and .implementation_plan.medium_priority' "$strategy_file" >/dev/null 2>&1; then
        log_error "Invalid strategy file structure"
        return 1
    fi

    log_info "Strategy file validation passed"
    return 0
}

# Generate code for specific problem type
generate_code_fix() {
    local problem_data="$1"
    local file_path="$2"

    local message level code file line_start
    message=$(echo "$problem_data" | jq -r '.problem.message // "unknown"')
    level=$(echo "$problem_data" | jq -r '.problem.level // "unknown"')
    code=$(echo "$problem_data" | jq -r '.problem.code // "unknown"')
    file=$(echo "$problem_data" | jq -r '.problem.location.file // "unknown"')
    line_start=$(echo "$problem_data" | jq -r '.problem.location.line // 0')

    local approach action
    approach=$(echo "$problem_data" | jq -r '.strategy.approach // "unknown"')
    action=$(echo "$problem_data" | jq -r '.strategy.recommended_action // "unknown"')

    log_debug "Generating fix for $code in $file:$line_start"

    # Check if target file exists
    if [[ ! -f "$file_path/$file" ]]; then
        log_warn "Target file not found: $file"
        return 1
    fi

    # Generate fix based on problem type
    case "$code" in
        "dead_code"|"unused_variables"|"unused_imports")
            generate_unused_code_fix "$file_path/$file" "$line_start" "$code"
            ;;
        "clippy::needless_return"|"clippy::redundant_clone"|"clippy::unnecessary_operation")
            generate_clippy_fix "$file_path/$file" "$line_start" "$code"
            ;;
        "E0381") # borrow checker
            generate_borrow_fix "$file_path/$file" "$line_start" "$message"
            ;;
        "E0277") # type mismatch
            generate_type_fix "$file_path/$file" "$line_start" "$message"
            ;;
        *)
            generate_generic_fix "$file_path/$file" "$line_start" "$message" "$action"
            ;;
    esac
}

# Generate fix for unused code
generate_unused_code_fix() {
    local file="$1"
    local line_start="$2"
    local code="$3"

    # Remove or comment out unused code
    local temp_file="${file}.fix.tmp"

    if [[ "$code" == "unused_imports" ]]; then
        # Remove unused import
        sed "${line_start}s/^.*$/\/\/ \/\/ Removed unused import/" "$file" > "$temp_file"
    elif [[ "$code" == "unused_variables" ]]; then
        # Prefix with underscore
        sed "${line_start}s/\\blet \\([a-zA-Z][a-zA-Z0-9_]*\\)/let _\\1/" "$file" > "$temp_file"
    else
        # Comment out unused code
        sed "${line_start}s/^/\/\/ /" "$file" > "$temp_file"
    fi

    if [[ -f "$temp_file" && -s "$temp_file" ]]; then
        mv "$temp_file" "$file"
        log_debug "Applied unused code fix at line $line_start"
    else
        rm -f "$temp_file"
        log_warn "Failed to apply unused code fix"
    fi
}

# Generate fix for clippy suggestions
generate_clippy_fix() {
    local file="$1"
    local line_start="$2"
    local code="$3"

    local temp_file="${file}.fix.tmp"

    case "$code" in
        "clippy::needless_return")
            # Remove unnecessary return
            sed "${line_start}s/return \\(.*\\);/\\1;/" "$file" > "$temp_file"
            ;;
        "clippy::redundant_clone")
            # Remove .clone() call
            sed "${line_start}s/\\.clone()//g" "$file" > "$temp_file"
            ;;
        "clippy::unnecessary_operation")
            # Remove unnecessary operation (simplified)
            sed "${line_start}s/.*$/\/\/ \/\/ Removed unnecessary operation/" "$file" > "$temp_file"
            ;;
    esac

    if [[ -f "$temp_file" && -s "$temp_file" ]]; then
        mv "$temp_file" "$file"
        log_debug "Applied clippy fix for $code at line $line_start"
    else
        rm -f "$temp_file"
        log_warn "Failed to apply clippy fix for $code"
    fi
}

# Generate fix for borrow checker issues
generate_borrow_fix() {
    local file="$1"
    local line_start="$2"
    local message="$3"

    local temp_file="${file}.fix.tmp"

    # Common borrow checker fixes
    if [[ "$message" =~ "borrow of moved value" ]]; then
        # Add .clone() for moved value
        sed "${line_start}s/\\([a-zA-Z][a-zA-Z0-9_]*\\)/\\1.clone()/g" "$file" > "$temp_file"
    elif [[ "$message" =~ "cannot borrow" ]]; then
        # Add reference operator
        sed "${line_start}s/\\([a-zA-Z][a-zA-Z0-9_]*\\)/\\&\\1/g" "$file" > "$temp_file"
    else
        # Generic fix - add comments for manual review
        sed "${line_start}s/^.*$/\/\/ \/\/ TODO: Fix borrow checker: $message/" "$file" > "$temp_file"
    fi

    if [[ -f "$temp_file" && -s "$temp_file" ]]; then
        mv "$temp_file" "$file"
        log_debug "Applied borrow checker fix at line $line_start"
    else
        rm -f "$temp_file"
        log_warn "Failed to apply borrow checker fix"
    fi
}

# Generate fix for type issues
generate_type_fix() {
    local file="$1"
    local line_start="$2"
    local message="$3"

    local temp_file="${file}.fix.tmp"

    # Add type annotation or conversion
    if [[ "$message" =~ "expected" && "$message" =~ "found" ]]; then
        # Try to add type annotation
        sed "${line_start}s/let \\([a-zA-Z][a-zA-Z0-9_]*\\)/let \\1: \/\/ TODO: Add type annotation/" "$file" > "$temp_file"
    else
        # Generic fix
        sed "${line_start}s/^.*$/\/\/ \/\/ TODO: Fix type error: $message/" "$file" > "$temp_file"
    fi

    if [[ -f "$temp_file" && -s "$temp_file" ]]; then
        mv "$temp_file" "$file"
        log_debug "Applied type fix at line $line_start"
    else
        rm -f "$temp_file"
        log_warn "Failed to apply type fix"
    fi
}

# Generate generic fix
generate_generic_fix() {
    local file="$1"
    local line_start="$2"
    local message="$3"
    local action="$4"

    local temp_file="${file}.fix.tmp"

    # Add comment with suggested action
    sed "${line_start}s/^.*$/\/\/ \/\/ TODO: $action - $message/" "$file" > "$temp_file"

    if [[ -f "$temp_file" && -s "$temp_file" ]]; then
        mv "$temp_file" "$file"
        log_debug "Applied generic fix at line $line_start"
    else
        rm -f "$temp_file"
        log_warn "Failed to apply generic fix"
    fi
}

# Create patch file from changes
create_patch_file() {
    local project_root="$1"
    local output_patch="$2"

    log_info "Creating patch file"

    cd "$project_root"

    # Create a normal patch against the cloned repository.
    if git diff --quiet --exit-code; then
        log_warn "No code changes generated"
        : > "$output_patch"
        return 1
    fi

    if git diff --binary > "$output_patch"; then
        log_info "Patch file created: $output_patch"
        return 0
    else
        log_error "Failed to create patch file"
        return 1
    fi
}

# Main function
main() {
    local input_file="$1"
    local output_file="$2"

    if [[ -z "$input_file" || -z "$output_file" ]]; then
        log_error "Usage: $0 <input_file> <output_file>"
        return 1
    fi

    log_info "Starting code generation"

    # Validate strategy file
    if ! validate_strategy_file "$input_file"; then
        return 1
    fi

    # Create working directory as an isolated clone so patches are relative to repo root.
    local work_dir="${PROJECT_ROOT}/.bacon/sessions/coder_work_$(date +%s)_${BASHPID}"

    if ! git clone "$PROJECT_ROOT" "$work_dir" 2>/dev/null; then
        log_error "Failed to create coder workspace"
        rm -rf "$work_dir"
        return 1
    fi

    # Process high priority issues first
    local high_priority_count=0
    local total_processed=0

    if command -v jq >/dev/null 2>&1; then
        # Process high priority issues
        while IFS= read -r issue; do
            local priority
            priority=$(echo "$issue" | jq -r '.strategy.priority // "unknown"')

            if [[ "$priority" == "high" ]]; then
                high_priority_count=$((high_priority_count + 1))
                total_processed=$((total_processed + 1))

                log_info "Processing high priority issue #$high_priority_count"

                # Generate fix for this issue
                if generate_code_fix "$issue" "$work_dir"; then
                    log_info "Generated fix for high priority issue #$high_priority_count"
                else
                    log_warn "Failed to generate fix for high priority issue #$high_priority_count"
                fi
            fi
        done < <(jq -c '.implementation_plan.high_priority[]?' "$input_file")

        # Process medium priority issues if time permits
        local medium_priority_count=0
        while IFS= read -r issue; do
            if [[ "$medium_priority_count" -lt 5 ]]; then  # Limit to 5 medium issues
                medium_priority_count=$((medium_priority_count + 1))
                total_processed=$((total_processed + 1))

                log_info "Processing medium priority issue #$medium_priority_count"

                if generate_code_fix "$issue" "$work_dir"; then
                    log_info "Generated fix for medium priority issue #$medium_priority_count"
                else
                    log_warn "Failed to generate fix for medium priority issue #$medium_priority_count"
                fi
            fi
        done < <(jq -c '.implementation_plan.medium_priority[]?' "$input_file")
    else
        log_error "jq is required for code generation"
        rm -rf "$work_dir"
        return 1
    fi

    # Create patch file
    if create_patch_file "$work_dir" "$output_file"; then
        log_info "Code generation completed successfully"
        log_info "Processed $total_processed issues (high: $high_priority_count)"
    else
        log_error "Code generation failed"
        rm -rf "$work_dir"
        return 1
    fi

    # Cleanup
    rm -rf "$work_dir"

    return 0
}

# Execute main function
main "$@"
