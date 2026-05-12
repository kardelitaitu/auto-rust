#!/bin/bash
# .bacon/scripts/bacon-apply-shadow.sh - Enhanced Shadow Workspace Manager
# Improved with safety checks, rollback, and better error handling

set -euo pipefail
IFS=$'\n\t'

# Configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
readonly SESSIONS_DIR="${PROJECT_ROOT}/.bacon/sessions"
readonly LOG_FILE="${SESSIONS_DIR}/bacon_apply.log"
readonly ROLLBACK_DIR="${SESSIONS_DIR}/rollbacks"

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

# Safety checks
validate_patch() {
    local patch_file="$1"
    
    # Check if patch file exists and is readable
    if [[ ! -f "$patch_file" ]]; then
        log_error "Patch file not found: $patch_file"
        return 1
    fi
    
    # Check if patch file is empty
    if [[ ! -s "$patch_file" ]]; then
        log_warn "Patch file is empty, nothing to apply"
        return 1
    fi
    
    # Basic patch validation
    if ! grep -q "^diff" "$patch_file" 2>/dev/null; then
        log_warn "Patch file doesn't contain diff format, treating as raw content"
    fi
    
    # Check for dangerous patterns
    local dangerous_patterns=("rm -rf" "sudo" "chmod 777" "chown")
    for pattern in "${dangerous_patterns[@]}"; do
        if grep -q "$pattern" "$patch_file" 2>/dev/null; then
            log_error "Dangerous pattern detected in patch: $pattern"
            return 1
        fi
    done
    
    log_info "Patch validation passed"
    return 0
}

# Create shadow workspace with safety checks
create_shadow_workspace() {
    local shadow_id="$(date +%s)_${BASHPID}"
    local shadow_base="${TMPDIR:-/tmp}"
    local shadow_dir="${shadow_base}/norino_shadow_${shadow_id}"
    
    log_info "Creating shadow workspace: $shadow_dir"
    
    # Clone repository to shadow workspace
    if ! git clone "$PROJECT_ROOT" "$shadow_dir" 2>/dev/null; then
        log_error "Failed to create shadow workspace"
        return 1
    fi
    
    # Verify clone integrity
    if [[ ! -d "$shadow_dir/.git" ]]; then
        log_error "Shadow workspace git repository corrupted"
        rm -rf "$shadow_dir"
        return 1
    fi
    
    echo "$shadow_dir"
}

# Apply patch with error handling
apply_patch_to_shadow() {
    local shadow_dir="$1"
    local patch_file="$2"
    
    log_info "Applying patch to shadow workspace"
    
    cd "$shadow_dir"
    
    # Store current state for rollback
    local commit_hash
    commit_hash=$(git rev-parse HEAD 2>/dev/null || echo "unknown")
    
    # Create a branch for the changes
    local branch_name="bacon_patch_$(date +%s)"
    if ! git checkout -b "$branch_name" 2>/dev/null; then
        log_warn "Failed to create branch, applying to current branch"
    fi
    
    # Apply the patch
    if git apply --check "$patch_file" 2>/dev/null; then
        if git apply "$patch_file" 2>/dev/null; then
            log_info "Patch applied successfully"
            
            # Check if there are any changes
            if git diff --quiet; then
                log_warn "No changes detected after patch application"
                return 1
            fi
            
            # Commit the changes
            if git add . 2>/dev/null && git commit -m "Bacon autonomous patch: $(date)" 2>/dev/null; then
                log_info "Changes committed in shadow workspace"
                return 0
            else
                log_error "Failed to commit changes"
                return 1
            fi
        else
            log_error "Failed to apply patch"
            return 1
        fi
    else
        log_error "Patch does not apply cleanly"
        return 1
    fi
}

