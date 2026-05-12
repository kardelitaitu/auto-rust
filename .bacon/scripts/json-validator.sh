#!/bin/bash
# .bacon/scripts/json-validator.sh - JSON Schema Validation Utility
# Validates JSON files against expected schemas for Bacon agent communications

set -euo pipefail
IFS=$'\n\t'

# Configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && (pwd -W 2>/dev/null || pwd))"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && (pwd -W 2>/dev/null || pwd))"
readonly SCHEMAS_DIR="${SCRIPT_DIR}/schemas"

# Logging utilities
log() {
    local level="$1"
    shift
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [$level] $*" >&2
}

log_error() { log "ERROR" "$@"; }
log_warn() { log "WARN" "$@"; }
log_info() { log "INFO" "$@"; }
log_debug() { log "DEBUG" "$@"; }

# Create schemas directory if it doesn't exist
ensure_schemas_dir() {
    if [[ ! -d "$SCHEMAS_DIR" ]]; then
        mkdir -p "$SCHEMAS_DIR"
        log_info "Created schemas directory: $SCHEMAS_DIR"
    fi
}

# Generate JSON schemas for different agent outputs
generate_schemas() {
    ensure_schemas_dir

    # Observer output schema
    cat > "$SCHEMAS_DIR/observer-output.json" << 'EOF'
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "problems": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "message": {"type": "string"},
          "level": {"type": "string", "enum": ["error", "warning", "note"]},
          "code": {"type": "string"},
          "spans": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "file_name": {"type": "string"},
                "line_start": {"type": "integer"},
                "line_end": {"type": "integer"},
                "column_start": {"type": "integer"},
                "column_end": {"type": "integer"},
                "text": {"type": "string"}
              },
              "required": ["file_name", "line_start", "line_end", "column_start", "column_end"]
            }
          },
          "rendered": {"type": "string"}
        },
        "required": ["message", "level", "code"]
      }
    },
    "summary": {
      "type": "object",
      "properties": {
        "total": {"type": "integer"},
        "by_level": {"type": "object"},
        "by_category": {"type": "object"}
      },
      "required": ["total"]
    }
  },
  "required": ["problems", "summary"]
}
EOF

    # Strategy output schema
    cat > "$SCHEMAS_DIR/strategy-output.json" << 'EOF'
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "strategies": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "problem": {
            "type": "object",
            "properties": {
              "message": {"type": "string"},
              "level": {"type": "string"},
              "code": {"type": "string"},
              "location": {
                "type": "object",
                "properties": {
                  "file": {"type": "string"},
                  "line": {"type": "integer"}
                }
              }
            }
          },
          "strategy": {
            "type": "object",
            "properties": {
              "priority": {"type": "string", "enum": ["high", "medium", "low"]},
              "category": {"type": "string"},
              "approach": {"type": "string"},
              "recommended_action": {"type": "string"},
              "estimated_effort": {"type": "string", "enum": ["low", "medium", "high"]},
              "risk_level": {"type": "string", "enum": ["low", "medium", "high"]}
            }
          }
        }
      }
    },
    "implementation_plan": {
      "type": "object",
      "properties": {
        "high_priority": {"type": "array"},
        "medium_priority": {"type": "array"},
        "low_priority": {"type": "array"}
      }
    },
    "summary": {
      "type": "object",
      "properties": {
        "total_problems": {"type": "integer"},
        "by_category": {"type": "object"},
        "by_priority": {"type": "object"},
        "estimated_effort": {"type": "string"}
      }
    }
  },
  "required": ["strategies", "summary"]
}
EOF

    # Hotspot input schema (from bacon)
    cat > "$SCHEMAS_DIR/hotspot-input.json" << 'EOF'
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "array",
  "items": {
    "type": "object",
    "properties": {
      "message": {"type": "string"},
      "level": {"type": "string"},
      "code": {
        "type": "object",
        "properties": {
          "code": {"type": "string"}
        }
      },
      "spans": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "file_name": {"type": "string"},
            "line_start": {"type": "integer"},
            "line_end": {"type": "integer"},
            "column_start": {"type": "integer"},
            "column_end": {"type": "integer"},
            "text": {"type": "string"}
          }
        }
      },
      "rendered": {"type": "string"}
    },
    "required": ["message", "level"]
  }
}
EOF

    # Metrics schema
    cat > "$SCHEMAS_DIR/metrics.json" << 'EOF'
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "array",
  "items": {
    "type": "object",
    "properties": {
      "timestamp": {"type": "string", "format": "date-time"},
      "event": {"type": "string"},
      "status": {"type": "string", "enum": ["success", "failed", "rejected"]}
    },
    "required": ["timestamp", "event", "status"]
  }
}
EOF

    log_info "JSON schemas generated successfully"
}

# Validate JSON against schema using jq
validate_with_jq() {
    local json_file="$1"
    local schema_file="$2"

    if [[ ! -f "$json_file" ]]; then
        log_error "JSON file not found: $json_file"
        return 1
    fi

    if [[ ! -f "$schema_file" ]]; then
        log_error "Schema file not found: $schema_file"
        return 1
    fi

    # Basic JSON syntax validation
    if ! jq empty "$json_file" 2>/dev/null; then
        log_error "Invalid JSON syntax in: $json_file"
        return 1
    fi

    # Schema validation (basic - jq doesn't have full JSON schema support)
    # This is a simplified validation that checks for required fields
    local schema_name
    schema_name=$(basename "$schema_file" .json)

    case "$schema_name" in
        "observer-output")
            validate_observer_output "$json_file"
            ;;
        "strategy-output")
            validate_strategy_output "$json_file"
            ;;
        "hotspot-input")
            validate_hotspot_input "$json_file"
            ;;
        "metrics")
            validate_metrics "$json_file"
            ;;
        *)
            log_warn "No specific validation for schema: $schema_name"
            return 0
            ;;
    esac
}

