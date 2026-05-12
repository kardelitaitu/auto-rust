#!/bin/bash
# .bacon/scripts/bacon-observer.sh - Enhanced Observer Agent
# Processes compiler output and extracts structured problem briefs

set -euo pipefail
IFS=$'\n\t'

# Configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && (pwd -W 2>/dev/null || pwd))"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && (pwd -W 2>/dev/null || pwd))"
readonly ROLES_DIR="${PROJECT_ROOT}/.bacon/roles"
readonly LOG_FILE="${PROJECT_ROOT}/.bacon/sessions/observer.log"

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

# Process clippy JSON output
process_clippy_output() {
    local input_file="$1"
    local output_file="$2"

    log_info "Processing clippy output: $(basename "$input_file")"

    # Check if input file exists
    if [[ ! -f "$input_file" ]]; then
        log_error "Input file not found: $input_file"
        return 1
    fi

    # Initialize output JSON
    local temp_output="${output_file}.tmp"
    echo '{"problems": [], "summary": {"total": 0, "by_level": {"error": 0, "warning": 0, "note": 0}, "by_category": {}}}' > "$temp_output"

    # Process each clippy message
    local total=0
    local errors=0
    local warnings=0
    local notes=0

    if command -v jq >/dev/null 2>&1; then
        # Use jq for proper JSON processing. Input may be a JSON array or Cargo JSON lines.
        while IFS= read -r line; do
            [[ -z "$line" ]] && continue

            local problem_entry
            problem_entry=$(printf '%s\n' "$line" | jq -c '
                (if (.message? | type) == "object" then .message else . end) as $msg |
                select(($msg.message // null) != null and ($msg.level // null) != null) |
                {
                    message: $msg.message,
                    level: $msg.level,
                    code: ($msg.code.code // $msg.code // "unknown"),
                    spans: (($msg.spans // []) | if length > 0 then [{
                        file_name: (.[0].file_name // "unknown"),
                        line_start: (.[0].line_start // 0),
                        line_end: (.[0].line_end // .[0].line_start // 0),
                        column_start: (.[0].column_start // 0),
                        column_end: (.[0].column_end // 0),
                        text: (.[0].text // [])
                    }] else [] end),
                    rendered: ($msg.rendered // "")
                }' 2>/dev/null || true)

            if [[ -z "$problem_entry" || "$problem_entry" == "null" ]]; then
                continue
            fi

            # Add to problems array
            jq --argjson problem "$problem_entry" '.problems += [$problem]' "$temp_output" > "${temp_output}.new" && mv "${temp_output}.new" "$temp_output"

            # Count by level
            local level
            level=$(printf '%s\n' "$problem_entry" | jq -r '.level' 2>/dev/null || echo "unknown")
            case "$level" in
                "error") errors=$((errors + 1)) ;;
                "warning") warnings=$((warnings + 1)) ;;
                "note") notes=$((notes + 1)) ;;
            esac
            total=$((total + 1))
        done < <(
            if jq -e 'type == "array"' "$input_file" >/dev/null 2>&1; then
                jq -c '.[]' "$input_file"
            else
                grep -v '^[[:space:]]*$' "$input_file"
            fi
        )

        # Update summary
        jq --arg total "$total" --arg errors "$errors" --arg warnings "$warnings" --arg notes "$notes" \
            '.summary.total = ($total | tonumber) |
             .summary.by_level.error = ($errors | tonumber) |
             .summary.by_level.warning = ($warnings | tonumber) |
             .summary.by_level.note = ($notes | tonumber)' \
            "$temp_output" > "${temp_output}.new" && mv "${temp_output}.new" "$temp_output"
    else
        # Fallback without jq - simple text processing
        log_warn "jq not available, using simple text processing"

        # Create simple JSON structure manually
        local problems_json="[]"
        local first=true

        while IFS= read -r line; do
            if [[ "$line" =~ ^\{.*\}$ ]]; then
                # Extract basic info with grep/sed
                local message level code file line_num

                message=$(echo "$line" | grep -o '"message":"[^"]*"' | sed 's/"message":"\([^"]*\)"/\1/' || echo "unknown")
                level=$(echo "$line" | grep -o '"level":"[^"]*"' | sed 's/"level":"\([^"]*\)"/\1/' || echo "unknown")
                code=$(echo "$line" | grep -o '"code":{"code":"[^"]*"' | sed 's/"code":{"code":"\([^"]*\)"/\1/' || echo "unknown")

                # Add to problems (simplified)
                if [[ "$first" == "true" ]]; then
                    problems_json='[{"message":"'"$message"'","level":"'"$level"'","code":"'"$code"'"}]'
                    first=false
                else
                    problems_json="${problems_json%,},{\"message\":\"$message\",\"level\":\"$level\",\"code\":\"$code\"}]"
                fi

                case "$level" in
                    "error") errors=$((errors + 1)) ;;
                    "warning") warnings=$((warnings + 1)) ;;
                    "note") notes=$((notes + 1)) ;;
                esac
                total=$((total + 1))
            fi
        done < "$input_file"

        # Create final JSON
        cat > "$temp_output" << EOF
{
  "problems": $problems_json,
  "summary": {
    "total": $total,
    "by_level": {
      "error": $errors,
      "warning": $warnings,
      "note": $notes
    },
    "by_category": {}
  }
}
EOF
    fi

    # Move temp file to final location
    mv "$temp_output" "$output_file"

    log_info "Processed $total problems (errors: $errors, warnings: $warnings, notes: $notes)"
}

# Extract code context for problems
extract_code_context() {
    local output_file="$1"

    log_debug "Extracting code context for problems"

    if ! command -v jq >/dev/null 2>&1; then
        log_warn "jq not available, skipping code context extraction"
        return 0
    fi

    # Read problems and extract context
    local temp_context="${output_file}.context.tmp"
    jq -r '.problems[] | "\(.spans[0].file_name // ""):\(.spans[0].line_start // 0):\(.spans[0].line_end // 0)"' "$output_file" | while IFS=: read -r file start end; do
        local target_file="$file"
        if [[ ! -f "$target_file" && -f "${PROJECT_ROOT}/${file}" ]]; then
            target_file="${PROJECT_ROOT}/${file}"
        fi

        if [[ -f "$target_file" && -n "$start" && -n "$end" && "$start" -gt 0 ]]; then
            # Extract lines around the problem
            local context_start=$((start - 2))
            local context_end=$((end + 2))

            if [[ "$context_start" -lt 1 ]]; then
                context_start=1
            fi

            echo "--- Context for $file:$start-$end ---"
            sed -n "${context_start},${context_end}p" "$target_file" 2>/dev/null || true
            echo
        fi
    done > "$temp_context"

    # Add context to the output
    if [[ -f "$temp_context" && -s "$temp_context" ]]; then
        jq --rawfile context "$temp_context" '. + {context: $context}' "$output_file" > "${output_file}.new" && mv "${output_file}.new" "$output_file"
    fi

    rm -f "$temp_context"
}

# Main function
main() {
    local input_file="$1"
    local output_file="$2"

    if [[ -z "$input_file" || -z "$output_file" ]]; then
        log_error "Usage: $0 <input_file> <output_file>"
        return 1
    fi

    log_info "Starting observer analysis"

    # Process the input
    if process_clippy_output "$input_file" "$output_file"; then
        # Extract code context
        extract_code_context "$output_file"

        # Validate output against schema
        if [[ -f "${SCRIPT_DIR}/json-validator.sh" ]]; then
            if "${SCRIPT_DIR}/json-validator.sh" validate "$output_file" "observer-output"; then
                log_debug "Output validation passed"
            else
                log_warn "Output validation failed, but continuing"
            fi
        fi

        log_info "Observer analysis completed successfully"
        return 0
    else
        log_error "Observer analysis failed"
        return 1
    fi
}

# Execute main function
main "$@"
