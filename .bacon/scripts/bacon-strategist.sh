#!/bin/bash
# .bacon/scripts/bacon-strategist.sh - Enhanced Strategy Agent
# Analyzes problems and generates technical specifications

set -euo pipefail
IFS=$'\n\t'

# Configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && (pwd -W 2>/dev/null || pwd))"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && (pwd -W 2>/dev/null || pwd))"
readonly ROLES_DIR="${PROJECT_ROOT}/.bacon/roles"
readonly LOG_FILE="${PROJECT_ROOT}/.bacon/sessions/strategist.log"

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

# Analyze problem severity and priority
analyze_problem_priority() {
    local level="$1"
    local code="$2"

    case "$level" in
        "error")
            echo "high"
            ;;
        "warning")
            case "$code" in
                "dead_code"|"unused_imports"|"unused_variables") echo "low" ;;
                "clippy::all"|"clippy::pedantic") echo "medium" ;;
                *) echo "medium" ;;
            esac
            ;;
        "note")
            echo "low"
            ;;
        *)
            echo "medium"
            ;;
    esac
}

# Categorize problems by type
categorize_problem() {
    local code="$1"
    local message="$2"

    # Concurrency issues
    if [[ "$message" =~ (mutex|lock|atomic|sync|thread|concurrent|deadlock|race) ]]; then
        echo "concurrency"
        return
    fi

    # Memory issues
    if [[ "$message" =~ (memory|leak|alloc|drop|borrow|lifetime) ]]; then
        echo "memory"
        return
    fi

    # Performance issues
    if [[ "$message" =~ (performance|slow|inefficient|optimize|benchmark) ]]; then
        echo "performance"
        return
    fi

    # Code style/quality
    if [[ "$code" =~ (clippy::|style|format|unused) ]]; then
        echo "style"
        return
    fi

    # Security issues
    if [[ "$message" =~ (security|unsafe|vulnerability|expose|leak) ]]; then
        echo "security"
        return
    fi

    # Default category
    echo "general"
}

# Generate strategy for each problem
generate_strategy() {
    local problem_data="$1"
    local strategy_file="$2"

    log_info "Generating strategy for problems"

    if ! command -v jq >/dev/null 2>&1; then
        log_error "jq is required for strategy generation"
        return 1
    fi

    # Initialize strategy document
    cat > "$strategy_file" << EOF
{
  "strategies": [],
  "summary": {
    "total_problems": 0,
    "by_category": {},
    "by_priority": {},
    "estimated_effort": "unknown"
  },
  "generated_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "agent": "bacon-strategist"
}
EOF

    local temp_strategies="${strategy_file}.tmp"
    local total_problems=0
    declare -A categories
    declare -A priorities

    # Process each problem
    while IFS= read -r problem; do
        local message level code file line_start
        message=$(echo "$problem" | jq -r '.message // "unknown"')
        level=$(echo "$problem" | jq -r '.level // "unknown"')
        code=$(echo "$problem" | jq -r '.code // "unknown"')
        file=$(echo "$problem" | jq -r '.spans[0].file_name // "unknown"')
        line_start=$(echo "$problem" | jq -r '.spans[0].line_start // 0')

        # Analyze problem
        local priority category
        priority=$(analyze_problem_priority "$level" "$code")
        category=$(categorize_problem "$code" "$message")

        # Generate strategy based on category
        local strategy_approach recommended_action estimated_effort risk_level

        case "$category" in
            "concurrency")
                strategy_approach="Use atomic primitives or message passing channels"
                recommended_action="Review shared state and consider using Arc<Mutex<>> or mpsc channels"
                estimated_effort="medium"
                risk_level="high"
                ;;
            "memory")
                strategy_approach="Review ownership and lifetime management"
                recommended_action="Check for unnecessary clones, consider using references or Cow"
                estimated_effort="medium"
                risk_level="medium"
                ;;
            "performance")
                strategy_approach="Profile and optimize hot paths"
                recommended_action="Use cargo bench to identify bottlenecks, consider algorithms optimization"
                estimated_effort="high"
                risk_level="medium"
                ;;
            "style")
                strategy_approach="Apply code style improvements"
                recommended_action="Run cargo fmt and address clippy suggestions"
                estimated_effort="low"
                risk_level="low"
                ;;
            "security")
                strategy_approach="Security audit and remediation"
                recommended_action="Review unsafe blocks and validate input handling"
                estimated_effort="high"
                risk_level="high"
                ;;
            *)
                strategy_approach="General code improvement"
                recommended_action="Address the specific compiler warning/error"
                estimated_effort="medium"
                risk_level="medium"
                ;;
        esac

        # Create strategy entry
        local strategy_entry
        strategy_entry=$(jq -n -c \
            --arg message "$message" \
            --arg level "$level" \
            --arg code "$code" \
            --arg file "$file" \
            --arg line_start "$line_start" \
            --arg priority "$priority" \
            --arg category "$category" \
            --arg approach "$strategy_approach" \
            --arg action "$recommended_action" \
            --arg effort "$estimated_effort" \
            --arg risk "$risk_level" \
            '{
                problem: {
                    message: $message,
                    level: $level,
                    code: $code,
                    location: {
                        file: $file,
                        line: ($line_start | tonumber)
                    }
                },
                strategy: {
                    priority: $priority,
                    category: $category,
                    approach: $approach,
                    recommended_action: $action,
                    estimated_effort: $effort,
                    risk_level: $risk
                }
            }')

        # Add to strategies array
        jq --argjson strategy "$strategy_entry" '.strategies += [$strategy]' "$strategy_file" > "${strategy_file}.new" && mv "${strategy_file}.new" "$strategy_file"

        # Update counters
        total_problems=$((total_problems + 1))
        categories["$category"]=$((${categories["$category"]:-0} + 1))
        priorities["$priority"]=$((${priorities["$priority"]:-0} + 1))
    done < <(jq -c '.problems[]' "$problem_data")

    # Update summary
    local categories_json priorities_json

    # Convert categories to JSON
    categories_json="{"
    for cat in "${!categories[@]}"; do
        if [[ "$categories_json" != "{" ]]; then
            categories_json+=","
        fi
        categories_json+="\"$cat\":${categories[$cat]}"
    done
    categories_json+="}"

    # Convert priorities to JSON
    priorities_json="{"
    for pri in "${!priorities[@]}"; do
        if [[ "$priorities_json" != "{" ]]; then
            priorities_json+=","
        fi
        priorities_json+="\"$pri\":${priorities[$pri]}"
    done
    priorities_json+="}"

    # Update summary in final JSON
    jq \
        --argjson total "$total_problems" \
        --argjson categories "$categories_json" \
        --argjson priorities "$priorities_json" \
        '.summary.total_problems = $total |
         .summary.by_category = $categories |
         .summary.by_priority = $priorities |
         .summary.estimated_effort = "medium"' \
        "$strategy_file" > "${strategy_file}.new" && mv "${strategy_file}.new" "$strategy_file"

    log_info "Generated strategies for $total_problems problems"
}

