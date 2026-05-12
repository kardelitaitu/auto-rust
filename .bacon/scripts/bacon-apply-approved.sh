#!/bin/bash
# .bacon/scripts/bacon-apply-approved.sh - Apply reviewed Bacon patch candidates

set -euo pipefail
IFS=$'\n\t'

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && (pwd -W 2>/dev/null || pwd))"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && (pwd -W 2>/dev/null || pwd))"
readonly SESSIONS_DIR="${PROJECT_ROOT}/.bacon/sessions"
readonly APPROVED_DIR="${SESSIONS_DIR}/approved_patches"
readonly APPLIED_DIR="${APPROVED_DIR}/applied"
readonly LOG_FILE="${SESSIONS_DIR}/bacon_apply_approved.log"

log() {
    local level="$1"
    shift
    mkdir -p "$SESSIONS_DIR"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [$level] $*" | tee -a "$LOG_FILE"
}

log_error() { log "ERROR" "$@"; }
log_warn() { log "WARN" "$@"; }
log_info() { log "INFO" "$@"; }

usage() {
    cat << EOF
Usage: $0 [patch-file|--latest] [--dry-run] [--run-check] [--force]

Applies a verified patch from .bacon/sessions/approved_patches.

Options:
  --latest      Apply the newest approved patch. Default when no patch is given.
  --dry-run     Validate the patch without changing files.
  --run-check   Run .\\check.ps1 after applying; rollback if it fails.
  --force       Allow applying on a dirty worktree.
EOF
}

latest_patch() {
    if [[ ! -d "$APPROVED_DIR" ]]; then
        return 1
    fi
    find "$APPROVED_DIR" -maxdepth 1 -type f -name '*.diff' -print0 2>/dev/null |
        while IFS= read -r -d '' file; do
            printf '%s\t%s\n' "$(stat -c '%Y' "$file")" "$file"
        done |
        sort -nr |
        head -n 1 |
        cut -f2-
}

is_clean_worktree() {
    git -C "$PROJECT_ROOT" diff --quiet --exit-code &&
        git -C "$PROJECT_ROOT" diff --cached --quiet --exit-code
}

run_full_check() {
    local check_script="${PROJECT_ROOT}/check.ps1"
    if [[ ! -f "$check_script" ]]; then
        log_error "Full check required but missing: $check_script"
        return 1
    fi

    log_info "Running full check: .\\check.ps1"
    if command -v pwsh >/dev/null 2>&1; then
        (cd "$PROJECT_ROOT" && pwsh -NoProfile -ExecutionPolicy Bypass -File "$check_script")
    elif command -v powershell >/dev/null 2>&1; then
        (cd "$PROJECT_ROOT" && powershell -NoProfile -ExecutionPolicy Bypass -File "$check_script")
    else
        log_error "PowerShell is required to run .\\check.ps1"
        return 1
    fi
}

main() {
    local patch_file=""
    local dry_run="false"
    local run_check="false"
    local force="false"

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --latest)
                patch_file=""
                shift
                ;;
            --dry-run)
                dry_run="true"
                shift
                ;;
            --run-check)
                run_check="true"
                shift
                ;;
            --force)
                force="true"
                shift
                ;;
            -h|--help)
                usage
                return 0
                ;;
            -*)
                log_error "Unknown option: $1"
                usage
                return 2
                ;;
            *)
                patch_file="$1"
                shift
                ;;
        esac
    done

    if [[ -z "$patch_file" ]]; then
        patch_file="$(latest_patch || true)"
    fi

    if [[ -z "$patch_file" || ! -f "$patch_file" ]]; then
        log_error "No approved patch found"
        return 1
    fi

    mkdir -p "$APPLIED_DIR"

    if [[ "$force" != "true" ]] && ! is_clean_worktree; then
        log_error "Working tree is dirty; commit/stash first or pass --force"
        return 1
    fi

    log_info "Validating approved patch: $patch_file"
    if ! git -C "$PROJECT_ROOT" apply --check "$patch_file"; then
        log_error "Patch does not apply cleanly"
        return 1
    fi

    if [[ "$dry_run" == "true" ]]; then
        log_info "Dry run passed"
        return 0
    fi

    git -C "$PROJECT_ROOT" apply "$patch_file"
    log_info "Patch applied"

    if [[ "$run_check" == "true" ]]; then
        if ! run_full_check; then
            log_error "Full check failed; rolling back patch"
            git -C "$PROJECT_ROOT" apply -R "$patch_file" || log_warn "Rollback failed; inspect worktree manually"
            return 1
        fi
        log_info "Full check passed"
    fi

    if [[ "$patch_file" == "$APPROVED_DIR"/* ]]; then
        mv "$patch_file" "${APPLIED_DIR}/$(basename "$patch_file")"
        log_info "Patch moved to applied queue"
    fi
}

main "$@"
