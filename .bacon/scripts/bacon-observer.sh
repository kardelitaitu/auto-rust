#!/bin/bash
# .bacon/scripts/bacon-observer.sh - Enhanced Observer Agent
# Processes compiler output and extracts structured problem briefs

set -euo pipefail
IFS=$'\n\t'

# Configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
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
    echo '{"problems": [], "summary": {"total": 0, "errors": 0, "warnings": 0, "notes": 0}}' > "$temp_output"
    
    # Process each clippy message
    local total=0
    local errors=0
    local warnings=0
    local notes=0
    
    if command -v jq >/dev/null 2>&1; then
        # Use jq for proper JSON processing
        while IFS= read -r line; do
            if [[ -n "$line" ]]; then
                local problem_entry
                problem_entry=$(echo "$line" | jq -c '{
                    message: .message,
                    level: .level,
                    code: .code.code,
                    spans: [{
                        file_name: .spans[0].file_name,
                        line_start: .spans[0].line_start,
                        line_end: .spans[0].line_end,
                        column_start: .spans[0].column_start,
                        column_end: .spans[0].column_end,
                        text: .spans[0].text
                    }],
                    rendered: .rendered
                }' 2>/dev/null || echo "{}")
                
                # Add to problems array
                jq --argjson problem "$problem_entry" '.problems += [$problem]' "$temp_output" > "${temp_output}.new" && mv "${temp_output}.new" "$temp_output"
                
                # Count by level
                local level
                level=$(echo "$line" | jq -r '.level' 2>/dev/null || echo "unknown")
                case "$level" in
                    "error") ((errors++)) ;;
                    "warning") ((warnings++)) ;;
                    "note") ((notes++)) ;;
                esac
                ((total++))
            fi
        done < "$input_file"
        
        # Update summary
        jq --arg total "$total" --arg errors "$errors" --arg warnings "$warnings" --arg notes "$notes" \
            '.summary.total = ($total | tonumber) | 
             .summary.errors = ($errors | tonumber) | 
             .summary.warnings = ($warnings | tonumber) | 
             .summary.notes = ($notes | tonumber)' \
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
                    "error") ((errors++)) ;;
                    "warning") ((warnings++)) ;;
                    "note") ((notes++)) ;;
                esac
                ((total++))
            fi
        done < "$input_file"
        
        # Create final JSON
        cat > "$temp_output" << EOF
{
  "problems": $problems_json,
  "summary": {
    "total": $total,
    "errors": $errors,
    "warnings": $warnings,
    "notes": $notes
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
    jq -r '.problems[] | "\(.spans[0].file_name):\(.spans[0].line_start):\(.spans[0].line_end)"' "$output_file" | while IFS=: read -r file start end; do
        if [[ -f "$file" && -n "$start" && -n "$end" ]]; then
            # Extract lines around the problem
            local context_start=$((start - 2))
            local context_end=$((end + 2))
            
            if [[ "$context_start" -lt 1 ]]; then
                context_start=1
            fi
            
            echo "--- Context for $file:$start-$end ---"
            sed -n "${context_start},${context_end}p" "$file" 2>/dev/null || true
            echo
        fi
    done > "$temp_context"
    
    # Add context to the output
    if [[ -f "$temp_context" && -s "$temp_context" ]]; then
        jq --slurpfile context "$temp_context" '. + {context: $context[0]}' "$output_file" > "${output_file}.new" && mv "${output_file}.new" "$output_file"
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