# Generate implementation plan
generate_implementation_plan() {
    local strategy_file="$1"
    local plan_file="$2"

    log_info "Generating implementation plan"

    if ! command -v jq >/dev/null 2>&1; then
        log_error "jq is required for implementation plan generation"
        return 1
    fi

    # Keep the full strategy records while grouping them for the coder.
    jq '
        .strategies |= sort_by(.strategy.priority, .strategy.category) |
        .implementation_plan = {
            high_priority: [
                .strategies[] |
                select(.strategy.priority == "high") |
                . + {status: "pending"}
            ],
            medium_priority: [
                .strategies[] |
                select(.strategy.priority == "medium") |
                . + {status: "pending"}
            ],
            low_priority: [
                .strategies[] |
                select(.strategy.priority == "low") |
                . + {status: "pending"}
            ]
        }
    ' "$strategy_file" > "$plan_file"

    log_info "Implementation plan generated"
}

# Main function
main() {
    local input_file="$1"
    local output_file="$2"

    if [[ -z "$input_file" || -z "$output_file" ]]; then
        log_error "Usage: $0 <input_file> <output_file>"
        return 1
    fi

    log_info "Starting strategy analysis"

    # Check if input file exists
    if [[ ! -f "$input_file" ]]; then
        log_error "Input file not found: $input_file"
        return 1
    fi

    # Validate input against schema
    if [[ -f "${SCRIPT_DIR}/json-validator.sh" ]]; then
        if ! "${SCRIPT_DIR}/json-validator.sh" validate "$input_file" "observer-output"; then
            log_warn "Input validation failed, but continuing"
        fi
    fi

    # Generate strategies
    local temp_strategy="${output_file}.strategy.tmp"
    if generate_strategy "$input_file" "$temp_strategy"; then
        # Generate implementation plan
        generate_implementation_plan "$temp_strategy" "$output_file"

        # Cleanup temp file
        rm -f "$temp_strategy"

        # Validate output against schema
        if [[ -f "${SCRIPT_DIR}/json-validator.sh" ]]; then
            if "${SCRIPT_DIR}/json-validator.sh" validate "$output_file" "strategy-output"; then
                log_debug "Output validation passed"
            else
                log_warn "Output validation failed, but continuing"
            fi
        fi

        log_info "Strategy analysis completed successfully"
        return 0
    else
        log_error "Strategy analysis failed"
        return 1
    fi
}

# Execute main function
main "$@"