# Run verification in shadow workspace
verify_shadow_workspace() {
    local shadow_dir="$1"
    
    log_info "Running verification in shadow workspace"
    
    cd "$shadow_dir"
    
    # Run basic checks
    local checks_passed=0
    local total_checks=0
    
    # 1. Cargo check
    total_checks=$((total_checks + 1))
    if cargo check 2>/dev/null; then
        checks_passed=$((checks_passed + 1))
        log_info "Cargo check passed"
    else
        log_error "Cargo check failed"
    fi
    
    # 2. Cargo clippy (if available)
    if command -v cargo-clippy >/dev/null 2>&1; then
        total_checks=$((total_checks + 1))
        if cargo clippy --all-targets --all-features -- -D warnings 2>/dev/null; then
            checks_passed=$((checks_passed + 1))
            log_info "Cargo clippy passed"
        else
            log_error "Cargo clippy failed"
        fi
    fi
    
    # 3. Cargo test (quick)
    total_checks=$((total_checks + 1))
    if cargo test --lib --bins 2>/dev/null; then
        checks_passed=$((checks_passed + 1))
        log_info "Cargo test (lib/bins) passed"
    else
        log_error "Cargo test (lib/bins) failed"
    fi
    
    # 4. Build check
    total_checks=$((total_checks + 1))
    if cargo build --release 2>/dev/null; then
        checks_passed=$((checks_passed + 1))
        log_info "Cargo build passed"
    else
        log_error "Cargo build failed"
    fi
    
    log_info "Verification results: $checks_passed/$total_checks checks passed"
    
    if [[ "$checks_passed" -eq "$total_checks" ]]; then
        log_info "All verification checks passed"
        return 0
    else
        log_error "Some verification checks failed"
        return 1
    fi
}

# Create rollback point
create_rollback_point() {
    local shadow_dir="$1"
    local patch_file="$2"
    local rollback_id="$(date +%s)"
    
    mkdir -p "$ROLLBACK_DIR"
    
    # Store patch for rollback
    cp "$patch_file" "${ROLLBACK_DIR}/patch_${rollback_id}.diff"
    
    # Store git diff for rollback
    cd "$shadow_dir"
    git diff HEAD~1 > "${ROLLBACK_DIR}/rollback_${rollback_id}.diff" 2>/dev/null || true
    
    # Store metadata
    cat > "${ROLLBACK_DIR}/metadata_${rollback_id}.json" << EOF
{
  "rollback_id": "$rollback_id",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "patch_file": "patch_${rollback_id}.diff",
  "rollback_file": "rollback_${rollback_id}.diff",
  "shadow_dir": "$shadow_dir"
}
EOF
    
    log_info "Rollback point created: $rollback_id"
    echo "$rollback_id"
}

# Queue verified changes for manual application
queue_verified_patch() {
    local shadow_dir="$1"
    local rollback_id="$2"
    
    log_info "Queueing verified patch for manual application"
    
    local approved_dir="${SESSIONS_DIR}/approved_patches"
    local approved_patch="${approved_dir}/verified_${rollback_id}.diff"
    mkdir -p "$approved_dir"
    
    cd "$shadow_dir"
    if git show --format= --binary HEAD > "$approved_patch" 2>/dev/null; then
        log_info "Verified patch queued: $approved_patch"
        return 0
    else
        log_error "Failed to queue verified patch"
        return 1
    fi
}

# Cleanup function
cleanup() {
    local shadow_dir="$1"
    
    if [[ -n "$shadow_dir" && -d "$shadow_dir" ]]; then
        log_info "Cleaning up shadow workspace: $shadow_dir"
        rm -rf "$shadow_dir"
    fi
}

# Main function
main() {
    local patch_file="$1"
    
    if [[ -z "$patch_file" ]]; then
        log_error "Usage: $0 <patch_file>"
        return 1
    fi
    
    log_info "Starting patch application process"
    
    # Validate patch
    if ! validate_patch "$patch_file"; then
        return 1
    fi
    
    # Create shadow workspace
    local shadow_dir
    if ! shadow_dir=$(create_shadow_workspace); then
        return 1
    fi
    
    # Ensure cleanup
    trap "cleanup '$shadow_dir'" EXIT INT TERM
    
    # Apply patch to shadow
    if ! apply_patch_to_shadow "$shadow_dir" "$patch_file"; then
        return 1
    fi
    
    # Verify shadow workspace
    if ! verify_shadow_workspace "$shadow_dir"; then
        log_error "Shadow workspace verification failed, aborting"
        return 1
    fi
    
    # Create rollback point
    local rollback_id
    rollback_id=$(create_rollback_point "$shadow_dir" "$patch_file")
    
    # Queue patch instead of mutating the main repository automatically
    if queue_verified_patch "$shadow_dir" "$rollback_id"; then
        log_info "SUCCESS: Patch verified and queued"
        return 0
    else
        log_error "FAILED: Could not apply patch to main repository"
        return 1
    fi
}

# Execute main function
main "$@"