# Validate observer output
validate_observer_output() {
    local json_file="$1"

    # Check required top-level fields
    local problems_count
    problems_count=$(jq '.problems | length // 0' "$json_file" 2>/dev/null || echo "0")

    if [[ "$problems_count" -eq 0 ]]; then
        log_warn "No problems found in observer output"
    fi

    # Check each problem has required fields
    local invalid_problems
    invalid_problems=$(jq -r '.problems[] | select(.message == null or .level == null or .code == null) | "Problem missing required fields"' "$json_file" 2>/dev/null || echo "")

    if [[ -n "$invalid_problems" ]]; then
        log_error "Invalid problems found: $invalid_problems"
        return 1
    fi

    # Check summary exists
    local total_problems
    total_problems=$(jq '.summary.total // 0' "$json_file" 2>/dev/null || echo "0")

    if [[ "$total_problems" -ne "$problems_count" ]]; then
        log_warn "Summary total ($total_problems) doesn't match problems count ($problems_count)"
    fi

    log_debug "Observer output validation passed"
    return 0
}

# Validate strategy output
validate_strategy_output() {
    local json_file="$1"

    # Check required top-level fields
    local strategies_count
    strategies_count=$(jq '.strategies | length // 0' "$json_file" 2>/dev/null || echo "0")

    if [[ "$strategies_count" -eq 0 ]]; then
        log_warn "No strategies found in strategy output"
    fi

    # Check each strategy has required fields
    local invalid_strategies
    invalid_strategies=$(jq -r '.strategies[] | select(.problem == null or .strategy == null) | "Strategy missing required fields"' "$json_file" 2>/dev/null || echo "")

    if [[ -n "$invalid_strategies" ]]; then
        log_error "Invalid strategies found: $invalid_strategies"
        return 1
    fi

    # Check summary exists
    local total_problems
    total_problems=$(jq '.summary.total_problems // 0' "$json_file" 2>/dev/null || echo "0")

    if [[ "$total_problems" -ne "$strategies_count" ]]; then
        log_warn "Summary total ($total_problems) doesn't match strategies count ($strategies_count)"
    fi

    log_debug "Strategy output validation passed"
    return 0
}

# Validate hotspot input
validate_hotspot_input() {
    local json_file="$1"

    # Check if it's an array
    local is_array
    is_array=$(jq 'if type == "array" then "true" else "false" end' "$json_file" 2>/dev/null || echo "false")

    if [[ "$is_array" != "true" ]]; then
        log_error "Hotspot input should be an array"
        return 1
    fi

    # Check each item has required fields
    local invalid_items
    invalid_items=$(jq -r '.[] | select(.message == null or .level == null) | "Hotspot item missing required fields"' "$json_file" 2>/dev/null || echo "")

    if [[ -n "$invalid_items" ]]; then
        log_error "Invalid hotspot items found: $invalid_items"
        return 1
    fi

    log_debug "Hotspot input validation passed"
    return 0
}

# Validate metrics
validate_metrics() {
    local json_file="$1"

    # Check if it's an array
    local is_array
    is_array=$(jq 'if type == "array" then "true" else "false" end' "$json_file" 2>/dev/null || echo "false")

    if [[ "$is_array" != "true" ]]; then
        log_error "Metrics should be an array"
        return 1
    fi

    # Check each item has required fields
    local invalid_items
    invalid_items=$(jq -r '.[] | select(.timestamp == null or .event == null or .status == null) | "Metric item missing required fields"' "$json_file" 2>/dev/null || echo "")

    if [[ -n "$invalid_items" ]]; then
        log_error "Invalid metric items found: $invalid_items"
        return 1
    fi

    # Check timestamp format
    local invalid_timestamps
    invalid_timestamps=$(jq -r '.[] | select(.timestamp | test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$") | not) | "Invalid timestamp format"' "$json_file" 2>/dev/null || echo "")

    if [[ -n "$invalid_timestamps" ]]; then
        log_error "Invalid timestamp formats found: $invalid_timestamps"
        return 1
    fi

    log_debug "Metrics validation passed"
    return 0
}

# Main function
main() {
    local action="${1:-validate}"
    local json_file="$2"
    local schema_type="$3"

    case "$action" in
        "generate-schemas")
            generate_schemas
            ;;
        "validate")
            if [[ -z "$json_file" || -z "$schema_type" ]]; then
                log_error "Usage: $0 validate <json_file> <schema_type>"
                log_error "Schema types: observer-output, strategy-output, hotspot-input, metrics"
                exit 1
            fi

            # Generate schemas if they don't exist
            if [[ ! -d "$SCHEMAS_DIR" ]]; then
                generate_schemas
            fi

            local schema_file="$SCHEMAS_DIR/${schema_type}.json"
            if validate_with_jq "$json_file" "$schema_file"; then
                log_info "Validation passed: $json_file against $schema_type schema"
                exit 0
            else
                log_error "Validation failed: $json_file against $schema_type schema"
                exit 1
            fi
            ;;
        "list-schemas")
            ensure_schemas_dir
            log_info "Available schemas:"
            ls -1 "$SCHEMAS_DIR"/*.json 2>/dev/null || log_info "No schemas found"
            ;;
        *)
            log_error "Unknown action: $action"
            log_error "Usage: $0 <generate-schemas|validate|list-schemas> [args]"
            exit 1
            ;;
    esac
}

# Execute main function
main "$@"
